#include "native_config.h"
#include "ios.h"
#include "mmio.h"
#include "os.h"
#include "platform.h"
#include "input_file.h"
#include <array>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <vector>
#include <map>

namespace {
using Create=void*(*)();using Destroy=void(*)(void*);
using Ioctl=int32_t(*)(void*,void*,uint32_t,uint32_t,uint32_t,uint32_t);
using Input=int32_t(*)(void*,const uint8_t*);
struct Pending {uint32_t request,cmd,ni,no,vectors;};
struct State;
std::map<uint32_t, std::weak_ptr<State>> states;
uint32_t next_state=1;
struct State {
    uint32_t id=0;
    void* bt;Destroy destroy;Ioctl ioctl;Input input;
    std::vector<Pending> pending;
    ~State(){destroy(bt);}
    int32_t execute(const Pending& p){
        int32_t rc=ioctl(native_platform_context(),bt,p.cmd,p.ni,p.no,p.vectors);
        if(rc==-1000)ppc_fatal("Rust Bluetooth protocol failure");return rc;
    }
    void poll(){
        std::array<uint8_t,8> packet{0,0,0,128,128,0,0,0};
        if(const char* path=native_config_get("MW_INPUT_FILE")) {
            read_native_input(path,packet);
        }
        if(input(bt,packet.data())!=0)ppc_fatal("Rust virtual controller input failed");
        for(size_t i=0;i<pending.size();) {
            int32_t result=execute(pending[i]);
            if(!result){++i;continue;}
            uint32_t request=pending[i].request;pending.erase(pending.begin()+i);ios_reply(request,result);
        }
    }
};
void schedule(const std::shared_ptr<State>& state){
    os_schedule(ppc_timebase()+303750,{OSEventKind::Bluetooth,state->id});
}
void poll_event(const OSEventData& event){
    auto found=states.find(event.a);
    if(found==states.end())return;
    if(auto live=found->second.lock()){live->poll();schedule(live);}
    else states.erase(found);
}
class PlatformBluetooth:public IOSDevice {
    std::shared_ptr<State> state;
public:
    PlatformBluetooth(){
        auto symbol=[](const char* name){void* p=native_platform_symbol(name);if(!p)ppc_fatal("missing virtual controller ABI %s",name);return p;};
        os_register_event(OSEventKind::Bluetooth,poll_event);
        state=std::make_shared<State>();
        state->id=next_state++;states[state->id]=state;
        state->destroy=reinterpret_cast<Destroy>(symbol("mw_bt_destroy"));
        state->ioctl=reinterpret_cast<Ioctl>(symbol("mw_bt_ioctlv"));
        state->input=reinterpret_cast<Input>(symbol("mw_bt_tick"));
        state->bt=reinterpret_cast<Create>(symbol("mw_bt_create"))();
        if(!state->bt)ppc_fatal("virtual controller creation failed");
        schedule(state);std::fprintf(stderr,"[input] Rust virtual controller active; explicit MW_INPUT_FILE packets\n");
    }
    void snapshot_state(StateArchive& ar) override {
        if(ar.reading)states.erase(state->id);
        ar.pod(state->id);if(!state->id)throw std::runtime_error("Snapshot Bluetooth id");
        states[state->id]=state;ar.vector(state->pending,4);
        native_platform_blob(ar,state->bt,"mw_bt_state_save","mw_bt_state_load");
    }
    int32_t ioctlv(const IOSRequest& r,uint32_t cmd,const IOSVector* in,uint32_t ni,const IOSVector* io,uint32_t no) override {
        static unsigned traced=0;
        if(NATIVE_OPTION("MW_TRACE_INPUT")&&ni==2&&no==1&&traced++<100)std::fprintf(stderr,"[input] USB=%u ep=%02X length-bytes=%02X%02X out=%u\n",cmd,g_mem[in[0].addr],g_mem[in[1].addr],g_mem[in[1].addr+1],io[0].len);
        Pending p{r.addr,cmd,ni,no,r.arg[3]};int32_t result=state->execute(p);
        bool receive=(cmd==2&&ni==2)||(cmd==1&&ni==2&&g_mem[in[0].addr]==0x82);
        if(receive&&result==0){
            if(state->pending.size()>=4)return IPC_EINVAL;
            state->pending.push_back(p);return IPC_PENDING;
        }
        return result;
    }
};
}
std::unique_ptr<IOSDevice> create_platform_bluetooth(){
    if(!native_platform_context())return nullptr;
    return std::make_unique<PlatformBluetooth>();
}

void native_bluetooth_state(StateArchive& ar){ar.pod(next_state);}
