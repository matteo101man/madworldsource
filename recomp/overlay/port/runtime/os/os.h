// Host-side OS core: threads (fibers) and interrupt delivery.
#pragma once

#include <cstdint>
#include <functional>

#include "ppc_context.h"

constexpr uint32_t MSR_EE = 0x8000;  // external interrupts enabled

void os_threads_init(uint8_t* m, PPCContext* main_ctx);

// Interrupt frames. On hardware OSLoadContext never returns: interrupt handlers end
// with it to resume the interrupted context. Here, delivery runs inside a frame and
// OSLoadContext(interrupted context) unwinds back to it by throwing this.
struct OSInterruptReturn {};
void os_irq_push(uint32_t ctx);   // per fiber
void os_irq_pop();
bool os_irq_is_return(uint32_t ctx);

// Interrupt sources ------------------------------------------------------
// PI interrupt causes (INTSR bits)
constexpr uint32_t PI_INT_VI = 1u << 8;
constexpr uint32_t PI_INT_PE_TOKEN = 1u << 9;
constexpr uint32_t PI_INT_PE_FINISH = 1u << 10;
constexpr uint32_t PI_INT_CP = 1u << 11;
constexpr uint32_t PI_INT_DSP = 1u << 6;
constexpr uint32_t PI_INT_AI = 1u << 5;
constexpr uint32_t PI_INT_EXI = 1u << 4;
constexpr uint32_t PI_INT_HOLLYWOOD = 1u << 14;  // Hollywood IRQs (IPC, GPIO...)

// Hollywood PPC IRQ flags (0xCD000030)
constexpr uint32_t HW_IRQ_IPC = 1u << 30;

void pi_set_cause(uint32_t bits, bool on);
void hollywood_set_irq(uint32_t bits, bool on);

// Timed host events (in 60.75 MHz time base ticks), run from interrupt delivery.
enum class OSEventKind : uint32_t { Ai, Dsp, Vi, DiRead, IpcAck, Bluetooth, Count };
struct OSEventData {
    OSEventKind kind{};
    uint32_t a=0,b=0,c=0;
    uint64_t value=0;
};
using OSEventHandler = void (*)(const OSEventData&);
void os_register_event(OSEventKind, OSEventHandler);
void os_schedule(uint64_t when_tb, OSEventData);

// Delivers pending interrupts if MSR[EE] allows (runs guest handlers).
void os_deliver_interrupts(PPCContext& c, uint8_t* m);
// Hardware progresses even when the guest masks interrupt delivery.
void os_poll_hardware_events();
// Sleeps until the next scheduled event or decrementer deadline (idle loop).
void os_wait_for_event();

// Bounded diagnostic snapshot; called only by explicit MW_TRACE_IDLE.
void os_trace_hardware_state();
