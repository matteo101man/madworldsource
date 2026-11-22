// Terminal host lifecycle, independent of original gameplay. One run per DLL.
#include "embed_host.h"
#include "native_config.h"
#include "ppc_memory.h"
#include "rso.h"
#include "mmio.h"
#include "host_fiber.h"
#include "snapshot.h"
#include <atomic>
#include <cstdio>
#include <exception>
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
namespace {
std::atomic<bool> started{false},stop_requested{false};
HostFiber* supervisor=nullptr;DWORD owner=0;int result=0;
HostFiber guest;
HostFiber* snapshot_target=nullptr;
int snapshot_action=0;
char error_message[1024]{};int(*boot_entry)()=nullptr;
[[noreturn]] void park(int code){
    result=code;host_fiber_switch(*supervisor);
    std::abort(); // A terminally parked guest fiber must never resume.
}
void WINAPI boot(void*){
    try{result=boot_entry();}
    catch(const std::exception& error){std::snprintf(error_message,sizeof error_message,"%s",error.what());result=-1;}
    catch(...){std::snprintf(error_message,sizeof error_message,"Unhandled native core exception");result=-1;}
    park(result);
}
}
bool native_host_embedded(){return supervisor&&GetCurrentThreadId()==owner;}
bool native_host_started(){return started.load(std::memory_order_acquire);}
void native_host_snapshot_park(bool load){
    if(!native_host_embedded())throw std::runtime_error("Snapshot requires embedded host");
    snapshot_target=host_fiber_current();snapshot_action=load?2:1;
    host_fiber_switch(*supervisor);
}
void native_host_poll(){if(native_host_embedded()&&stop_requested.load(std::memory_order_acquire))park(0);}
void native_host_fail(const char* message){
    if(native_host_embedded()){
        std::snprintf(error_message,sizeof error_message,"%s",message);park(-1);
    }
}
extern "C" __declspec(dllexport) uint32_t mw_native_core_abi(){return 1;}
extern "C" __declspec(dllexport) void mw_native_request_stop(){stop_requested.store(true,std::memory_order_release);}
// Read only after mw_native_run returns on its owner thread.
extern "C" __declspec(dllexport) const char* mw_native_last_error(){return error_message;}
int native_host_run(int(*entry)()){
    if(started.exchange(true,std::memory_order_acq_rel))return -2;
    native_snapshot_start_clock();
    native_config_freeze();
    if(IsThreadAFiber()){std::snprintf(error_message,sizeof error_message,"Core owner thread is already a fiber");return -3;}
    owner=GetCurrentThreadId();supervisor=host_fiber_current();
    boot_entry=entry;
    if(!host_fiber_create(guest,boot,nullptr)){supervisor=nullptr;owner=0;return -5;}
    host_fiber_switch(guest);
    while(snapshot_action){
        bool load=snapshot_action==2;snapshot_action=0;
        try{native_snapshot_process(load);}
        catch(const std::exception& error){std::snprintf(error_message,sizeof error_message,"%s",error.what());std::fprintf(stderr,"[snapshot] rejected: %s\n",error.what());result=-1;break;}
        HostFiber* resume=load?os_threads_resume():snapshot_target;
        if(!resume){result=-1;break;}
        host_fiber_switch(*resume);
    }
    // All guest stacks are parked. Destroy sinks before their borrowed RAM.
    os_clear_hardware_events();ios_shutdown_devices();
    native_platform_shutdown();
    os_threads_shutdown(&guest);
    host_fiber_destroy(guest);
    ppc_reset_debug_context();mw_rso_attach(nullptr);g_mem=nullptr;
    ppc_memory_shutdown();
    supervisor=nullptr;owner=0;
    return result;
}
