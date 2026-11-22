#pragma once
#include "ppc_context.h"

// Retail enemy module text: canonical address is an analysis key, never a heap pin.
constexpr uint32_t MW_RSO_BASE = 0x81000000u;
constexpr uint32_t MW_RSO_TEXT_OFF = 248u;
constexpr uint32_t MW_RSO_TEXT_SIZE = 1188540u;
extern const PPCFuncEntry g_rso_funcs[];
extern const size_t g_rso_func_count;
extern const PPCFuncName g_rso_func_names[];
extern const size_t g_rso_func_name_count;
inline uint32_t mw_rso_base(uint8_t* m) { return ld32(m, 0x8060BCACu); }
inline uint32_t mw_rso_runtime_pc(uint8_t* m, uint32_t canonical) {
    return mw_rso_base(m) + canonical - MW_RSO_BASE;
}
inline uint32_t mw_rso_canonical_pc(uint8_t* m, uint32_t actual) {
    uint32_t base = mw_rso_base(m);
    return base && actual - base - MW_RSO_TEXT_OFF < MW_RSO_TEXT_SIZE
        ? MW_RSO_BASE + actual - base : actual;
}
inline uint32_t mw_rso_branch_target(uint8_t* m, uint32_t canonical) {
    uint32_t pc = mw_rso_runtime_pc(m, canonical), word = ld32(m, pc);
    int32_t displacement = (int32_t)(word << 6) >> 6;
    displacement &= ~3;
    return (word & 2) ? (uint32_t)displacement : pc + displacement;
}
void mw_rso_attach(uint8_t* m);
PPCFunc mw_rso_lookup(uint32_t actual);
// Load base of the attached module (0 when none); keys lookup caches.
uint32_t mw_rso_current_base();
const char* mw_rso_func_name(uint32_t address, uint32_t* start);
