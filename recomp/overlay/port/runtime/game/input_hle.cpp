#include "native_config.h"
#include "hle.h"
#include "hle_funcs.h"
#include "os.h"
#include "platform.h"
#include "snapshot.h"
#include "../ios/input_file.h"
#include <array>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <deque>
// PC adaptation: the game's WPAD/KPAD API boundary is answered by host code, so the
// Wii remote Bluetooth stack (WUD/BTE, IOS /dev/usb/oh1/57e/305, HID) never starts.
// research/INPUT-HLE.md lists every hooked function and its evidence. The only read
// is cPadSystem_updateChannel 8020EFC0 -> KPADReadEx, which uses status[0] alone.
// MW_INPUT_HLE=legacy runs the original SDK and Bluetooth path instead;
// MW_INPUT_HLE=compare runs legacy and logs where the HLE KPADStatus would differ.
bool native_frame_replay_pad(uint32_t& hold,float* acc,float* fs,float* stick);
void native_frame_record_pad(uint32_t hold,const float* acc,const float* fs,const float* stick);
namespace {
enum class Mode {Hle,Legacy,Compare};
Mode mode(){
    static const Mode value=[]{const char* v=native_config_get("MW_INPUT_HLE");
        if(!v)return Mode::Hle;if(!std::strcmp(v,"legacy"))return Mode::Legacy;if(!std::strcmp(v,"compare"))return Mode::Compare;
        ppc_fatal("MW_INPUT_HLE must be legacy or compare");return Mode::Hle;}();
    return value;
}
bool hle(){return mode()==Mode::Hle;}
bool speaker_trace(){static const bool on=[]{const char* v=native_config_get("MW_TRACE_REMOTE_SPEAKER");return v&&!std::strcmp(v,"1");}();return on;}
struct Sample {uint32_t hold;float acc[3],fs_acc[3],stick[2];};
using Create=void*(*)();using Destroy=void(*)(void*);
using Read=int32_t(*)(void*,const uint8_t*,uint64_t,Sample*);
using Control=int32_t(*)(void*,uint32_t,uint32_t);using On=int32_t(*)(void*,uint32_t);using Prompt=int32_t(*)(void*,uint32_t);
using Stream=int32_t(*)(void*,void*,uint32_t,const uint8_t*,uint32_t);
using Motor=int32_t(*)(void*,uint32_t,uint32_t);
struct Host {
    void* pad=nullptr;Read read=nullptr;Control control=nullptr;On speaker_on=nullptr;Prompt prompt=nullptr;Stream stream=nullptr;
    Motor motor=nullptr;// optional: gamepad rumble in the PC frontend
    Host(){
        auto symbol=[](const char* name){void* p=native_platform_symbol(name);if(!p)ppc_fatal("missing input HLE ABI %s",name);return p;};
        read=reinterpret_cast<Read>(symbol("mw_pad_sample"));
        control=reinterpret_cast<Control>(symbol("mw_pad_speaker_control"));
        speaker_on=reinterpret_cast<On>(symbol("mw_pad_speaker_on"));
        prompt=reinterpret_cast<Prompt>(symbol("mw_pad_set_prompt"));
        stream=reinterpret_cast<Stream>(symbol("mw_pad_speaker_stream"));
        motor=reinterpret_cast<Motor>(native_platform_symbol("mw_pad_motor"));
        pad=reinterpret_cast<Create>(symbol("mw_pad_create"))();
        if(!pad)ppc_fatal("input HLE creation failed");
    }
};
Host& host(){static Host h;return h;}
struct Callback {uint32_t fn,chan;int32_t result;};
// Guest-visible state only; the gesture clock and speaker codec live in Rust.
struct State {
    uint32_t connect_cb[4]{},settle[4]{};uint8_t notified[4]{},data_format[4]{},dpd[4]{},motor[4]{};
    uint32_t prev_hold[4]{};float prev_acc[4]{},prev_fs_acc[4]{};
    std::deque<Callback> pending;
} g;
constexpr int32_t WPAD_ERR_NONE=0,WPAD_ERR_NO_CONTROLLER=-1;
// Only one remote with a Nunchuk exists, as with the former virtual controller.
bool connected(uint32_t chan){return chan==0;}
void queue(uint32_t fn,uint32_t chan,int32_t result){if(fn&&g.pending.size()<64)g.pending.push_back({fn,chan,result});}
// Original WPAD callbacks run later, from the Bluetooth stack with interrupts off;
// deliver them at the next KPADReadEx / WPADIsSpeakerEnabled, never inside the
// call that requested them, so callers have stored their own state first.
void deliver(PPCContext& c,uint8_t* m){
    while(!g.pending.empty()){
        Callback cb=g.pending.front();g.pending.pop_front();
        PPCContext saved=c;
        c.msr&=~MSR_EE;c.r[3]=cb.chan;c.r[4]=uint32_t(cb.result);
        ppc_call_indirect(c,m,cb.fn);
        c=saved;
    }
}
// The remote "connects" once connect callbacks have stopped changing, as the Bluetooth
// pairing did: the sound library (804F9B20) replaces cPadSystem's callback with its own
// 804F8000, which is the one that switches the remote speaker on, and only it sees connect.
constexpr uint32_t kConnectSettleReads=30;
void connect_events(uint32_t chan){
    if(chan<4&&connected(chan)&&g.connect_cb[chan]&&!g.notified[chan]&&++g.settle[chan]>=kConnectSettleReads){g.notified[chan]=1;queue(g.connect_cb[chan],chan,WPAD_ERR_NONE);}
}
std::array<uint8_t,8> packet(){
    std::array<uint8_t,8> p{0,0,0,128,128,0,0,0};
    if(const char* path=NATIVE_OPTION("MW_INPUT_FILE"))read_native_input(path,p);
    return p;
}
float norm(const float* v){return std::sqrt(v[0]*v[0]+v[1]*v[1]+v[2]*v[2]);}
// KPADStatus (0xB0) as written by KPADiRead / reset 8036B0EC. DPD is never valid
// (no IR, as before), so pointer fields keep KPAD's reset values.
void write_status(uint8_t* m,uint32_t at,uint32_t chan,const Sample& s){
    for(uint32_t i=0;i<0xB0;i+=4)st32(m,at+i,0);
    uint32_t prev=g.prev_hold[chan];
    st32(m,at+0x00,s.hold);st32(m,at+0x04,s.hold&~prev);st32(m,at+0x08,prev&~s.hold);
    for(int i=0;i<3;i++)stf32(m,at+0x0C+4*i,s.acc[i]);
    float value=norm(s.acc);stf32(m,at+0x18,value);stf32(m,at+0x1C,value-g.prev_acc[chan]);
    stf32(m,at+0x34,1.f);stf32(m,at+0x54,1.f);// horizon (1,0), acc_vertical (1,0)
    st8(m,at+0x5C,1);st8(m,at+0x5D,0);st8(m,at+0x5E,0);st8(m,at+0x5F,g.data_format[chan]);// WPAD_DEV_FREESTYLE
    stf32(m,at+0x60,s.stick[0]);stf32(m,at+0x64,s.stick[1]);
    for(int i=0;i<3;i++)stf32(m,at+0x68+4*i,s.fs_acc[i]);
    float fs=norm(s.fs_acc);stf32(m,at+0x74,fs);stf32(m,at+0x78,fs-g.prev_fs_acc[chan]);
    g.prev_hold[chan]=s.hold;g.prev_acc[chan]=value;g.prev_fs_acc[chan]=fs;
}
bool guest_mapped(uint32_t p,uint32_t n){return (p>=0x80000000u&&p<=0x81800000u-n)||(p>=0x90000000u&&p<=0x94000000u-n);}
// The remote prompt the game itself shows (cPlayer +0x11C8 cockpit, remote child +0x24;
// active byte +0, kind +0x18 as stored by setSwingNavigation's fn_802278D8). Read-only.
uint32_t remote_prompt(uint8_t* m){
    uint32_t player=native_snapshot_player();
    if(!guest_mapped(player,0x11CC))return 0;
    uint32_t cockpit=ld32(m,player+0x11C8);if(!guest_mapped(cockpit,0x2C))return 0;
    uint32_t child=ld32(m,cockpit+0x24);if(!guest_mapped(child,0x1C))return 0;
    return ld8(m,child)?ld8(m,child+0x18):0;
}
bool sample(uint32_t chan,Sample& s,uint8_t* m){
    host().prompt(host().pad,remote_prompt(m));
    auto p=packet();
    int32_t rc=host().read(host().pad,p.data(),ppc_timebase(),&s);
    if(rc==-1000)ppc_fatal("Rust input HLE failure");
    // MW_INPUT_REPLAY / MW_FRAME_RECORD (game/frame_record.cpp, default off): recorded pad in, samples out.
    if(native_frame_replay_pad(s.hold,s.acc,s.fs_acc,s.stick))rc=0;
    if(const char* choice=NATIVE_OPTION("MW_TEST_CHALLENGE_CHOICE")){
        bool yes=!std::strcmp(choice,"accept"),no=!std::strcmp(choice,"decline");
        if(!yes&&!no)ppc_fatal("MW_TEST_CHALLENGE_CHOICE requires accept/decline");
        uint32_t owner=ld32(m,0x8069e2c0),object=guest_mapped(owner,0x40)?ld32(m,owner+0x3c):0;
        if(rc==0&&guest_mapped(object,0x30)&&ld8(m,object)){
            s.hold=0;static uint64_t selected=0;
            if(ld32(m,object+0x24)==1){
                if(!selected)selected=ppc_timebase();uint64_t age=ppc_timebase()-selected;
                if(yes&&age>60750000/2)s.hold=0x800;
                if(no&&age>60750000/2&&age<60750000*3/4)s.hold=0x400;
                if(no&&age>60750000)s.hold=0x800;
            }
        }
    }
    // Explicit own-process cutscene-skip fixture: supply one WPAD B packet
    // through the original KPAD/cPad path. Never write event/game state.
    if(const char* requested=NATIVE_OPTION("MW_TEST_EVENT_SKIP_FRAME")){
        char* end=nullptr;unsigned long frame=std::strtoul(requested,&end,0);
        if(!end||*end||frame<1||frame>1078)ppc_fatal("MW_TEST_EVENT_SKIP_FRAME requires1..1078");
        const uint32_t event=0x80606cf8;
        if(rc==0&&ld32(m,event+4)==5&&ld32(m,event+0x114)==frame){
            s.hold|=0x400;static bool logged=false;
            if(!logged){logged=true;std::fprintf(stderr,"[event-skip-fixture] original WPAD B packet at event frame%lu\n",frame);}
        }
    }
    if(rc==0)native_frame_record_pad(s.hold,s.acc,s.fs_acc,s.stick);
    return rc==0;
}
void compare(uint8_t* m,uint32_t chan,uint32_t status,int32_t count){
    static unsigned lines=0;static uint8_t last[0xB0];static uint32_t last_mask=~0u;
    if(chan!=0||count<1||lines>=400)return;
    Sample s{};if(!sample(chan,s,m))return;
    uint8_t legacy[0xB0];std::memcpy(legacy,m+status,0xB0);
    State keep=g;write_status(m,status,chan,s);
    uint8_t mine[0xB0];std::memcpy(mine,m+status,0xB0);for(uint32_t i=0;i<0xB0;i++)st8(m,status+i,legacy[i]);
    g.prev_hold[chan]=keep.prev_hold[chan];g.prev_acc[chan]=keep.prev_acc[chan];g.prev_fs_acc[chan]=keep.prev_fs_acc[chan];
    auto word=[](const uint8_t* b,uint32_t o){return uint32_t(b[o])<<24|uint32_t(b[o+1])<<16|uint32_t(b[o+2])<<8|b[o+3];};
    auto real=[&](const uint8_t* b,uint32_t o){uint32_t w=word(b,o);float f;std::memcpy(&f,&w,4);return f;};
    uint32_t mask=0;char text[2048];int n=0;
    for(uint32_t o=0;o<0xB0;o+=4){
        bool bytes=o==0x5C,ints=o<0x0C;
        bool differ=bytes?std::memcmp(legacy+o,mine+o,4)!=0:ints?word(legacy,o)!=word(mine,o):std::fabs(real(legacy,o)-real(mine,o))>0.05f;
        if(!differ)continue;mask|=1u<<(o/4%32);
        if(n<1900)n+=bytes||ints?std::snprintf(text+n,sizeof text-n," +%02X:%08x/%08x",o,word(legacy,o),word(mine,o))
                                :std::snprintf(text+n,sizeof text-n," +%02X:%.3f/%.3f",o,real(legacy,o),real(mine,o));
    }
    if(mask==last_mask&&!std::memcmp(last,legacy,0x0C))return;
    last_mask=mask;std::memcpy(last,legacy,sizeof last);lines++;
    std::fprintf(stderr,"[input-compare] count=%d legacy/hle%s\n",count,n?text:" equal");
}
}
// KPADInit 8036E260 (sole caller of KPADiInit -> WPADInit -> WUDInit).
PPC_FUNC(f_8036E260){
    if(!hle()){f_8036E260_recomp(c,m);if(mode()==Mode::Compare)host();return;}
    host();std::fprintf(stderr,"[input] HLE controller active: no Bluetooth stack; explicit packets\n");
}
// WPADRegisterAllocator 8032B1D8 (thunk to WUD).
PPC_FUNC(f_8032B1D8){if(!hle()){f_8032B1D8_recomp(c,m);return;}}
// WPADGetStatus 8032B1DC: cPadSystem init spins until WPAD_STATE_SETUP (3).
PPC_FUNC(f_8032B1DC){if(!hle()){f_8032B1DC_recomp(c,m);return;}c.r[3]=3;}
// KPADSet{Pos,Hori,Dist,Acc}Param: the game passes (0.1,1)/(0,1): no smoothing.
PPC_FUNC(f_8036B07C){if(!hle())f_8036B07C_recomp(c,m);}
PPC_FUNC(f_8036B098){if(!hle())f_8036B098_recomp(c,m);}
PPC_FUNC(f_8036B0B4){if(!hle())f_8036B0B4_recomp(c,m);}
PPC_FUNC(f_8036B0D0){if(!hle())f_8036B0D0_recomp(c,m);}
// KPADSetConnectCallback 8036E820(chan, fn).
PPC_FUNC(f_8036E820){
    if(!hle()){f_8036E820_recomp(c,m);return;}
    uint32_t chan=c.r[3]&3;g.connect_cb[chan]=c.r[4];g.settle[chan]=0;
}
// KPADDisableDPD 8036E888 / KPADEnableDPD 8036E8A4 (chan).
PPC_FUNC(f_8036E888){if(!hle()){f_8036E888_recomp(c,m);return;}g.dpd[c.r[3]&3]=0;}
PPC_FUNC(f_8036E8A4){if(!hle()){f_8036E8A4_recomp(c,m);return;}g.dpd[c.r[3]&3]=1;}
// s32 KPADReadEx(chan, KPADStatus* buf, u32 length, s32* err).
PPC_FUNC(f_8036D950){
    if(!hle()){
        uint32_t chan=c.r[3],buf=c.r[4];f_8036D950_recomp(c,m);
        if(mode()==Mode::Compare)compare(m,chan,buf,int32_t(c.r[3]));
        return;
    }
    uint32_t chan=c.r[3],buf=c.r[4],length=c.r[5],err=c.r[6];
    connect_events(chan);deliver(c,m);
    Sample s{};
    bool live=chan<4&&connected(chan)&&length>0&&sample(chan,s,m);
    if(live)write_status(m,buf,chan,s);
    if(err)st32(m,err,uint32_t(live?WPAD_ERR_NONE:WPAD_ERR_NO_CONTROLLER));
    c.r[3]=live?1:0;
}
// s32 WPADProbe(chan, u32* type): 0 with type WPAD_DEV_FREESTYLE, or no controller.
PPC_FUNC(f_8032C564){
    if(!hle()){f_8032C564_recomp(c,m);return;}
    bool live=c.r[3]<4&&connected(c.r[3]);
    if(c.r[4])st32(m,c.r[4],live?1:253);
    c.r[3]=uint32_t(live?WPAD_ERR_NONE:WPAD_ERR_NO_CONTROLLER);
}
// s32 WPADSetDataFormat(chan, fmt).
PPC_FUNC(f_8032C7A0){
    if(!hle()){f_8032C7A0_recomp(c,m);return;}
    if(c.r[3]>=4||!connected(c.r[3])){c.r[3]=uint32_t(WPAD_ERR_NO_CONTROLLER);return;}
    g.data_format[c.r[3]]=uint8_t(c.r[4]);c.r[3]=0;
}
// void WPADDisconnect(chan): honoured for the absent channels; the one remote stays.
PPC_FUNC(f_8032C418){if(!hle()){f_8032C418_recomp(c,m);return;}}
// u8 WPADGetRadioSensitivity(chan): 0 means "no signal" to the game.
PPC_FUNC(f_8032B1E0){if(!hle()){f_8032B1E0_recomp(c,m);return;}c.r[3]=c.r[3]<4&&connected(c.r[3])?100:0;}
// s32 WPADGetInfoAsync(chan, WPADInfo* info, cb): full battery, Nunchuk attached.
PPC_FUNC(f_8032C8AC){
    if(!hle()){f_8032C8AC_recomp(c,m);return;}
    uint32_t chan=c.r[3],info=c.r[4],cb=c.r[5];
    if(chan>=4||!connected(chan)){c.r[3]=uint32_t(WPAD_ERR_NO_CONTROLLER);return;}
    if(info){
        for(uint32_t i=0;i<24;i+=4)st32(m,info+i,0);
        st32(m,info+0,g.dpd[chan]);st32(m,info+4,uint32_t(host().speaker_on(host().pad,chan)==1));st32(m,info+8,1);
        st8(m,info+20,4);st8(m,info+21,1);
    }
    queue(cb,chan,WPAD_ERR_NONE);c.r[3]=0;
}
// void WPADControlMotor(chan, on) / WPADEnableMotor(on) / WPADResetAutoSleepTimeCount(chan).
PPC_FUNC(f_8032CACC){
    if(!hle()){f_8032CACC_recomp(c,m);return;}
    if(c.r[3]<4){g.motor[c.r[3]]=uint8_t(c.r[4]!=0);if(host().motor)host().motor(host().pad,c.r[3],c.r[4]!=0);}
}
PPC_FUNC(f_8032C4D8){if(!hle())f_8032C4D8_recomp(c,m);}
PPC_FUNC(f_8032C50C){if(!hle())f_8032C50C_recomp(c,m);}
// BOOL WPADIsSpeakerEnabled(chan): polled by the AXRmt speaker alarm 804F7C74.
PPC_FUNC(f_8032D2EC){
    uint32_t chan=c.r[3];
    if(!hle())f_8032D2EC_recomp(c,m);
    else{deliver(c,m);c.r[3]=chan<4&&host().speaker_on(host().pad,chan)==1?1:0;}
    static uint32_t last[4]={9,9,9,9};
    if(speaker_trace()&&chan<4&&last[chan]!=c.r[3]){last[chan]=c.r[3];std::fprintf(stderr,"[speaker-api] IsSpeakerEnabled chan=%u -> %u lr=%08x\n",chan,c.r[3],c.lr);}
}
// s32 WPADControlSpeaker(chan, cmd, cb): 0 off, 1 on, 2 mute, 3 unmute, 4 play.
PPC_FUNC(f_8032D330){
    uint32_t chan=c.r[3],cmd=c.r[4],cb=c.r[5];
    if(speaker_trace())std::fprintf(stderr,"[speaker-api] ControlSpeaker chan=%u cmd=%u cb=%08x lr=%08x\n",chan,cmd,cb,c.lr);
    if(!hle()){f_8032D330_recomp(c,m);return;}
    if(chan>=4||!connected(chan)){c.r[3]=uint32_t(WPAD_ERR_NO_CONTROLLER);return;}
    if(host().control(host().pad,chan,cmd)==-1000)ppc_fatal("Rust speaker HLE failure");
    queue(cb,chan,WPAD_ERR_NONE);c.r[3]=0;
}
// BOOL WPADCanSendStreamData(chan) / s32 WPADSendStreamData(chan, data, len).
PPC_FUNC(f_8032EB08){
    if(!hle()){f_8032EB08_recomp(c,m);return;}
    c.r[3]=c.r[3]<4&&host().speaker_on(host().pad,c.r[3])==1?1:0;
}
PPC_FUNC(f_8032EB8C){
    if(!hle()){f_8032EB8C_recomp(c,m);return;}
    uint32_t chan=c.r[3],data=c.r[4],len=c.r[5];
    if(chan>=4||!connected(chan)||len>64){c.r[3]=uint32_t(WPAD_ERR_NO_CONTROLLER);return;}
    uint8_t bytes[64];for(uint32_t i=0;i<len;i++)bytes[i]=ld8(m,data+i);
    if(host().stream(native_platform_context(),host().pad,chan,bytes,len)==-1000)ppc_fatal("Rust speaker HLE failure");
    c.r[3]=0;
}
// Home Button Menu only (HOME is unbound on PC): sync, settings, DPD sensitivity.
#define MW_INPUT_STUB(addr) PPC_FUNC(f_##addr){if(!hle()){f_##addr##_recomp(c,m);return;}c.r[3]=0;}
MW_INPUT_STUB(8032B1CC) MW_INPUT_STUB(8032B1D0) MW_INPUT_STUB(8032B1D4)
MW_INPUT_STUB(8032CB98) MW_INPUT_STUB(8032CBCC) MW_INPUT_STUB(8032CDE8)
MW_INPUT_STUB(8032E94C) MW_INPUT_STUB(8032E980) MW_INPUT_STUB(80330918)
MW_INPUT_STUB(8032C678) MW_INPUT_STUB(8032C700)

void native_input_state(StateArchive& ar){
    ar.tag(0x4d57494e);
    ar.raw(g.connect_cb,sizeof g.connect_cb);ar.raw(g.settle,sizeof g.settle);ar.raw(g.notified,sizeof g.notified);ar.raw(g.data_format,sizeof g.data_format);
    ar.raw(g.dpd,sizeof g.dpd);ar.raw(g.motor,sizeof g.motor);
    ar.raw(g.prev_hold,sizeof g.prev_hold);ar.raw(g.prev_acc,sizeof g.prev_acc);ar.raw(g.prev_fs_acc,sizeof g.prev_fs_acc);
    std::vector<Callback> pending(g.pending.begin(),g.pending.end());ar.vector(pending,64);
    if(ar.reading)g.pending.assign(pending.begin(),pending.end());
    native_platform_blob(ar,host().pad,"mw_pad_state_save","mw_pad_state_load");
}
