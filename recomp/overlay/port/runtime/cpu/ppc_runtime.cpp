// Default runtime services for recompiled code: indirect calls, SPRs, traps,
// and crash reporting with the guest call stack.
#include "ppc_runtime.h"
#include "native_config.h"
#include "os.h"
#include "rso.h"
#include "embed_host.h"

#include <algorithm>
#include <chrono>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>

namespace {
PPCTrace g_main_trace;
}
bool g_ppc_poll_enabled=false;
uint32_t g_ppc_poll_calls=0,g_ppc_poll_host=0;
PPCTrace* g_ppc_trace = &g_main_trace;
PPCContext* g_ppc_context_for_debug=nullptr;

void ppc_enable_interrupt_polling() {g_ppc_poll_enabled=true;}
void ppc_reset_debug_context(){g_ppc_poll_enabled=false;g_main_trace.depth=0;g_ppc_trace=&g_main_trace;g_ppc_context_for_debug=nullptr;}
void ppc_poll_slow(PPCContext& c,uint8_t* m,bool deliver) {
    native_host_poll();
    mw_rso_attach(m);
    if(deliver)os_deliver_interrupts(c,m);
    g_ppc_context_for_debug=&c;
}

const char* ppc_func_name(uint32_t addr, uint32_t* start) {
    if (const char* name = mw_rso_func_name(addr, start)) return name;
    const PPCFuncName* begin = g_ppc_func_names;
    const PPCFuncName* end = g_ppc_func_names + g_ppc_func_name_count;
    auto it = std::upper_bound(begin, end, addr, [](uint32_t a, const PPCFuncName& e) { return a < e.addr; });
    if (it == begin) return nullptr;
    --it;
    if (addr >= it->end) return nullptr;
    if (start) *start = it->addr;
    return it->name;
}

void ppc_print_guest_stack(FILE* out) {
    const PPCTrace* t = g_ppc_trace;
    uint32_t n = std::min(t->depth, PPCTrace::kMax);
    std::fprintf(out, "guest call stack (innermost last, %u deep):\n", t->depth);
    for (uint32_t i = n > 40 ? n - 40 : 0; i < n; i++) {
        const char* name = ppc_func_name(t->stack[i], nullptr);
        std::fprintf(out, "  %3u  0x%08X  %s\n", i, t->stack[i], name ? name : "?");
    }
}

[[noreturn]] void ppc_fatal(const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    char message[1024];va_list copy;va_copy(copy,ap);std::vsnprintf(message,sizeof message,fmt,copy);va_end(copy);
    std::fprintf(stderr, "[ppc] fatal: ");
    std::vfprintf(stderr, fmt, ap);
    std::fprintf(stderr, "\n");
    va_end(ap);
    ppc_print_guest_stack(stderr);
    std::fflush(stderr);
    std::fflush(stdout);
    native_host_fail(message);
    std::_Exit(1);
}

PPCFunc native_frame_record_observe(uint32_t addr, PPCFunc fn);
PPCFunc ppc_lookup(uint32_t addr) {
    // Indirect calls (every guest virtual call) searched ~26k entries, then the RSO
    // table on a miss. A direct-mapped cache returns the same function; RSO entries
    // depend on the module's load base, so the base is part of the key.
    struct Slot { uint32_t addr, rso_base; PPCFunc fn; };
    static Slot cache[8192];
    const uint32_t rso_base = mw_rso_current_base();
    Slot& slot = cache[(addr >> 2) & 8191];
    if (slot.fn && slot.addr == addr && slot.rso_base == rso_base) return slot.fn;
    const PPCFuncEntry* begin = g_ppc_funcs;
    const PPCFuncEntry* end = g_ppc_funcs + g_ppc_func_count;
    auto it = std::lower_bound(begin, end, addr, [](const PPCFuncEntry& e, uint32_t a) { return e.addr < a; });
    PPCFunc fn = (it != end && it->addr == addr) ? it->fn : mw_rso_lookup(addr);
    if (fn) fn = native_frame_record_observe(addr, fn);  // game/frame_record.cpp, default off
    if (fn) slot = {addr, rso_base, fn};
    return fn;
}

void ppc_call_indirect(PPCContext& c, uint8_t* m, uint32_t addr) {
    PPCFunc fn = ppc_lookup(addr);
    if (!fn) ppc_fatal("indirect call to 0x%08X has no recompiled function (lr=0x%08X rso-base=0x%08X canonical=0x%08X)", addr, c.lr, mw_rso_base(m), mw_rso_canonical_pc(m, addr));
    fn(c, m);
}

uint32_t ppc_mfspr(PPCContext& c, uint32_t spr) { return c.spr[spr & 1023]; }
void ppc_mtspr(PPCContext& c, uint8_t*, uint32_t spr, uint32_t value) { c.spr[spr & 1023] = value; }

// Test-only guest clock acceleration: MW_TEST_TIME_SCALE=N (1..16) makes the
// guest time base, and therefore every timebase-driven device model, advance N
// times faster than the host clock. Default 1 (original rate). Not for play.
unsigned ppc_test_time_scale() {
    static const unsigned scale = [] {
        const char* v = native_config_get("MW_TEST_TIME_SCALE");
        if (!v) return 1u;
        char* end = nullptr; unsigned long n = std::strtoul(v, &end, 10);
        if (!end || *end || n < 1 || n > 16) ppc_fatal("MW_TEST_TIME_SCALE must be an integer 1..16");
        return (unsigned)n;
    }();
    return scale;
}

namespace { auto tb_origin=std::chrono::steady_clock::now();uint64_t tb_offset=0; }
bool ppc_bench_unthrottled() {
    static const bool on = [] { const char* v = native_config_get("MW_BENCH_UNTHROTTLED"); return v && v[0] == '1' && !v[1]; }();
    return on;
}
void ppc_skip_timebase(uint64_t ticks) { tb_offset += ticks; }
void ppc_set_timebase(uint64_t tb){tb_origin=std::chrono::steady_clock::now();tb_offset=tb;}
uint64_t ppc_timebase() {
    // Wii time base runs at bus clock / 4 = 60.75 MHz.
    using namespace std::chrono;

    uint64_t ns = (uint64_t)duration_cast<nanoseconds>(steady_clock::now() - tb_origin).count();
    return tb_offset + ns * ppc_test_time_scale() * 243 / 4000;
}

uint32_t ppc_mftb(PPCContext&, uint32_t tbr) {
    uint64_t tb = ppc_timebase();
    return tbr == 269 ? (uint32_t)(tb >> 32) : (uint32_t)tb;
}

void ppc_mtmsr(PPCContext& c, uint8_t*, uint32_t value) { c.msr = value; }
// The Wii OS's only system call (installed at 0x80000C00 by __OSInitSystemCall)
// syncs/flushes the CPU's write pipeline after cache maintenance (DCFlushRange etc.).
// Host memory is coherent, so there's nothing to do.
void ppc_syscall(PPCContext&, uint8_t*) {}
// rfi jumps to SRR0 with MSR = SRR1. Outside exception handlers the SDK only uses it
// to switch MMU translation off/on around a call (e.g. BATConfig's real-mode helper):
// jumping *to* a function start, or jumping *back* to the caller (SRR0 == LR).
// There's no MMU here, so both become plain control flow; generated code returns
// right after ppc_rfi.
void ppc_rfi(PPCContext& c, uint8_t* m) {
    uint32_t target = c.spr[26];
    if (target < 0x80000000u) target |= 0x80000000u;  // physical (real mode) -> cached alias
    c.msr = c.spr[27];
    if (PPCFunc fn = ppc_lookup(target)) {
        fn(c, m);
        return;
    }
    if ((target & 0x3FFFFFFFu) == (c.lr & 0x3FFFFFFFu)) return;  // back to the caller
    ppc_fatal("rfi to 0x%08X: neither a function start nor the return address (lr=0x%08X)", target, c.lr);
}
void ppc_trap(PPCContext&, uint32_t addr) { ppc_fatal("trap at 0x%08X", addr); }
void ppc_illegal(PPCContext&, uint32_t addr) { ppc_fatal("illegal instruction at 0x%08X", addr); }
void ppc_fell_off(PPCContext&, uint32_t addr) { ppc_fatal("execution ran past the end of code at 0x%08X", addr); }
void ppc_unimplemented(PPCContext&, uint32_t addr, const char* what) {
    ppc_fatal("unimplemented instruction '%s' at 0x%08X", what, addr);
}

#include "snapshot.h"
void native_cpu_state(StateArchive& ar){ar.pod(g_ppc_poll_enabled);ar.pod(g_ppc_poll_calls);}
