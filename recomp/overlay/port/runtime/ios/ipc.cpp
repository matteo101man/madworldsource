#include "native_config.h"
// IPC registers between the PowerPC and IOS (0xCD000000).
//
//   +0x0 PPCMSG   request address (physical), written by the game
//   +0x4 PPCCTRL  X1 (1): send request   Y2 (2): request acknowledged   Y1 (4): reply ready
//                 X2 (8): relaunch       IY1 (0x10) / IY2 (0x20): interrupt enables for Y1 / Y2
//   +0x8 ARMMSG   reply address (physical), read by the game
//
// A request is executed as soon as X1 is written. The acknowledgement (Y2) and the
// reply (Y1, with ARMMSG) are signalled through the Hollywood IPC interrupt, one at a time.
#include <cstdio>
#include <cstdlib>
#include <deque>

#include "ios.h"
#include "mmio.h"
#include "os.h"

namespace {
constexpr uint32_t X1 = 1, Y2 = 2, Y1 = 4, X2 = 8, IY1 = 0x10, IY2 = 0x20;

uint32_t g_ppc_msg = 0, g_arm_msg = 0;
uint32_t g_ctrl = 0;
std::deque<uint32_t> g_replies;
bool g_trace = native_config_get("MW_TRACE_IPC") != nullptr;

void update_irq() {
    bool irq = ((g_ctrl & Y1) && (g_ctrl & IY1)) || ((g_ctrl & Y2) && (g_ctrl & IY2));
    hollywood_set_irq(HW_IRQ_IPC, irq);
}

void try_send_reply() {
    // A reply goes out once the previous reply and the request ack have been handled.
    if ((g_ctrl & (Y1 | Y2)) || g_replies.empty()) return;
    g_arm_msg = g_replies.front();
    if (g_trace) std::fprintf(stderr, "[ipc] reply 0x%08X\n", g_arm_msg);
    g_replies.pop_front();
    g_ctrl |= Y1;
}

void acknowledge(const OSEventData& event) {
    g_ctrl&=~X1;g_ctrl|=Y2;
    ios_execute(event.a);
    try_send_reply();update_irq();
}

void write_ctrl(uint32_t v) {
    if (g_trace) std::fprintf(stderr, "[ipc] ctrl write 0x%02X (ctrl was 0x%02X)\n", v, g_ctrl);
    if (v & Y1) g_ctrl &= ~Y1;  // game consumed the reply
    if (v & Y2) g_ctrl &= ~Y2;  // game saw the acknowledgement
    g_ctrl = (g_ctrl & ~(IY1 | IY2)) | (v & (IY1 | IY2));
    if (v & X1) {
        if(g_ctrl&X1)ppc_fatal("overlapping IOS request before acknowledgement");
        g_ctrl|=X1;
        uint32_t request=g_ppc_msg;
        // Dolphin hardware measurements: ACK takes500 Wii time-base ticks.
        // Latch the pointer now; a later PPCMSG write cannot change this request.
        os_schedule(ppc_timebase()+500,{OSEventKind::IpcAck,request});
    }
    try_send_reply();
    update_irq();
}
}  // namespace

void ipc_queue_reply(uint32_t req_phys) {
    g_replies.push_back(req_phys);
    try_send_reply();
    update_irq();
}

MMIO_DEVICE(ipc) {
    os_register_event(OSEventKind::IpcAck, acknowledge);
    mmio_register(
        0xCD000000u, 0x10,
        [](uint32_t a, int) -> uint32_t {
            if (g_trace) std::fprintf(stderr, "[ipc] read 0x%08X (ctrl 0x%02X)\n", a, g_ctrl);
            switch (a & 0xF) {
                case 0x0: return g_ppc_msg;
                case 0x4: return g_ctrl;
                case 0x8: return g_arm_msg;
            }
            return 0;
        },
        [](uint32_t a, uint32_t v, int) {
            switch (a & 0xF) {
                case 0x0: g_ppc_msg = v; break;
                case 0x4: write_ctrl(v); break;
            }
        });
}

#include "snapshot.h"
void native_ipc_state(StateArchive& ar){ar.pod(g_ppc_msg);ar.pod(g_arm_msg);ar.pod(g_ctrl);std::vector<uint32_t> values;if(!ar.reading)values.assign(g_replies.begin(),g_replies.end());ar.vector(values,65536);if(ar.reading)g_replies.assign(values.begin(),values.end());}
