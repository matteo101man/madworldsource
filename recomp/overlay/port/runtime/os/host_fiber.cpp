#include "host_fiber.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <array>
#include <cstdint>
#include <exception>
#include <mutex>
#include <stdexcept>
#include "snapshot.h"
#include <cerrno>

// MSVC's statically linked runtimes keep EH chains and CRT state in FLS.
// Each owned stack needs independent blocks just as a Win32 fiber did. These
// private symbols are linked from our pinned static CRT; no guessed PTD layout.
extern "C" void* __vcrt_getptd();
extern "C" void* __acrt_getptd_head();
extern "C" void __vcrt_freeptd_for_this_thread();
extern "C" void __acrt_freeptd();
extern "C" void** __current_exception();
extern "C" int* __processing_throw();
extern "C" void mw_fiber_trampoline();

namespace {
struct CrtSlots { DWORD vcrt, ucrt; };
const CrtSlots& crt_slots() {
    static const CrtSlots indices = [] {
        DWORD saved_error = GetLastError();
        void* vcrt = __vcrt_getptd();
        void* ucrt = __acrt_getptd_head();
        CrtSlots found{FLS_OUT_OF_INDEXES, FLS_OUT_OF_INDEXES};
        // Discover slots by the runtime's own PTD addresses, never by a guessed
        // Windows TEB/PTD layout. Invalid FLS indices return null.
        for (DWORD i = 0; i < 4096; ++i) {
            void* value = FlsGetValue(i);
            if (value == vcrt) { if (found.vcrt != FLS_OUT_OF_INDEXES) std::terminate(); found.vcrt = i; }
            if (value == ucrt) { if (found.ucrt != FLS_OUT_OF_INDEXES) std::terminate(); found.ucrt = i; }
        }
        if (found.vcrt == FLS_OUT_OF_INDEXES || found.ucrt == FLS_OUT_OF_INDEXES) std::terminate();
        SetLastError(saved_error);
        return found;
    }();
    return indices;
}
constexpr size_t stack_size = 8u << 20, guard_size = 64u << 10;
constexpr uintptr_t arena_base = 0x40000000000ull;
std::array<bool, 256> slots{};
std::mutex slots_mutex;
thread_local HostFiber root;
thread_local HostFiber* current = &root;
struct Call {
    HostFiber* caller;
    void (*entry)(void*);
    void* arg;
    std::exception_ptr error;
};
void call_main(void* arg) {
    auto& call = *static_cast<Call*>(arg);
    try { call.entry(call.arg); } catch (...) { call.error = std::current_exception(); }
    host_fiber_switch(*call.caller);
    std::terminate();
}
}
HostFiber* host_fiber_current() { return current; }
bool host_fiber_create(HostFiber& fiber, void (*entry)(void*), void* arg) {
    if (fiber.allocation) return false;
    std::lock_guard<std::mutex> lock(slots_mutex);
    for (size_t slot = 0; slot < slots.size(); ++slot) {
        if (slots[slot]) continue;
        auto* base = reinterpret_cast<unsigned char*>(arena_base + slot * stack_size);
        if (VirtualAlloc(base, stack_size, MEM_RESERVE, PAGE_NOACCESS) != base) continue;
        if (!VirtualAlloc(base + guard_size, stack_size - guard_size, MEM_COMMIT, PAGE_READWRITE)) {
            VirtualFree(base, 0, MEM_RELEASE);
            return false;
        }
        slots[slot] = true;
        fiber.allocation = base;
        fiber.slot = slot;
        mw_fiber_prepare(&fiber, base + stack_size, base, entry, arg);
        return true;
    }
    return false;
}
void host_fiber_destroy(HostFiber& fiber) {
    if (!fiber.allocation) return;
    if (&fiber == current) std::terminate();
    std::lock_guard<std::mutex> lock(slots_mutex);
    void* vcrt = FlsGetValue(crt_slots().vcrt);
    void* ucrt = FlsGetValue(crt_slots().ucrt);
    if (!FlsSetValue(crt_slots().vcrt, fiber.vcrt) || !FlsSetValue(crt_slots().ucrt, fiber.ucrt)) std::terminate();
    __vcrt_freeptd_for_this_thread();
    __acrt_freeptd();
    if (!FlsSetValue(crt_slots().vcrt, vcrt) || !FlsSetValue(crt_slots().ucrt, ucrt)) std::terminate();
    if (!VirtualFree(fiber.allocation, 0, MEM_RELEASE)) std::terminate();
    slots[fiber.slot] = false;
    fiber = HostFiber{};
}
void host_fiber_switch(HostFiber& to) {
    HostFiber* from = current;
    if (from == &to) return;
    from->vcrt = FlsGetValue(crt_slots().vcrt);
    from->ucrt = FlsGetValue(crt_slots().ucrt);
    if (!FlsSetValue(crt_slots().vcrt, to.vcrt) || !FlsSetValue(crt_slots().ucrt, to.ucrt)) std::terminate();
    current = &to;
    mw_fiber_switch(from, &to);
}
void host_fiber_call(void (*entry)(void*), void* arg) {
    HostFiber fiber;
    Call call{current, entry, arg, {}};
    if (!host_fiber_create(fiber, call_main, &call)) throw std::runtime_error("Cannot allocate fixed guest boot stack");
    host_fiber_switch(fiber);
    host_fiber_destroy(fiber);
    if (call.error) std::rethrow_exception(call.error);
}

void host_fiber_state(StateArchive& ar,HostFiber& fiber){
    uint64_t slot=fiber.slot, rsp=reinterpret_cast<uint64_t>(fiber.rsp);
    ar.pod(slot);ar.pod(rsp);
    if(slot>=slots.size())throw std::runtime_error("Snapshot stack slot");
    auto* base=reinterpret_cast<uint8_t*>(arena_base+slot*stack_size);
    if(rsp<reinterpret_cast<uint64_t>(base)+guard_size||rsp>reinterpret_cast<uint64_t>(base)+stack_size-264||(rsp&15)!=8)
        throw std::runtime_error("Snapshot stack pointer outside owned stack");
    if(ar.reading){
        if(fiber.allocation&&fiber.slot!=slot)throw std::runtime_error("Snapshot boot stack slot mismatch");
        if(!fiber.allocation){
            std::lock_guard<std::mutex> lock(slots_mutex);
            if(slots[slot]||VirtualAlloc(base,stack_size,MEM_RESERVE,PAGE_NOACCESS)!=base)throw std::runtime_error("Snapshot fixed stack address occupied");
            if(!VirtualAlloc(base+guard_size,stack_size-guard_size,MEM_COMMIT,PAGE_READWRITE)){VirtualFree(base,0,MEM_RELEASE);throw std::runtime_error("Snapshot stack commit failed");}
            slots[slot]=true;fiber.allocation=base;fiber.slot=size_t(slot);
        }
        fiber.rsp=reinterpret_cast<void*>(rsp);
    }
    // Catches must have ended before the checkpoint. Recreate clean runtime
    // blocks in the new process, never restore old heap pointers from CRT PTDs.
    void* saved_vcrt=FlsGetValue(crt_slots().vcrt);void* saved_ucrt=FlsGetValue(crt_slots().ucrt);
    if(!FlsSetValue(crt_slots().vcrt,fiber.vcrt)||!FlsSetValue(crt_slots().ucrt,fiber.ucrt))std::terminate();
    bool active=*__current_exception()!=nullptr||*__processing_throw()!=0;
    int saved_errno=errno;
    fiber.vcrt=FlsGetValue(crt_slots().vcrt);fiber.ucrt=FlsGetValue(crt_slots().ucrt);
    if(!FlsSetValue(crt_slots().vcrt,saved_vcrt)||!FlsSetValue(crt_slots().ucrt,saved_ucrt))std::terminate();
    if(active)throw std::runtime_error("Snapshot refused: active host C++ exception on guest stack");
    ar.pod(saved_errno);
    if(ar.reading){if(!FlsSetValue(crt_slots().ucrt,fiber.ucrt))std::terminate();errno=saved_errno;if(!FlsSetValue(crt_slots().ucrt,saved_ucrt))std::terminate();}
    size_t size=reinterpret_cast<uint64_t>(base)+stack_size-rsp;
    if(!ar.reading){
        HMODULE module=nullptr;
        GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,reinterpret_cast<LPCWSTR>(host_fiber_state),&module);
        auto start=reinterpret_cast<uint64_t>(module);auto* dos=reinterpret_cast<IMAGE_DOS_HEADER*>(module);
        auto end=start+reinterpret_cast<IMAGE_NT_HEADERS64*>(start+dos->e_lfanew)->OptionalHeader.SizeOfImage;
        auto* frame=reinterpret_cast<uint64_t*>(rsp);CONTEXT context{};context.ContextFlags=CONTEXT_FULL;
        context.Rip=frame[32];context.Rsp=rsp+264;
        context.Rbx=frame[31];context.Rbp=frame[30];context.Rsi=frame[29];context.Rdi=frame[28];
        context.R12=frame[27];context.R13=frame[26];context.R14=frame[25];context.R15=frame[24];
        bool complete=false;
        for(unsigned i=0;i<4096;++i){
            if(context.Rip>=reinterpret_cast<uint64_t>(mw_fiber_trampoline)&&context.Rip<reinterpret_cast<uint64_t>(mw_fiber_trampoline)+16){complete=true;break;}
            if(context.Rip<start||context.Rip>=end||context.Rsp<rsp||context.Rsp>=reinterpret_cast<uint64_t>(base)+stack_size)
                throw std::runtime_error("Snapshot refused: foreign code or invalid host stack frame");
            DWORD64 image=0;auto entry=RtlLookupFunctionEntry(context.Rip,&image,nullptr);
            if(entry){PVOID data=nullptr;DWORD64 establisher=0;RtlVirtualUnwind(UNW_FLAG_NHANDLER,image,context.Rip,entry,&context,&data,&establisher,nullptr);}
            else{context.Rip=*reinterpret_cast<uint64_t*>(context.Rsp);context.Rsp+=8;}
        }
        if(!complete)throw std::runtime_error("Snapshot stack unwind exceeded bound");
    }
    ar.raw(reinterpret_cast<void*>(rsp),size);
}
