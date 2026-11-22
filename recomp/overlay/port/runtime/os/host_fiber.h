#pragma once
#include <cstddef>

// Windows x64 ABI context. Saved registers live in the owned stack frame.
struct HostFiber {
    void* rsp = nullptr;
    void* allocation = nullptr;
    size_t slot = 0;
    void* vcrt = nullptr;
    void* ucrt = nullptr;
};
static_assert(offsetof(HostFiber, rsp) == 0 && sizeof(void*) == 8);
extern "C" void mw_fiber_switch(HostFiber* from, HostFiber* to);
extern "C" void mw_fiber_prepare(HostFiber*, void* top, void* reservation, void (*entry)(void*), void* arg);
HostFiber* host_fiber_current();
bool host_fiber_create(HostFiber&, void (*entry)(void*), void* arg);
void host_fiber_destroy(HostFiber&);
void host_fiber_switch(HostFiber& to);
// Runs a returning callback on an owned stack; exceptions rethrow on the caller.
void host_fiber_call(void (*entry)(void*), void* arg);
class StateArchive;
void host_fiber_state(StateArchive&,HostFiber&);
HostFiber* os_threads_resume();
