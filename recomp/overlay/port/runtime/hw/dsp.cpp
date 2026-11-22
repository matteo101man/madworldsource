#include "native_config.h"
// Optional Rust DSP interpreter bridge. Gameplay stays in the native CPU core.
// Gecko-derived platform library is GPL-3.0; see native-platform/README.md.
#include "mmio.h"
#include "os.h"
#include "platform.h"
#include "embed_host.h"
#include "rso.h"
#include <algorithm>
#include <cstdlib>
#include <cstdio>
#include <chrono>
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
namespace {
void* dsp=nullptr;
// Options are configured before startup. Avoid CRT environment lookup on every
// DSP mailbox access; this changes diagnostics lookup only, never MMIO ordering.
bool trace_dsp=false;
HMODULE platform_library=nullptr;
using PlatformResolver=void*(*)(const char*);
PlatformResolver platform_resolver=nullptr;
using Read=int32_t(*)(void*,uint32_t,uint32_t,uint32_t*);
using Write=int32_t(*)(void*,uint32_t,uint32_t,uint32_t);
using Tick=int32_t(*)(void*,uint64_t);
using Irq=uint32_t(*)(void*);
Read read_dsp;Write write_dsp;Tick tick_dsp;Irq irq_dsp;
// Optional: next timebase tick at which a poll observes a guest-visible interrupt.
using PollHint=uint64_t(*)(void*);
PollHint poll_hint=nullptr;
using GxCommand=int32_t(*)(void*,const uint8_t*,size_t);
GxCommand gx_command;
using ProfileClock=std::chrono::steady_clock;
struct IoProfile {bool started=false;ProfileClock::time_point start{};uint64_t gx_calls=0,gx_bytes=0,gx_ns=0,sync_calls=0,sync_ns=0;} io_profile;
bool profile_enabled(){static const bool enabled=[](){const char* value=native_config_get("MW_PROFILE_NATIVE_IO");return value&&value[0]=='1'&&value[1]==0;}();return enabled;}
uint64_t profile_ns(ProfileClock::time_point start){return uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(ProfileClock::now()-start).count());}
void write_native_code_map(){
    const char* path=native_config_get("MW_PROFILE_CODE_MAP");if(!path||!*path)return;
    FILE* file=std::fopen(path,"wb");if(!file)ppc_fatal("Cannot create native profiling map");
    std::fprintf(file,"{\"profileMap\":1,\"pid\":%lu,\"tid\":%lu}\n",GetCurrentProcessId(),GetCurrentThreadId());
    size_t count=0;
    auto emit=[&](const PPCFuncEntry* functions,size_t length){
        for(size_t i=0;i<length;++i){
            DWORD64 native=reinterpret_cast<DWORD64>(functions[i].fn),base=0;
            auto unwind=RtlLookupFunctionEntry(native,&base,nullptr);if(!unwind)continue;
            std::fprintf(file,"{\"canonical\":%u,\"begin\":%llu,\"end\":%llu}\n",functions[i].addr,base+unwind->BeginAddress,base+unwind->EndAddress);++count;
        }
    };
    emit(g_ppc_funcs,g_ppc_func_count);emit(g_rso_funcs,g_rso_func_count);
    std::fclose(file);std::fprintf(stderr,"NATIVE_CODE_MAP {\"pid\":%lu,\"tid\":%lu,\"entries\":%zu}\n",GetCurrentProcessId(),GetCurrentThreadId(),count);
}
void sync(){
    const bool profile=profile_enabled();const auto began=profile?ProfileClock::now():ProfileClock::time_point{};
    if(tick_dsp(dsp,ppc_timebase())!=0)ppc_fatal("Rust DSP interpreter failed");
    pi_set_cause(PI_INT_DSP,irq_dsp(dsp)!=0);
    if(profile){++io_profile.sync_calls;io_profile.sync_ns+=profile_ns(began);}
}
void update(const OSEventData&){
    sync();
    // Default cadence 0.1 ms. With the AX HLE the platform reports when its next
    // interrupt (AI/ARAM DMA completion) is due; poll exactly then, bounded to
    // [0.1 ms, 1 ms]. Register accesses still synchronise on demand.
    const uint64_t now=ppc_timebase();
    uint64_t next=now+6075;
    if(poll_hint){const uint64_t hint=poll_hint(dsp);if(hint)next=std::clamp(hint,now+6075,now+60750);}
    os_schedule(next,{OSEventKind::Dsp});
}
}
void* native_platform_context(){return dsp;}
extern "C" __declspec(dllexport) int mw_native_bind_platform(PlatformResolver resolver){
    if(native_host_started())return -1;platform_resolver=resolver;return 0;
}
void* native_platform_symbol(const char* name){return platform_resolver?platform_resolver(name):platform_library?reinterpret_cast<void*>(GetProcAddress(platform_library,name)):nullptr;}
void native_platform_shutdown(){
    if(dsp){
        auto destroy=reinterpret_cast<void(*)(void*)>(native_platform_symbol("mw_dsp_destroy"));
        if(destroy)destroy(dsp);dsp=nullptr;
    }
    if(platform_library){FreeLibrary(platform_library);platform_library=nullptr;}
    g_native_dirty_counter=nullptr;g_native_dirty_pages=nullptr;
    if(profile_enabled()&&io_profile.started){
        std::fprintf(stderr,"NATIVE_IO_PROFILE {\"elapsedNs\":%llu,\"gxCalls\":%llu,\"gxBytes\":%llu,\"gxNs\":%llu,\"syncCalls\":%llu,\"syncNs\":%llu}\n",profile_ns(io_profile.start),io_profile.gx_calls,io_profile.gx_bytes,io_profile.gx_ns,io_profile.sync_calls,io_profile.sync_ns);
        io_profile.started=false;
    }
}
void native_platform_gx_scene(uint32_t scene){
    if(!dsp)return;
    auto mark=reinterpret_cast<int32_t(*)(void*,uint32_t)>(native_platform_symbol("mw_gx_scene"));
    if(!mark||mark(dsp,scene))ppc_fatal("Rust GX scene marker failed");
}
void native_platform_vi_write(uint32_t address,uint32_t value,int size){
    if(!dsp)return;
    sync();
    auto write=reinterpret_cast<int32_t(*)(void*,uint32_t,uint32_t,uint32_t)>(native_platform_symbol("mw_vi_write"));
    if(!write||write(dsp,address,size,value)<0)ppc_fatal("Rust VI presentation synchronization failed");
}
void native_platform_ai_control(uint32_t value){
    if(!dsp)return;
    auto write=reinterpret_cast<int32_t(*)(void*,uint32_t)>(native_platform_symbol("mw_ai_control"));
    if(!write||write(dsp,value))ppc_fatal("Rust AID control synchronization failed");
}
// A display-list call the FIFO verified as draw-only and unchanged (MW_GX_DL_PASSTHROUGH);
// the decoder reads the list from guest RAM itself. Every other 0x40 is flattened by the FIFO.
void native_platform_gx_list(const uint8_t call[9]){
    if(!dsp)return;
    int32_t rc=gx_command(dsp,call,9);
    if(rc)ppc_fatal("Rust GX display-list call failed %d",rc);
}
void native_platform_gx(const uint8_t* bytes,size_t size){
    if(!dsp||!size||bytes[0]==0x40)return;
    const bool profile=profile_enabled();const auto began=profile?ProfileClock::now():ProfileClock::time_point{};
    int32_t rc=gx_command(dsp,bytes,size);
    if(profile){++io_profile.gx_calls;io_profile.gx_bytes+=size;io_profile.gx_ns+=profile_ns(began);}
    if(rc)ppc_fatal("Rust GX decode opcode %02X length %zu failed %d",bytes[0],size,rc);
}
MMIO_DEVICE(dsp) {
    os_register_event(OSEventKind::Dsp, update);
    trace_dsp=native_config_get("MW_TRACE_DSP")!=nullptr;
    const char* path=native_config_get("MW_PLATFORM_DLL");if(!path&&!platform_resolver)return;
    if(!platform_resolver){
        platform_library=LoadLibraryA(path);
        if(!platform_library)ppc_fatal("cannot load MW_PLATFORM_DLL (error %lu)",GetLastError());
    }
    auto find=[](const char* name){auto p=native_platform_symbol(name);if(!p)ppc_fatal("missing DSP ABI symbol %s",name);return p;};
    auto abi=reinterpret_cast<uint32_t(*)()>(find("mw_platform_abi"));
    if(abi()!=1)ppc_fatal("Rust platform ABI mismatch");
    auto create=reinterpret_cast<void*(*)(uint8_t*,uint8_t*)>(find("mw_dsp_create"));
    read_dsp=reinterpret_cast<Read>(find("mw_dsp_read"));write_dsp=reinterpret_cast<Write>(find("mw_dsp_write"));
    tick_dsp=reinterpret_cast<Tick>(find("mw_dsp_tick"));irq_dsp=reinterpret_cast<Irq>(find("mw_dsp_irq"));
    gx_command=reinterpret_cast<GxCommand>(find("mw_gx_command"));
    poll_hint=reinterpret_cast<PollHint>(native_platform_symbol("mw_dsp_poll_hint"));
    dsp=create(g_mem+0x80000000u,g_mem+0x90000000u);
    if(!dsp)ppc_fatal("Rust DSP construction failed");
    write_native_code_map();
    if(profile_enabled()){io_profile=IoProfile{};io_profile.started=true;io_profile.start=ProfileClock::now();}
    static uint64_t generation=0;static uint64_t pages[NATIVE_DIRTY_PAGES]{};
    g_native_dirty_counter=&generation;g_native_dirty_pages=pages;
    auto tracking=reinterpret_cast<int32_t(*)(void*,uint64_t*,uint64_t*,size_t)>(find("mw_memory_tracking"));
    if(tracking(dsp,&generation,pages,NATIVE_DIRTY_PAGES))ppc_fatal("Rust native memory tracking failed");
    mmio_register(0xcc005000,0x40,[](uint32_t a,int n){
        sync();uint32_t result=0;int32_t rc=read_dsp(dsp,a,n,&result);
        if(rc)ppc_fatal("DSP read%u at %08X failed %d",n*8,a,rc);
        if(trace_dsp&&(a&255)==6)std::fprintf(stderr,"[DSP] read mail-lo %04x\n",result);
        pi_set_cause(PI_INT_DSP,irq_dsp(dsp)!=0);return result;
    },[](uint32_t a,uint32_t v,int n){
        sync();int32_t rc=write_dsp(dsp,a,n,v);
        if(trace_dsp&&((a&255)<4||(a&255)==10))std::fprintf(stderr,"[DSP] write %08x=%04x\n",a,v);
        if(rc)ppc_fatal("DSP write%u at %08X failed %d",n*8,a,rc);
        pi_set_cause(PI_INT_DSP,irq_dsp(dsp)!=0);
    });
    std::fprintf(stderr,"[DSP] Rust interpreter attached to native MEM1/MEM2; optional MW_AUDIO_WAV captures original AID PCM\n");
    os_schedule(ppc_timebase()+6075,{OSEventKind::Dsp});
}

#include "snapshot.h"
void native_platform_blob(StateArchive& ar,void* handle,const char* save_name,const char* load_name){
    using Save=int32_t(*)(void*,uint8_t**,size_t*);using Load=int32_t(*)(void*,const uint8_t*,size_t);using Free=void(*)(uint8_t*,size_t);
    std::vector<uint8_t> bytes;
    if(!ar.reading){
        auto save=reinterpret_cast<Save>(native_platform_symbol(save_name));auto free=reinterpret_cast<Free>(native_platform_symbol("mw_state_free"));
        uint8_t* data=nullptr;size_t length=0;
        if(!save||!free||save(handle,&data,&length)||!data||length>256u*1024*1024)throw std::runtime_error("Native platform snapshot save failed");
        try{bytes.assign(data,data+length);}catch(...){free(data,length);throw;}free(data,length);
    }
    ar.vector(bytes,256u*1024*1024);
    if(ar.reading){auto load=reinterpret_cast<Load>(native_platform_symbol(load_name));if(!load||load(handle,bytes.data(),bytes.size()))throw std::runtime_error("Native platform snapshot load failed");}
}
void native_platform_state(StateArchive& ar){
    if(!dsp)throw std::runtime_error("Snapshot requires the native hardware platform");
    native_platform_blob(ar,dsp,"mw_platform_state_save","mw_platform_state_load");
    if(ar.reading){native_memory_written(0x80000000,24u<<20);native_memory_written(0x90000000,64u<<20);}
}
