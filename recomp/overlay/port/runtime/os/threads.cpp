#include "native_config.h"
// Guest threads on host fibers.
//
// The Wii OS switches threads by saving/loading register contexts (OSSaveContext /
// OSLoadContext with rfi), which can't work for recompiled code: a thread's real
// state lives on the host stack. Instead each guest OSThread runs on its own host
// fiber, and the scheduler core (SelectThread) is replaced with host code that
// keeps the SDK's own run queues and switches fibers.
//
// Everything else in the OS thread library (OSCreateThread, OSSleepThread,
// OSWakeupThread, mutexes, message queues...) stays recompiled and works through
// this core. When no thread is ready, the idle loop delivers interrupts (alarms,
// IOS replies, video retrace) until one becomes ready, like the hardware does.
#include <cstdio>
#include <cstdlib>
#include <unordered_map>
#include <vector>
#include <array>
#include "snapshot.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include "hle.h"
#include "embed_host.h"
#include "host_fiber.h"
#include <exception>
#include "os.h"

namespace {
// Globals of the OS thread library (r13 = 0x8071F720)
constexpr uint32_t CUR_THREAD = 0x800000E4u;     // __OSCurrentThread
constexpr uint32_t CUR_CONTEXT = 0x800000D4u;    // OSCurrentContext
constexpr uint32_t RUN_QUEUE = 0x806A7810u;      // OSThreadQueue[32]: {head, tail}
constexpr uint32_t IDLE_CONTEXT = 0x806A7910u;
constexpr uint32_t RUN_QUEUE_BITS = 0x8071F720u - 30944;
constexpr uint32_t RESCHEDULE = 0x8071F720u - 30948;
constexpr uint32_t RESCHEDULE_DISABLED = 0x8071F720u - 30952;
constexpr uint32_t SWITCH_CALLBACK = 0x8071F720u - 32512;

// OSThread / OSContext fields
constexpr uint32_t T_STATE = 0x2C8, T_PRIORITY = 0x2D0, T_QUEUE = 0x2DC, T_NEXT = 0x2E0, T_PREV = 0x2E4;
enum : uint32_t { STATE_INACTIVE = 0, STATE_READY = 1, STATE_RUNNING = 2, STATE_WAITING = 4, STATE_MORIBUND = 8 };


struct GuestThread {
    bool used=false;
    uint32_t addr = 0;
    PPCContext context_storage{};
    PPCTrace trace_storage{};
    HostFiber storage;
    HostFiber* fiber = nullptr;
    PPCContext* ctx = nullptr;
    PPCTrace* trace = nullptr;
    uint32_t entry = 0, exit_lr = 0;
    bool exited = false;
    std::vector<uint32_t> irq_frames;  // contexts of interrupts being delivered on this fiber
};

std::array<GuestThread,256> g_records;
std::unordered_map<uint32_t, GuestThread*> g_threads;
GuestThread* g_boot = nullptr;
GuestThread* g_cur = nullptr;       // whose fiber is executing right now
std::vector<GuestThread*> g_dead;   // exited fibers, deleted from another fiber
uint8_t* g_m = nullptr;
bool g_trace = native_config_get("MW_TRACE_SCHED") != nullptr;

GuestThread* thread_for(uint32_t addr) {
    GuestThread*& t = g_threads[addr];
    if (!t) {
        for(size_t i=1;i<g_records.size();++i)if(!g_records[i].used){t=&g_records[i];t->used=true;t->addr=addr;break;}
        if(!t)ppc_fatal("Guest thread metadata slots exhausted");
    }
    return t;
}

void WINAPI fiber_main(void* p) {
    GuestThread* t = static_cast<GuestThread*>(p);
    PPCContext& c = *t->ctx;
    try{
    ppc_call_indirect(c, g_m, t->entry);
    // The thread function returned: continue at its context's LR (OSExitThread), result in r3.
    ppc_call_indirect(c, g_m, t->exit_lr);
    ppc_fatal("guest thread 0x%08X came back from its exit function", t->addr);
    }catch(const std::exception& error){native_host_fail(error.what());throw;}
    catch(...){native_host_fail("Unhandled guest fiber exception");throw;}
}

// Builds the first host register state of a thread from its guest OSContext.
void start_fiber(GuestThread* t) {
    uint8_t* m = g_m;
    uint32_t ctx = t->addr;
    if (!t->ctx) t->ctx = &t->context_storage;
    if (!t->trace) t->trace = &t->trace_storage;
    PPCContext& c = *t->ctx;
    std::memset(&c, 0, sizeof c);
    std::memset(t->trace, 0, sizeof *t->trace);
    for (int i = 0; i < 32; i++) c.r[i] = ld32(m, ctx + 4 * i);
    c.cr = ld32(m, ctx + 0x80);
    c.lr = ld32(m, ctx + 0x84);
    c.ctr = ld32(m, ctx + 0x88);
    ppc_set_xer(c, ld32(m, ctx + 0x8C));
    for (int i = 0; i < 32; i++) {
        c.f[i].u0 = ld64(m, ctx + 0x90 + 8 * i);
        uint64_t ps1 = ld64(m, ctx + 0x1C8 + 8 * i);
        std::memcpy(&c.f[i].ps1, &ps1, 8);
    }
    c.fpscr = ld32(m, ctx + 0x194);
    for (int i = 0; i < 8; i++) c.gqr[i] = ld32(m, ctx + 0x1A4 + 4 * i);
    c.msr = ld32(m, ctx + 0x19C);
    t->entry = ld32(m, ctx + 0x198);
    t->exit_lr = c.lr;
    t->exited = false;
    std::erase(g_dead,t); // A reused SDK address belongs to this new generation.
    if (t->fiber) host_fiber_destroy(*t->fiber);
    t->fiber = &t->storage;
    if (!host_fiber_create(*t->fiber, fiber_main, t)) ppc_fatal("Fixed stack allocation failed for guest thread 0x%08X", t->addr);
}

void switch_to(GuestThread* t) {
    if (t == g_cur) return;
    if (!t->fiber || t->exited) start_fiber(t);
    g_cur = t;
    g_ppc_trace = t->trace;
    host_fiber_switch(*t->fiber);
    // Back on this fiber: whoever switched here already set g_cur / g_ppc_trace.
    for (GuestThread* d : g_dead)
        if (d != g_cur && d->exited && d->fiber) {
            host_fiber_destroy(*d->fiber);
            d->fiber = nullptr;
        }
    g_dead.clear();
}

void call_guest(PPCContext& c, uint8_t* m, uint32_t fn, uint32_t a0, uint32_t a1 = 0) {
    c.r[3] = a0;
    c.r[4] = a1;
    ppc_call_indirect(c, m, fn);
}

void switch_callback(PPCContext& c, uint8_t* m, uint32_t from, uint32_t to) {
    if (uint32_t cb = ld32(m, SWITCH_CALLBACK)) call_guest(c, m, cb, from, to);
}

void enqueue_ready(uint8_t* m, uint32_t th) {
    uint32_t prio = ld32(m, th + T_PRIORITY);
    uint32_t q = RUN_QUEUE + 8 * prio;
    st32(m, th + T_QUEUE, q);
    uint32_t tail = ld32(m, q + 4);
    if (tail) st32(m, tail + T_NEXT, th);
    else st32(m, q, th);
    st32(m, th + T_PREV, tail);
    st32(m, th + T_NEXT, 0);
    st32(m, q + 4, th);
    st32(m, RUN_QUEUE_BITS, ld32(m, RUN_QUEUE_BITS) | (1u << (31 - prio)));
    st32(m, RESCHEDULE, 1);
}

uint32_t dequeue_highest(uint8_t* m) {
    uint32_t bits = ld32(m, RUN_QUEUE_BITS);
    uint32_t prio = ppc_cntlzw(bits);
    uint32_t q = RUN_QUEUE + 8 * prio;
    uint32_t th = ld32(m, q);
    uint32_t next = ld32(m, th + T_NEXT);
    if (next) st32(m, next + T_PREV, 0);
    else st32(m, q + 4, 0);
    st32(m, q, next);
    if (!next) st32(m, RUN_QUEUE_BITS, bits & ~(1u << (31 - prio)));
    st32(m, th + T_QUEUE, 0);
    st16(m, th + T_STATE, STATE_RUNNING);
    return th;
}
}  // namespace

void os_threads_init(uint8_t* m, PPCContext* main_ctx) {
    g_m = m;
    // Boot already runs on an owned stack, including the standalone CLI.
    HostFiber* fiber = host_fiber_current();
    if (!fiber->allocation) ppc_fatal("Guest boot requires an owned host stack");
    g_boot=&g_records[0];g_boot->used=true;
    g_boot->fiber=fiber;g_boot->ctx=main_ctx;g_boot->trace=g_ppc_trace;g_cur=g_boot;
}

void os_threads_shutdown(HostFiber* retained_fiber){
    for(auto& pair:g_threads){
        GuestThread* t=pair.second;
        // The boot stack belongs to the host entry wrapper, even after it returns.
        if(t!=g_boot&&t->fiber&&t->fiber!=retained_fiber)host_fiber_destroy(*t->fiber);
        if(t!=g_boot)*t=GuestThread{};
    }
    g_threads.clear();g_dead.clear();
    if(g_boot)*g_boot=GuestThread{};
    g_cur=nullptr;g_m=nullptr;
}

void os_irq_push(uint32_t ctx) { g_cur->irq_frames.push_back(ctx); }
void os_irq_pop() { g_cur->irq_frames.pop_back(); }
bool os_irq_is_return(uint32_t ctx) { return !g_cur->irq_frames.empty() && g_cur->irq_frames.back() == ctx; }

// SelectThread(BOOL yield): the scheduler core. Returns 0 (to the thread that called
// it, once that thread runs again).
PPC_FUNC(f_80301900) {
    uint32_t yield = c.r[3];
    c.r[3] = 0;
    if (g_trace)
        std::fprintf(stderr, "[sched] SelectThread(%u) cur=0x%08X state=%u ctx=0x%08X disabled=%u\n", yield,
                     ld32(m, CUR_THREAD), ld32(m, CUR_THREAD) ? ld16(m, ld32(m, CUR_THREAD) + T_STATE) : 0,
                     ld32(m, CUR_CONTEXT), ld32(m, RESCHEDULE_DISABLED));
    if (ld32(m, RESCHEDULE_DISABLED)) return;
    uint32_t cur = ld32(m, CUR_THREAD);
    if (ld32(m, CUR_CONTEXT) != cur) return;  // in an interrupt handler / idle: not now

    // Bind the boot fiber to the thread the OS considers current.
    if (g_cur && g_cur->addr == 0 && cur) {
        GuestThread* boot = g_cur;
        boot->addr = cur;
        g_threads[cur] = boot;
    }

    if (cur && ld16(m, cur + T_STATE) == STATE_RUNNING) {
        if (!yield) {
            int32_t best = (int32_t)ppc_cntlzw(ld32(m, RUN_QUEUE_BITS));
            if ((int32_t)ld32(m, cur + T_PRIORITY) <= best) return;  // still the best choice
        }
        st16(m, cur + T_STATE, STATE_READY);
        enqueue_ready(m, cur);
    }
    // The original SDK sets detached exits to0 and joinable exits to8.
    // Both host stacks are terminal; a reused guest address needs a fresh fiber.
    if (cur && (ld16(m, cur + T_STATE) == STATE_MORIBUND || ld16(m,cur+T_STATE)==STATE_INACTIVE) && g_cur && g_cur->addr == cur && !g_cur->exited) {
        g_cur->exited = true;
        g_dead.push_back(g_cur);
    }

    while (ld32(m, RUN_QUEUE_BITS) == 0) {
        // Idle: no thread can run. Deliver interrupts until one becomes ready.
        switch_callback(c, m, cur, 0);
        st32(m, CUR_THREAD, 0);
        call_guest(c, m, 0x802FAB38u, IDLE_CONTEXT);  // OSSetCurrentContext(&IdleContext)
        uint32_t msr = c.msr;
        c.msr |= MSR_EE;
        while (ld32(m, RUN_QUEUE_BITS) == 0) {
            native_host_poll();
            if(NATIVE_OPTION("MW_TRACE_IDLE")) {
                static uint64_t last=0;
                uint64_t now=ppc_timebase();
                if(!last||now-last>=5ull*60750000) {
                    last=now;
                    std::fprintf(stderr,"[idle] owner=%08x msr=%08x ready=%08x disabled=%08x resched=%08x threads=%zu\n",g_cur?g_cur->addr:0,c.msr,ld32(m,RUN_QUEUE_BITS),ld32(m,RESCHEDULE_DISABLED),ld32(m,RESCHEDULE),g_threads.size());
                    unsigned count=0;
                    for(const auto& item:g_threads) {
                        if(++count>32)break;
                        const GuestThread* thread=item.second;
                        const PPCTrace* trace=thread->trace;
                        std::fprintf(stderr,"[idle-thread] address=%08x state=%u priority=%u queue=%08x exited=%u depth=%u tail=",thread->addr,ld16(m,thread->addr+T_STATE),ld32(m,thread->addr+T_PRIORITY),ld32(m,thread->addr+T_QUEUE),thread->exited,trace?trace->depth:0);
                        if(trace){unsigned n=trace->depth<PPCTrace::kMax?trace->depth:PPCTrace::kMax;for(unsigned i=n>4?n-4:0;i<n;++i)std::fprintf(stderr,"%08x,",trace->stack[i]);}
                        std::fprintf(stderr,"\n");
                    }
                    os_trace_hardware_state();
                }
            }
            os_deliver_interrupts(c, m);
            if (ld32(m, RUN_QUEUE_BITS) == 0) os_wait_for_event();
        }
        c.msr = msr & ~MSR_EE;
        call_guest(c, m, 0x802FAD60u, IDLE_CONTEXT);  // OSClearContext(&IdleContext)
        cur = 0;
    }

    if (g_trace) {
        uint32_t bits = ld32(m, RUN_QUEUE_BITS), prio = ppc_cntlzw(bits);
        std::fprintf(stderr, "[sched] select: cur=0x%08X bits=0x%08X prio=%u head=0x%08X\n", cur, bits, prio,
                     ld32(m, RUN_QUEUE + 8 * prio));
    }
    uint32_t next = dequeue_highest(m);
    st32(m, RESCHEDULE, 0);
    switch_callback(c, m, cur, next);
    st32(m, CUR_THREAD, next);
    call_guest(c, m, 0x802FAB38u, next);  // OSSetCurrentContext(next)
    c.r[3] = 0;
    switch_to(thread_for(next));
    c.r[3] = 0;
}

// OSSaveContext(OSContext*): with fibers there is nothing to save; "first return" is 0.
PPC_FUNC(f_802FABA0) { c.r[3] = 0; }

// OSLoadContext(OSContext*): resume a context; never returns on hardware.
//  - Resuming the context an interrupt interrupted (handlers end like this): unwind
//    back to the delivery point.
//  - Resuming the OS's current thread when it lives on another fiber: switch to it.
//    If this fiber is resumed later, it continues here (the code after the call is
//    the "returned to" path).
PPC_FUNC(f_802FAC20) {
    uint32_t ctx = c.r[3];
    if (os_irq_is_return(ctx)) throw OSInterruptReturn{};
    if (g_cur && ctx == g_cur->addr) return;
    if (ctx && ctx == ld32(m, CUR_THREAD)) switch_to(thread_for(ctx));
}

void os_threads_state(StateArchive& ar){
    if(!g_boot)throw std::runtime_error("Snapshot boot thread absent");
    ar.tag(0x54485244);
    uint32_t current=ar.reading?0:uint32_t(g_cur-g_records.data());ar.pod(current);
    if(current>=g_records.size())throw std::runtime_error("Snapshot current thread slot");
    HostFiber* boot_fiber=g_boot->fiber;PPCContext* boot_ctx=g_boot->ctx;PPCTrace* boot_trace=g_boot->trace;
    if(ar.reading){
        for(size_t i=1;i<g_records.size();++i){auto& t=g_records[i];if(t.fiber)host_fiber_destroy(*t.fiber);t=GuestThread{};}
        g_threads.clear();g_dead.clear();
    }
    std::vector<uint32_t> dead;
    if(!ar.reading)for(auto* t:g_dead)dead.push_back(uint32_t(t-g_records.data()));
    ar.vector(dead,256);
    for(size_t i=0;i<g_records.size();++i){
        auto& t=g_records[i];ar.pod(t.used);if(!t.used)continue;
        if(ar.reading){t.ctx=i?&t.context_storage:boot_ctx;t.trace=i?&t.trace_storage:boot_trace;}
        ar.pod(t.addr);ar.pod(*t.ctx);ar.pod(*t.trace);ar.pod(t.entry);ar.pod(t.exit_lr);ar.pod(t.exited);ar.vector(t.irq_frames,64);
        bool has_stack=t.fiber!=nullptr;ar.pod(has_stack);
        if(ar.reading)t.fiber=has_stack?(i?&t.storage:boot_fiber):nullptr;
        if(has_stack)host_fiber_state(ar,*t.fiber);
        if(ar.reading&&t.addr){if(g_threads.contains(t.addr))throw std::runtime_error("Duplicate snapshot thread address");g_threads[t.addr]=&t;}
    }
    if(ar.reading)for(uint32_t i:dead){if(i>=g_records.size()||!g_records[i].used||!g_records[i].exited)throw std::runtime_error("Invalid retired snapshot thread");g_dead.push_back(&g_records[i]);}
    if(ar.reading){g_boot=&g_records[0];g_cur=&g_records[current];if(!g_cur->used||!g_cur->fiber)throw std::runtime_error("Snapshot thread has no stack");g_ppc_trace=g_cur->trace;}
}
HostFiber* os_threads_resume(){return g_cur?g_cur->fiber:nullptr;}
