#include "host_fiber.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cfenv>
#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <stdexcept>
#include <malloc.h>
#include <cerrno>

void require(bool ok) { if (!ok) std::abort(); }
uintptr_t stack_base() { return reinterpret_cast<uintptr_t>(reinterpret_cast<NT_TIB*>(NtCurrentTeb())->StackBase); }
uintptr_t stack_limit() { return reinterpret_cast<uintptr_t>(reinterpret_cast<NT_TIB*>(NtCurrentTeb())->StackLimit); }
struct Probe { HostFiber* caller; unsigned rounds = 0; };
void worker(void* arg) {
    auto& probe = *static_cast<Probe*>(arg);
    volatile uint64_t marker = 0x123456789abcdef0ull;
    // Exercise compiler stack probes past the permanently inaccessible guard.
    volatile char* scratch = static_cast<char*>(_alloca(128u << 10));
    for (size_t i = 0; i < (128u << 10); i += 4096) scratch[i] = static_cast<char>(i / 4096);
    require(reinterpret_cast<uintptr_t>(&marker) > stack_limit());
    require(reinterpret_cast<uintptr_t>(&marker) < stack_base());
    require(stack_base() - stack_limit() == (8u << 20) - (64u << 10));
    std::fesetround(FE_DOWNWARD);
    errno = 234;
    for (unsigned i = 0; i < 512; ++i) {
        require(std::fegetround() == FE_DOWNWARD && marker == 0x123456789abcdef0ull);
        for (size_t j = 0; j < (128u << 10); j += 4096) require(scratch[j] == static_cast<char>(j / 4096));
        // A native task may yield inside an active C++ catch. The next fiber
        // must never inherit its EH chain (which points into this stack).
        try { throw i; } catch (unsigned value) {
            require(value == i && errno == 234);
            ++probe.rounds;
            host_fiber_switch(*probe.caller);
            require(errno == 234);
        }
    }
    std::abort();
}
int destroyed = 0;
struct Scope { ~Scope() { ++destroyed; } };
void throwing(void*) { Scope scope; throw std::runtime_error("owned stack exception"); }
int main() {
    auto* root = host_fiber_current();
    auto base = stack_base(), limit = stack_limit();
    int rounding = std::fegetround();
    std::fesetround(FE_UPWARD);
    errno = 123;
    HostFiber fiber;
    Probe probe{root};
    require(host_fiber_create(fiber, worker, &probe));
    void* allocation = fiber.allocation;
    for (unsigned i = 0; i < 512; ++i) {
        host_fiber_switch(fiber);
        require(probe.rounds == i + 1 && std::fegetround() == FE_UPWARD);
        require(errno == 123);
        try { throw i; } catch (unsigned value) { require(value == i); }
        require(host_fiber_current() == root && stack_base() == base && stack_limit() == limit);
    }
    host_fiber_destroy(fiber);
    require(host_fiber_create(fiber, worker, &probe));
    require(fiber.allocation == allocation);
    host_fiber_destroy(fiber);
    for (int i = 0; i < 3; ++i) {
        bool caught = false;
        try { host_fiber_call(throwing, nullptr); } catch (const std::runtime_error&) { caught = true; }
        require(caught && destroyed == i + 1 && host_fiber_current() == root);
        require(stack_base() == base && stack_limit() == limit && std::fegetround() == FE_UPWARD);
    }
    std::fesetround(rounding);
    std::puts("Owned fibers: 512 yields, stack probes, TEB/FP restore, fixed slot reuse and exception cleanup pass");
}
