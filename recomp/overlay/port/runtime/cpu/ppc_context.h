// PowerPC (Gekko/Broadway) CPU state and helpers used by recompiled code.
//
// Guest memory: `m` points at the base of a 4 GB host reservation, so guest
// address A lives at m + A. MEM1 (0x80000000) / MEM2 (0x90000000) and their
// uncached mirrors (0xC0000000 / 0xD0000000) are mapped onto the same host
// pages by the runtime. Hardware registers (0xCC000000-0xCDFFFFFF) are never
// mapped; accesses there are routed to mmio_* handlers (GX FIFO, VI, PI, ...).
// Guest memory is big-endian; every load/store byte-swaps.
#pragma once

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include "native_dirty.h"
#include "ppc_quant_scale.h"

#if defined(_MSC_VER) && !defined(__clang__)
#include <stdlib.h>
#define PPC_BSWAP16 _byteswap_ushort
#define PPC_BSWAP32 _byteswap_ulong
#define PPC_BSWAP64 _byteswap_uint64
#define PPC_UNLIKELY(x) (x)
#define PPC_FORCEINLINE __forceinline
#else
#define PPC_BSWAP16 __builtin_bswap16
#define PPC_BSWAP32 __builtin_bswap32
#define PPC_BSWAP64 __builtin_bswap64
#define PPC_UNLIKELY(x) __builtin_expect(!!(x), 0)
#define PPC_FORCEINLINE inline __attribute__((always_inline))
#endif

struct FPR {
    union {
        double ps0;
        uint64_t u0;
    };
    double ps1;
};

struct PPCContext {
    uint32_t r[32];
    FPR f[32];
    uint32_t cr;            // CR0 in bits 31..28, CR7 in bits 3..0 (PowerPC bit order)
    uint32_t lr;
    uint32_t ctr;
    uint32_t ca, so, ov;    // XER bits, kept separately
    uint32_t xer_bc;        // XER byte count (lswx/stswx)
    uint32_t gqr[8];        // SPR 912-919
    uint32_t fpscr;
    uint32_t msr;
    uint32_t sr[16];
    uint32_t spr[1024];     // everything else, by SPR number
};

using PPCFunc = void (*)(PPCContext& c, uint8_t* m);
struct PPCFuncEntry {
    uint32_t addr;
    PPCFunc fn;
};

#define PPC_FUNC(name) void name(PPCContext& c, uint8_t* m)

// Guest call stack, for crash reports and logs. Each guest thread gets its own;
// the scheduler points g_ppc_trace at the running thread's.
struct PPCTrace {
    static constexpr uint32_t kMax = 4096;
    uint32_t stack[kMax];
    uint32_t depth;
};
extern PPCTrace* g_ppc_trace;
struct PPCEnterGuard {
    explicit PPCEnterGuard(uint32_t addr) {
        PPCTrace* t = g_ppc_trace;
        if (t->depth < PPCTrace::kMax) t->stack[t->depth] = addr;
        t->depth++;
    }
    ~PPCEnterGuard() { g_ppc_trace->depth--; }
};
// Function boundaries provide cooperative interrupt safe points in original
// polling loops that never call OSRestoreInterrupts. Disabled in CPU difftests.
// Called at every guest function entry, so the common case is inline: interrupt
// delivery keeps its cadence (every 64th poll counted while polling is enabled and
// MSR[EE]=1); the host stop check and RSO attach run on that path or every 64th call.
void ppc_poll_slow(PPCContext& c,uint8_t* m,bool deliver);
extern bool g_ppc_poll_enabled;
extern uint32_t g_ppc_poll_calls,g_ppc_poll_host;
extern PPCContext* g_ppc_context_for_debug;
inline void ppc_poll(PPCContext& c,uint8_t* m) {
    g_ppc_context_for_debug=&c;
    const bool deliver=g_ppc_poll_enabled&&(c.msr&0x8000u /*MSR_EE*/)&&((++g_ppc_poll_calls&63)==0);
    if(PPC_UNLIKELY(deliver||((++g_ppc_poll_host&63)==0)))ppc_poll_slow(c,m,deliver);
}
void ppc_enable_interrupt_polling();
#define PPC_ENTER(addr) ppc_poll(c,m); PPCEnterGuard ppc_enter_guard_(addr)

struct PPCFuncName {
    uint32_t addr, end;
    const char* name;
};

// ---- provided by the generated code / runtime ----------------------------
extern const PPCFuncEntry g_ppc_funcs[];
extern const size_t g_ppc_func_count;
extern const PPCFuncName g_ppc_func_names[];
extern const size_t g_ppc_func_name_count;

void ppc_call_indirect(PPCContext& c, uint8_t* m, uint32_t addr);
uint32_t ppc_mfspr(PPCContext& c, uint32_t spr);
void ppc_mtspr(PPCContext& c, uint8_t* m, uint32_t spr, uint32_t value);
uint32_t ppc_mftb(PPCContext& c, uint32_t tbr);
void ppc_mtmsr(PPCContext& c, uint8_t* m, uint32_t value);
void ppc_syscall(PPCContext& c, uint8_t* m);
void ppc_rfi(PPCContext& c, uint8_t* m);
void ppc_trap(PPCContext& c, uint32_t addr);
void ppc_illegal(PPCContext& c, uint32_t addr);
void ppc_fell_off(PPCContext& c, uint32_t addr);
void ppc_unimplemented(PPCContext& c, uint32_t addr, const char* what);

uint8_t mmio_read8(uint32_t addr);
uint16_t mmio_read16(uint32_t addr);
uint32_t mmio_read32(uint32_t addr);
void mmio_write8(uint32_t addr, uint8_t v);
void mmio_write16(uint32_t addr, uint16_t v);
void mmio_write32(uint32_t addr, uint32_t v);

// ---- memory ------------------------------------------------------------
static inline bool ppc_is_mmio(uint32_t a) { return (a & 0xFE000000u) == 0xCC000000u; }

static PPC_FORCEINLINE uint8_t ld8(uint8_t* m, uint32_t a) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) return mmio_read8(a);
    return m[a];
}
static PPC_FORCEINLINE uint32_t ld16(uint8_t* m, uint32_t a) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) return mmio_read16(a);
    uint16_t v;
    std::memcpy(&v, m + a, 2);
    return PPC_BSWAP16(v);
}
static PPC_FORCEINLINE uint32_t lds16(uint8_t* m, uint32_t a) { return (uint32_t)(int32_t)(int16_t)ld16(m, a); }
static PPC_FORCEINLINE uint32_t ld32(uint8_t* m, uint32_t a) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) return mmio_read32(a);
    uint32_t v;
    std::memcpy(&v, m + a, 4);
    return PPC_BSWAP32(v);
}
static PPC_FORCEINLINE uint64_t ld64(uint8_t* m, uint32_t a) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) return ((uint64_t)mmio_read32(a) << 32) | mmio_read32(a + 4);
    uint64_t v;
    std::memcpy(&v, m + a, 8);
    return PPC_BSWAP64(v);
}
static PPC_FORCEINLINE void st8(uint8_t* m, uint32_t a, uint32_t v) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) { mmio_write8(a, (uint8_t)v); return; }
    m[a] = (uint8_t)v;native_memory_written(a,1);
}
static PPC_FORCEINLINE void st16(uint8_t* m, uint32_t a, uint32_t v) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) { mmio_write16(a, (uint16_t)v); return; }
    uint16_t s = PPC_BSWAP16((uint16_t)v);
    std::memcpy(m + a, &s, 2);native_memory_written(a,2);
}
static PPC_FORCEINLINE void st32(uint8_t* m, uint32_t a, uint32_t v) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) { mmio_write32(a, v); return; }
    uint32_t s = PPC_BSWAP32(v);
    std::memcpy(m + a, &s, 4);native_memory_written(a,4);
}
static PPC_FORCEINLINE void st64(uint8_t* m, uint32_t a, uint64_t v) {
    if (PPC_UNLIKELY(ppc_is_mmio(a))) { mmio_write32(a, (uint32_t)(v >> 32)); mmio_write32(a + 4, (uint32_t)v); return; }
    uint64_t s = PPC_BSWAP64(v);
    std::memcpy(m + a, &s, 8);native_memory_written(a,8);
}
// byte-reversed forms: guest little-endian = host order
static PPC_FORCEINLINE uint32_t ld32le(uint8_t* m, uint32_t a) { return PPC_BSWAP32(ld32(m, a)); }
static PPC_FORCEINLINE uint32_t ld16le(uint8_t* m, uint32_t a) { return PPC_BSWAP16((uint16_t)ld16(m, a)); }
static PPC_FORCEINLINE void st32le(uint8_t* m, uint32_t a, uint32_t v) { st32(m, a, PPC_BSWAP32(v)); }
static PPC_FORCEINLINE void st16le(uint8_t* m, uint32_t a, uint32_t v) { st16(m, a, PPC_BSWAP16((uint16_t)v)); }

static PPC_FORCEINLINE float ldf32(uint8_t* m, uint32_t a) {
    uint32_t u = ld32(m, a);
    float f;
    std::memcpy(&f, &u, 4);
    return f;
}
static PPC_FORCEINLINE void stf32(uint8_t* m, uint32_t a, float f) {
    uint32_t u;
    std::memcpy(&u, &f, 4);
    st32(m, a, u);
}

static PPC_FORCEINLINE void ppc_dcbz(uint8_t* m, uint32_t a) { std::memset(m + (a & ~31u), 0, 32);native_memory_written(a&~31u,32); }

// ---- condition register ------------------------------------------------
static inline uint32_t cr_bit(const PPCContext& c, int bi) { return (c.cr >> (31 - bi)) & 1; }
static inline void cr_set_bit(PPCContext& c, int bi, uint32_t v) {
    uint32_t mask = 1u << (31 - bi);
    c.cr = (c.cr & ~mask) | (v ? mask : 0);
}
static inline uint32_t cr_get_field(const PPCContext& c, int f) { return (c.cr >> (28 - 4 * f)) & 0xF; }
static inline void cr_set_field(PPCContext& c, int f, uint32_t nib) {
    int sh = 28 - 4 * f;
    c.cr = (c.cr & ~(0xFu << sh)) | ((nib & 0xF) << sh);
}
static inline void cr_cmp_s(PPCContext& c, int f, int32_t a, int32_t b) {
    cr_set_field(c, f, (a < b ? 8 : a > b ? 4 : 2) | (c.so & 1));
}
static inline void cr_cmp_u(PPCContext& c, int f, uint32_t a, uint32_t b) {
    cr_set_field(c, f, (a < b ? 8 : a > b ? 4 : 2) | (c.so & 1));
}
static PPC_FORCEINLINE void cr_cmp_f(PPCContext& c, int f, double a, double b) {
    uint32_t n = std::isnan(a) || std::isnan(b) ? 1 : a < b ? 8 : a > b ? 4 : 2;
    cr_set_field(c, f, n);
    c.fpscr = (c.fpscr & ~0xF000u) | (n << 12);  // FPCC
}
static inline void cr0_rec(PPCContext& c, uint32_t v) { cr_cmp_s(c, 0, (int32_t)v, 0); }
static inline void cr1_rec_fp(PPCContext& c) { cr_set_field(c, 1, c.fpscr >> 28); }

static inline uint32_t ppc_get_xer(const PPCContext& c) {
    return (c.so << 31) | (c.ov << 30) | (c.ca << 29) | (c.xer_bc & 0x7F);
}
static inline void ppc_set_xer(PPCContext& c, uint32_t v) {
    c.so = (v >> 31) & 1;
    c.ov = (v >> 30) & 1;
    c.ca = (v >> 29) & 1;
    c.xer_bc = v & 0x7F;
}

// ---- integer helpers ---------------------------------------------------
static inline uint32_t ppc_rotl(uint32_t v, uint32_t n) { n &= 31; return n ? (v << n) | (v >> (32 - n)) : v; }
static inline uint32_t ppc_cntlzw(uint32_t v) {
    if (!v) return 32;
#if defined(_MSC_VER) && !defined(__clang__)
    unsigned long i;
    _BitScanReverse(&i, v);
    return 31 - i;
#else
    return (uint32_t)__builtin_clz(v);
#endif
}
static inline uint32_t ppc_slw(uint32_t s, uint32_t b) { b &= 0x3F; return b > 31 ? 0 : s << b; }
static inline uint32_t ppc_srw(uint32_t s, uint32_t b) { b &= 0x3F; return b > 31 ? 0 : s >> b; }
static inline uint32_t ppc_srawi(PPCContext& c, uint32_t s, uint32_t sh) {
    int32_t v = (int32_t)s;
    if (sh == 0) { c.ca = 0; return s; }
    c.ca = (v < 0 && (s & ((1u << sh) - 1))) ? 1 : 0;
    return (uint32_t)(v >> sh);
}
static inline uint32_t ppc_sraw(PPCContext& c, uint32_t s, uint32_t b) {
    b &= 0x3F;
    if (b > 31) { c.ca = ((int32_t)s < 0); return (int32_t)s < 0 ? 0xFFFFFFFFu : 0; }
    return ppc_srawi(c, s, b);
}
static inline uint32_t ppc_divw(uint32_t a, uint32_t b) {
    int32_t x = (int32_t)a, y = (int32_t)b;
    if (y == 0 || (x == INT32_MIN && y == -1)) return x < 0 ? 0xFFFFFFFFu : 0;
    return (uint32_t)(x / y);
}
static inline uint32_t ppc_divwu(uint32_t a, uint32_t b) { return b ? a / b : 0; }

// ---- floating point ----------------------------------------------------
static inline double ppc_round_single(double x) { return (double)(float)x; }

static inline uint64_t ppc_fctiw_impl(double b, int mode) {
    int32_t r;
    if (std::isnan(b)) r = INT32_MIN;
    else {
        double t;
        switch (mode) {
            case 1: t = std::trunc(b); break;
            case 2: t = std::ceil(b); break;
            case 3: t = std::floor(b); break;
            default: t = std::nearbyint(b); break;
        }
        if (t >= 2147483647.0) r = INT32_MAX;
        else if (t <= -2147483648.0) r = INT32_MIN;
        else r = (int32_t)t;
    }
    // Broadway retains the input sign as bit32 when conversion produces0.
    // Dolphin Interpreter_FloatingPoint.cpp, pinned409881c, HW-tested marker.
    uint64_t encoded = 0xFFF8000000000000ull | (uint32_t)r;
    if(r==0&&std::signbit(b))encoded|=uint64_t(1)<<32;
    return encoded;
}
static inline uint64_t ppc_fctiwz(PPCContext&, double b) { return ppc_fctiw_impl(b, 1); }
static inline uint64_t ppc_fctiw(PPCContext& c, double b) { return ppc_fctiw_impl(b, c.fpscr & 3); }
static inline void ppc_mtfsf(PPCContext& c, uint32_t fm, uint32_t v) {
    uint32_t mask = 0;
    for (int i = 0; i < 8; i++)
        if (fm & (0x80u >> i)) mask |= 0xF0000000u >> (4 * i);
    c.fpscr = (c.fpscr & ~mask) | (v & mask);
}

// ---- Gekko quantized load/store (psq_l / psq_st) ------------------------
static inline double ppc_dequant(uint8_t* m, uint32_t ea, uint32_t type, int scale) {
    double f = ppc_quant_factor(-scale);
    switch (type) {
        case 4: return ld8(m, ea) * f;
        case 5: return ld16(m, ea) * f;
        case 6: return (int8_t)ld8(m, ea) * f;
        case 7: return (int16_t)ld16(m, ea) * f;
        default: return (double)ldf32(m, ea);
    }
}
static inline void ppc_quant(uint8_t* m, uint32_t ea, uint32_t type, int scale, double v) {
    double s = v * ppc_quant_factor(scale);
    auto clamp = [](double x, double lo, double hi) { return x < lo ? lo : x > hi ? hi : x; };
    switch (type) {
        case 4: st8(m, ea, (uint32_t)clamp(s, 0, 255)); break;
        case 5: st16(m, ea, (uint32_t)clamp(s, 0, 65535)); break;
        case 6: st8(m, ea, (uint32_t)(int32_t)clamp(s, -128, 127)); break;
        case 7: st16(m, ea, (uint32_t)(int32_t)clamp(s, -32768, 32767)); break;
        default: stf32(m, ea, (float)v); break;
    }
}
static inline int ppc_qscale(uint32_t bits6) { return (int)((int32_t)(bits6 << 26) >> 26); }
static inline uint32_t ppc_qsize(uint32_t type) { return type == 4 || type == 6 ? 1 : type == 5 || type == 7 ? 2 : 4; }

static inline void psq_load(PPCContext& c, uint8_t* m, uint32_t ea, int fd, int w, uint32_t gqr) {
    uint32_t type = (gqr >> 16) & 7;
    int scale = ppc_qscale((gqr >> 24) & 0x3F);
    double p0 = ppc_dequant(m, ea, type, scale);
    double p1 = w ? 1.0 : ppc_dequant(m, ea + ppc_qsize(type), type, scale);
    c.f[fd].ps0 = p0;
    c.f[fd].ps1 = p1;
}
static inline void psq_store(PPCContext& c, uint8_t* m, uint32_t ea, int fs, int w, uint32_t gqr) {
    uint32_t type = gqr & 7;
    int scale = ppc_qscale((gqr >> 8) & 0x3F);
    ppc_quant(m, ea, type, scale, c.f[fs].ps0);
    if (!w) ppc_quant(m, ea + ppc_qsize(type), type, scale, c.f[fs].ps1);
}

// ---- string loads/stores (lswi/stswi) -----------------------------------
static inline void ppc_lswi(PPCContext& c, uint8_t* m, int rd, uint32_t ea, int nb) {
    int r = rd - 1, shift = 0;
    for (int i = 0; i < nb; i++) {
        if (shift == 0) { r = (r + 1) & 31; c.r[r] = 0; }
        c.r[r] |= (uint32_t)ld8(m, ea + i) << (24 - shift);
        shift = (shift + 8) & 31;
    }
}
static inline void ppc_stswi(PPCContext& c, uint8_t* m, int rs, uint32_t ea, int nb) {
    int r = rs - 1, shift = 0;
    for (int i = 0; i < nb; i++) {
        if (shift == 0) r = (r + 1) & 31;
        st8(m, ea + i, c.r[r] >> (24 - shift));
        shift = (shift + 8) & 31;
    }
}
