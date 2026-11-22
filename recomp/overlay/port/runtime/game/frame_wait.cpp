#include "hle.h"
#include "native_config.h"
#include "os.h"
uint64_t vi_next_retrace_tb();  // hw/vi.cpp
// Frame-rate wait (fn_8026F324, called once per game frame). The original spins
// on ppc_poll until the retrace counter at 0x806A4174 (bumped by fn_8026F364
// from the VI retrace callback) reaches the frames-per-update target stored at
// 0x805CBE20, then clears the counter. At the frame cap that spin took a third
// of the CPU thread. Same prologue, call, polling, compare, CR0, registers,
// stack and counter reset as the original; when the counter is short and
// interrupts can be delivered, due events are delivered and the thread sleeps
// until the next hardware deadline instead of spinning. The retrace interrupt
// is the only thing that can end the wait, so the observed count, the exit
// point and frame cadence are unchanged. Opt-in (MW_FRAME_WAIT_HLE=1) until frame
// pacing is verified against the original spin; otherwise the original body runs.
void native_frame_record_tick(uint8_t* m);  // game/frame_record.cpp (MW_FRAME_RECORD / MW_INPUT_REPLAY, default off)
PPC_FUNC(f_8026F324) {
    native_frame_record_tick(m);
    static const bool hle = [] { const char* v = native_config_get("MW_FRAME_WAIT_HLE"); return v && v[0] == '1' && !v[1]; }();
    if (!hle) {
        f_8026F324_recomp(c, m);
        return;
    }
    PPC_ENTER(0x8026F324u);
    { uint32_t ea = (c.r[1] + (uint32_t)-16); st32(m, ea, c.r[1]); c.r[1] = ea; }
    c.r[0] = c.lr;
    st32(m, (c.r[1] + (uint32_t)20), c.r[0]);
    c.lr = 0x8026F334u; f_802F7AD4(c, m);
    c.r[4] = 0u + ((uint32_t)-32675 << 16);
    c.r[3] = 0u + ((uint32_t)-32662 << 16);
    c.r[4] = ld32(m, (c.r[4] + (uint32_t)-16864));
    for (;;) {
        ppc_poll(c, m);  // the original loop's safe point, unchanged
        c.r[0] = ld32(m, (c.r[3] + (uint32_t)16756));
        cr_cmp_u(c, 0, c.r[0], c.r[4]);
        if (!cr_bit(c, 0)) break;
        if (g_ppc_poll_enabled && (c.msr & MSR_EE)) {
            // Deliver whatever is due now. While the next retrace (the only event
            // that can end this wait) is far away, sleep until the next hardware
            // event; close to it, spin like the original so the frame end is seen
            // immediately. The game adapts its frame rate to how late frames
            // start, so host timer oversleep (up to ~1.5 ms) must not delay it.
            os_deliver_interrupts(c, m);
            constexpr uint64_t spin_window = 182250;  // 3 ms: 1 ms wait cap + oversleep + margin
            if (ld32(m, (c.r[3] + (uint32_t)16756)) < c.r[4] && vi_next_retrace_tb() > ppc_timebase() + spin_window)
                os_wait_for_event();
        }
    }
    c.r[0] = 0u + (uint32_t)0;
    st32(m, (c.r[3] + (uint32_t)16756), c.r[0]);
    c.r[0] = ld32(m, (c.r[1] + (uint32_t)20));
    c.lr = c.r[0];
    c.r[1] = c.r[1] + (uint32_t)16;
}
