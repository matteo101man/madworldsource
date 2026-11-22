#include "native_config.h"
// Interrupt delivery: PI / Hollywood interrupt registers, the decrementer, and the
// OS's interrupt enable/disable functions.
//
// Recompiled code can't be interrupted asynchronously, so interrupts are delivered
// at safe points: when the game re-enables them (OSEnableInterrupts /
// OSRestoreInterrupts) and while the scheduler idles. Delivery calls the SDK's own
// handlers (the C side of the exception vectors) with the current context, so all
// the OS bookkeeping, thread wakeups and rescheduling run as recompiled code.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <chrono>
#include <queue>
#include <thread>
#include <vector>
#include <array>
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include "hle.h"
#include "mmio.h"
#include "os.h"

namespace {
constexpr uint32_t CUR_CONTEXT = 0x800000D4u;
constexpr uint32_t FN_DISPATCH_INTERRUPT = 0x802FECA8u;   // __OSDispatchInterrupt(exception, context)
constexpr uint32_t FN_DECREMENTER_CALLBACK = 0x802F96D4u; // DecrementerExceptionCallback(exception, context)
constexpr int EXC_EXTERNAL = 4, EXC_DECREMENTER = 8;

uint32_t g_pi_cause = 0;      // INTSR raw causes
uint32_t g_pi_mask = 0;       // INTMR
uint32_t g_hw_flags = 0;      // Hollywood PPC IRQ flags (latched)
uint32_t g_hw_level = 0;      // sources currently asserting (re-latch after a clear)
uint32_t g_hw_mask = 0;
bool g_dec_armed = false;
uint64_t g_dec_deadline = 0;

struct Event {
    uint64_t when;
    uint64_t seq;
    OSEventData data;
    bool operator>(const Event& o) const { return when != o.when ? when > o.when : seq > o.seq; }
};
std::priority_queue<Event, std::vector<Event>, std::greater<Event>> g_events;
uint64_t g_event_seq = 0;
std::array<OSEventHandler, size_t(OSEventKind::Count)> g_event_handlers{};
bool g_trace = native_config_get("MW_TRACE_IRQ") != nullptr;

void update_hollywood_line() {
    if (g_hw_flags & g_hw_mask) g_pi_cause |= PI_INT_HOLLYWOOD;
    else g_pi_cause &= ~PI_INT_HOLLYWOOD;
}

void run_due_events() {
    uint64_t now = ppc_timebase();
    while (!g_events.empty() && g_events.top().when <= now) {
        OSEventData data = g_events.top().data;
        g_events.pop();
        auto handler = g_event_handlers.at(size_t(data.kind));
        if (!handler) ppc_fatal("Unregistered hardware event %u", unsigned(data.kind));
        handler(data);
    }
}

void call_handler(PPCContext& c, uint8_t* m, uint32_t fn, int exception) {
    // An interrupt is invisible to the code it interrupts: on hardware the exception
    // handler saves every register and OSLoadContext restores them. Here the handler
    // runs on the interrupted thread's register set and is cut short at OSLoadContext
    // (skipping its epilogue), so snapshot the whole CPU state and put it back.
    // Host fibers share thread_local storage and can resume out of order.
    // Keep this snapshot on the calling fiber's stack, not a shared LIFO.
    PPCContext snapshot=c;
    uint32_t msr = c.msr;
    uint32_t ctx = ld32(m, CUR_CONTEXT);
    c.msr &= ~MSR_EE;
    c.r[3] = (uint32_t)exception;
    c.r[4] = ctx;
    os_irq_push(ctx);
    try {
        ppc_call_indirect(c, m, fn);
    } catch (const OSInterruptReturn&) {
        // handler finished with OSLoadContext(ctx): back to the interrupted code
    }
    os_irq_pop();
    c = snapshot;
    c.msr = msr;
}
}  // namespace

void pi_set_cause(uint32_t bits, bool on) {
    if (on) g_pi_cause |= bits;
    else g_pi_cause &= ~bits;
}

void hollywood_set_irq(uint32_t bits, bool on) {
    // Sources are levels: while one asserts, its flag comes back even after the
    // game acknowledges it (as the IPC block does on hardware).
    if (on) {
        g_hw_level |= bits;
        g_hw_flags |= bits;
    } else {
        g_hw_level &= ~bits;
        g_hw_flags &= ~bits;
    }
    update_hollywood_line();
}

void os_register_event(OSEventKind kind, OSEventHandler handler) {
    auto& slot = g_event_handlers.at(size_t(kind));
    if (slot && slot != handler) ppc_fatal("Conflicting hardware event handler");
    slot = handler;
}
void os_schedule(uint64_t when_tb, OSEventData data) {
    if (size_t(data.kind) >= g_event_handlers.size() || !g_event_handlers[size_t(data.kind)])
        ppc_fatal("Unregistered scheduled hardware event");
    g_events.push({when_tb, g_event_seq++, data});
}
void os_poll_hardware_events(){run_due_events();}

void os_deliver_interrupts(PPCContext& c, uint8_t* m) {
    for (int guard = 0; guard < 64 && (c.msr & MSR_EE); guard++) {
        run_due_events();
        if (g_dec_armed && ppc_timebase() >= g_dec_deadline) {
            g_dec_armed = false;
            if (g_trace) std::fprintf(stderr, "[irq] decrementer\n");
            call_handler(c, m, FN_DECREMENTER_CALLBACK, EXC_DECREMENTER);
        } else if (g_pi_cause & g_pi_mask) {
            if (g_trace)
                std::fprintf(stderr, "[irq] external cause=0x%X mask=0x%X hw=0x%X/0x%X os=0x%08X|0x%08X\n", g_pi_cause,
                             g_pi_mask, g_hw_flags, g_hw_mask, ld32(m, 0x800000C4u), ld32(m, 0x800000C8u));
            if (g_trace) {
                uint32_t table = ld32(m, 0x8071F720u - 30992);
                std::fprintf(stderr, "[irq]   handler table 0x%08X, [27] = 0x%08X, prio[0..3] = %08X %08X %08X %08X\n",
                             table, table ? ld32(m, table + 27 * 4) : 0, ld32(m, 0x805D3E80u), ld32(m, 0x805D3E84u),
                             ld32(m, 0x805D3E88u), ld32(m, 0x805D3E8Cu));
            }
            call_handler(c, m, FN_DISPATCH_INTERRUPT, EXC_EXTERNAL);
        } else {
            return;
        }
    }
}

void os_trace_hardware_state() {
    std::fprintf(stderr,"[idle-irq] cause=%08x mask=%08x hollywood=%08x/%08x dec-armed=%u dec-deadline=%llu now=%llu events=%zu\n",
        g_pi_cause,g_pi_mask,g_hw_flags,g_hw_mask,g_dec_armed,g_dec_deadline,ppc_timebase(),g_events.size());
}

// MW_BENCH_UNTHROTTLED=1 (diagnostic only, cpu/ppc_runtime.cpp): idle waits advance the
// guest time base to the next deadline instead of sleeping.
bool ppc_bench_unthrottled();
void ppc_skip_timebase(uint64_t ticks);

namespace {
// std::this_thread::sleep_for rounds up to whole milliseconds on MSVC (Sleep), and
// Sleep's granularity follows the global timer resolution (15.6 ms unless some
// process raised it). A high-resolution waitable timer honours sub-ms deadlines.
void host_wait_ns(uint64_t ns) {
    static HANDLE timer = CreateWaitableTimerExW(nullptr, nullptr, 0x00000002 /*CREATE_WAITABLE_TIMER_HIGH_RESOLUTION*/, TIMER_ALL_ACCESS);
    if (!timer) {
        std::this_thread::sleep_for(std::chrono::nanoseconds(ns));
        return;
    }
    LARGE_INTEGER due;
    due.QuadPart = -static_cast<LONGLONG>(std::max<uint64_t>(ns / 100, 1));  // relative, 100 ns units
    if (!SetWaitableTimer(timer, &due, 0, nullptr, nullptr, FALSE)) {
        std::this_thread::sleep_for(std::chrono::nanoseconds(ns));
        return;
    }
    WaitForSingleObject(timer, INFINITE);
}
}  // namespace

void os_wait_for_event() {
    run_due_events();
    if (g_pi_cause & g_pi_mask) return;
    uint64_t now = ppc_timebase();
    uint64_t next = now + 60750;  // at most 1 ms
    if (g_dec_armed) next = std::min(next, g_dec_deadline);
    if (!g_events.empty()) next = std::min(next, g_events.top().when);
    if (next > now && ppc_bench_unthrottled()) {
        // Benchmark: nothing can run until the next deadline, so skip to it.
        ppc_skip_timebase(next - now);
        return;
    }
    if (next > now) {
        // Same test-only MW_TEST_TIME_SCALE as ppc_timebase (validated there); read
        // locally so the interrupt unit also links in the standalone regression exe.
        static const uint64_t time_scale=[](){const char* v=native_config_get("MW_TEST_TIME_SCALE");unsigned long n=v?std::strtoul(v,nullptr,10):1;return uint64_t(n>=1&&n<=16?n:1);}();
        const uint64_t requested=(next-now)*4000/243/time_scale;
        static const bool profile=[](){const char* v=native_config_get("MW_PROFILE_NATIVE_IO");return v&&v[0]=='1'&&v[1]==0;}();
        using Clock=std::chrono::steady_clock;
        static uint64_t calls=0,requested_ns=0,actual_ns=0,max_overshoot=0,over2ms=0,over10ms=0;
        static auto report=Clock::now();
        const auto began=profile?Clock::now():Clock::time_point{};
        host_wait_ns(requested);
        if(profile){
            uint64_t actual=uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-began).count());
            uint64_t overshoot=actual>requested?actual-requested:0;
            ++calls;requested_ns+=requested;actual_ns+=actual;max_overshoot=std::max(max_overshoot,overshoot);over2ms+=overshoot>2000000;over10ms+=overshoot>10000000;
            if(Clock::now()-report>=std::chrono::seconds(1)){
                report=Clock::now();
                std::fprintf(stderr,"NATIVE_WAIT_PROFILE {\"calls\":%llu,\"requestedNs\":%llu,\"actualNs\":%llu,\"maxOvershootNs\":%llu,\"over2ms\":%llu,\"over10ms\":%llu}\n",calls,requested_ns,actual_ns,max_overshoot,over2ms,over10ms);
            }
        }
    }
}

// ---- OS functions -------------------------------------------------------

// BOOL OSDisableInterrupts(void)
PPC_FUNC(f_802FE818) {
    c.r[3] = (c.msr & MSR_EE) ? 1 : 0;
    c.msr &= ~MSR_EE;
}

// BOOL OSEnableInterrupts(void)
PPC_FUNC(f_802FE82C) {
    uint32_t old = (c.msr & MSR_EE) ? 1 : 0;
    c.msr |= MSR_EE;
    os_deliver_interrupts(c, m);
    c.r[3] = old;
}

// BOOL OSRestoreInterrupts(BOOL level)
PPC_FUNC(f_802FE840) {
    uint32_t old = (c.msr & MSR_EE) ? 1 : 0;
    if (c.r[3]) {
        c.msr |= MSR_EE;
        os_deliver_interrupts(c, m);
    } else {
        c.msr &= ~MSR_EE;
    }
    c.r[3] = old;
}

// void PPCMtdec(u32 ticks): arms the decrementer (alarms).
PPC_FUNC(f_802F7ACC) {
    int32_t ticks = (int32_t)c.r[3];
    if (g_trace) std::fprintf(stderr, "[irq] PPCMtdec(%d)\n", ticks);
    g_dec_armed = true;
    g_dec_deadline = ppc_timebase() + (ticks > 0 ? (uint64_t)ticks : 0);
}

// ---- PI interrupt registers (0xCC003000) ---------------------------------
MMIO_DEVICE(pi_interrupts) {
    mmio_register(
        0xCC003000u, 8,
        [](uint32_t a, int) -> uint32_t { return (a & 7) == 0 ? g_pi_cause : g_pi_mask; },
        [](uint32_t a, uint32_t v, int) {
            if ((a & 7) == 4) g_pi_mask = v;
            // INTSR is read-only; causes are acknowledged at their devices.
        });
}

// ---- Hollywood PPC IRQ flags / mask (0xCD000030 / 0xCD000034) --------------
MMIO_DEVICE(hollywood_irq) {
    mmio_register(
        0xCD000030u, 8,
        [](uint32_t a, int) -> uint32_t { return (a & 7) == 0 ? g_hw_flags : g_hw_mask; },
        [](uint32_t a, uint32_t v, int) {
            if ((a & 7) == 0) g_hw_flags = (g_hw_flags & ~v) | g_hw_level;  // write 1 to acknowledge
            else g_hw_mask = v;
            update_hollywood_line();
        });
}

// Guest fibers are parked; release queued callbacks without executing them.
void os_clear_hardware_events(){while(!g_events.empty())g_events.pop();}

#include "snapshot.h"
void os_events_state(StateArchive& ar){
    ar.tag(0x45564e54);ar.pod(g_pi_cause);ar.pod(g_pi_mask);ar.pod(g_hw_flags);ar.pod(g_hw_level);ar.pod(g_hw_mask);ar.pod(g_dec_armed);ar.pod(g_dec_deadline);ar.pod(g_event_seq);
    std::vector<Event> events;
    if(!ar.reading){auto copy=g_events;while(!copy.empty()){events.push_back(copy.top());copy.pop();}}
    ar.vector(events,65536);
    if(ar.reading){g_events={};for(const auto& e:events){if(size_t(e.data.kind)>=g_event_handlers.size()||!g_event_handlers[size_t(e.data.kind)]||e.seq>=g_event_seq)throw std::runtime_error("Invalid snapshot hardware event");g_events.push(e);}}
}
