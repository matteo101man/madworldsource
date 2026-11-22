// VI (video interface, 0xCC002000): register storage plus the vertical retrace interrupt.
//
// The game paces itself on retrace (VIWaitForRetrace sleeps until the next one).
// A host timer at the NTSC field rate raises the status bit of every enabled
// display interrupt (DI0-DI3: bit 28 enable, bit 31 status, cleared by the handler
// writing 0) and drives the PI VI interrupt line from them.
#include <cstdio>

#include "mmio.h"
#include "os.h"
#include "platform.h"

namespace {
constexpr uint32_t VI_BASE = 0xCC002000u;
constexpr uint64_t TB_PER_SECOND = 60750000ull;
constexpr uint64_t FIELD_TB = TB_PER_SECOND * 1001 / 60000;  // 59.94 Hz

uint16_t g_regs[0x80];  // 0x100 bytes of 16-bit registers
uint64_t g_next_field = 0;
uint32_t g_field = 0;

uint32_t di(int n) { return (uint32_t)g_regs[(0x30 + 4 * n) / 2] << 16 | g_regs[(0x32 + 4 * n) / 2]; }

void update_line() {
    bool irq = false;
    for (int n = 0; n < 4; n++) {
        uint32_t v = di(n);
        if ((v & (1u << 31)) && (v & (1u << 28))) irq = true;
    }
    pi_set_cause(PI_INT_VI, irq);
}

void retrace(const OSEventData&) {
    g_field++;
    for (int n = 0; n < 4; n++)
        if (di(n) & (1u << 28)) g_regs[(0x30 + 4 * n) / 2] |= 0x8000;  // status (bit 31)
    update_line();
    g_next_field += FIELD_TB;
    uint64_t now = ppc_timebase();
    if (g_next_field < now) g_next_field = now + FIELD_TB;  // don't try to catch up after stalls
    os_schedule(g_next_field, {OSEventKind::Vi});
}

uint32_t vi_read(uint32_t a, int size) {
    uint32_t off = a & 0xFF;
    if (off == 0x2C || off == 0x2E) {
        // VCT / HCT: current beam position, from time within the field.
        uint64_t pos = (ppc_timebase() % FIELD_TB) * 525 / FIELD_TB;
        if (off == 0x2C) return (uint32_t)(pos + 1);
        return 1;
    }
    if (size == 4) return (uint32_t)g_regs[off / 2] << 16 | g_regs[off / 2 + 1];
    if (size == 2) return g_regs[off / 2];
    return (g_regs[off / 2] >> ((off & 1) ? 0 : 8)) & 0xFF;
}

void vi_write(uint32_t a, uint32_t v, int size) {
    native_platform_vi_write(a,v,size);
    uint32_t off = a & 0xFF;
    if (size == 4) {
        g_regs[off / 2] = (uint16_t)(v >> 16);
        g_regs[off / 2 + 1] = (uint16_t)v;
    } else if (size == 2) {
        g_regs[off / 2] = (uint16_t)v;
    } else {
        uint16_t& r = g_regs[off / 2];
        r = (off & 1) ? (uint16_t)((r & 0xFF00) | (v & 0xFF)) : (uint16_t)((r & 0x00FF) | (v << 8));
    }
    if (off >= 0x30 && off < 0x40) update_line();
}
}  // namespace

MMIO_DEVICE(vi) {
    os_register_event(OSEventKind::Vi, retrace);
    mmio_register(VI_BASE, 0x100, vi_read, vi_write);
    g_next_field = ppc_timebase() + FIELD_TB;
    os_schedule(g_next_field, {OSEventKind::Vi});
}

#include "snapshot.h"
// Next scheduled retrace (timebase ticks); frame-rate wait HLE uses it to stop sleeping in time.
uint64_t vi_next_retrace_tb(){return g_next_field;}
void native_vi_state(StateArchive& ar){ar.pod(g_regs);ar.pod(g_next_field);ar.pod(g_field);}
