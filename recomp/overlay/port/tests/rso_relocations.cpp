#include "rso.h"
#include "ppc_memory.h"
#include <cstdio>
#include <cstdlib>
#include <initializer_list>

uint32_t called = 0;
PPCTrace trace{};
PPCTrace* g_ppc_trace = &trace;
bool g_ppc_poll_enabled=false;
uint32_t g_ppc_poll_calls=0,g_ppc_poll_host=0;
PPCContext* g_ppc_context_for_debug=nullptr;
void ppc_poll_slow(PPCContext&, uint8_t*, bool) {}
uint8_t* g_mem=nullptr;
uint8_t mmio_read8(uint32_t) { std::abort(); }
uint32_t mmio_read32(uint32_t) { std::abort(); }
uint16_t mmio_read16(uint32_t) { std::abort(); }
void mmio_write32(uint32_t, uint32_t) { std::abort(); }
void mmio_write16(uint32_t, uint16_t) { std::abort(); }
void ppc_call_indirect(PPCContext&, uint8_t*, uint32_t target) { called = target; }
#include "rso_fixture.inc"
const PPCFuncEntry g_rso_funcs[] = {{0x810000f8, f_810000F8}, {0x81000108, f_81000108}};
const size_t g_rso_func_count = 2;
const PPCFuncName g_rso_func_names[] = {{0x810000f8, 0x81000108, "synthetic_load"}, {0x81000108, 0x81000110, "synthetic_call"}};
const size_t g_rso_func_name_count = 2;
#define CHECK(x) do { if (!(x)) { std::fprintf(stderr, "FAIL %s\n", #x); return 1; } } while(0)
int main() {
    uint8_t* m = ppc_memory_init(); CHECK(m);
    mw_rso_attach(m);
    PPCContext c{};
    for (uint32_t base : {0x80800000u, 0x81200000u}) {
        st32(m, 0x8060bcac, base);
        uint32_t data = base + 0x1f000;
        st16(m, base + 250, (data + 0x8000) >> 16);
        st16(m, base + 254, data & 0xffff);
        st16(m, base + 258, 4);
        st32(m, data + 4, 0xa5c39e71);
        CHECK(mw_rso_lookup(base + 248) == f_810000F8);
        CHECK(!mw_rso_lookup(base + 249));
        CHECK(!mw_rso_lookup(base + 248 + MW_RSO_TEXT_SIZE));
        mw_rso_lookup(base + 248)(c, m);
        CHECK(c.r[3] == data && c.r[4] == 0xa5c39e71);
        CHECK(mw_rso_canonical_pc(m, base + 248) == 0x810000f8);
        CHECK(mw_rso_canonical_pc(m, 0x80004000) == 0x80004000);
        uint32_t pc = base + 264, target = 0x80008000;
        st32(m, pc, 0x48000001 | ((target - pc) & 0x03fffffc));
        mw_rso_lookup(pc)(c, m);
        CHECK(called == target && c.lr == pc + 4);
        st32(m, pc, 0x48000003 | (target & 0x03fffffc));
        CHECK(mw_rso_branch_target(m, 0x81000108) == 0x00008000);
        CHECK(trace.depth == 0);
    }
    st32(m, 0x8060bcac, 0); CHECK(!mw_rso_lookup(0x810000f8));
    ppc_memory_shutdown();
    std::puts("RSO: production-emitted HA/LO/load/call, two heap placements, lookup bounds, signed REL24, AA and runtime LR passed");
}

void native_snapshot_actor(uint32_t,uint32_t){}
void native_frame_record_rin(uint8_t*,uint32_t){}
