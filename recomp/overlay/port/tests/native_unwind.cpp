// Test the actual host bridge: nested frames must unwind before continuation.
#include "ppc_memory.h"
#include "../runtime/game/native_unwind.cpp"
#include <cstdarg>
#include <exception>

uint32_t mmio_read32(uint32_t) { std::abort(); }
void mmio_write32(uint32_t, uint32_t) { std::abort(); }
[[noreturn]] void ppc_fatal(const char* format, ...) {
    va_list args; va_start(args, format); std::vfprintf(stderr, format, args); va_end(args); std::abort();
}
int destroyed = 0, continued = 0;
bool returned_to_thrower = false;
struct Scope { ~Scope() { destroyed++; } };
void deep(PPCContext& c, uint8_t* m) {
    Scope inner;
    c.r[4] = 0x12345678;
    ppc_native_unwind(c, m, 0x8021f3a4);
    returned_to_thrower = true;
}
PPC_FUNC(f_8021F368_recomp) {
    Scope outer;
    uint32_t caller = c.r[1];
    c.r[1] -= 32;
    st32(m, c.r[1], caller);
    deep(c, m);
    returned_to_thrower = true;
}
PPC_FUNC(f_8021F3A4) {
    if (std::current_exception()) std::abort();
    if (destroyed != 2 || c.r[4] != 0x12345678) std::abort();
    continued++;
    c.r[1] = ld32(m, c.r[1]);
}
int main() {
    uint8_t* m = ppc_memory_init(); if (!m) return 2;
    PPCContext c{}; c.r[1] = 0x80010000;
    f_8021F368(c, m);
    bool valid = destroyed == 2 && continued == 1 && !returned_to_thrower && c.r[1] == 0x80010000;
    ppc_memory_shutdown();
    if (!valid) return 1;
    std::puts("Native unwind bridge: two host frames unwind before matching native continuation; restored guest registers/SP retained");
}
