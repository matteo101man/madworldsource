#include "native_config.h"
#include "hle.h"
#include "hle_funcs.h"
#include "platform.h"
#include "pc_death_cinematic.h"
#include <cmath>
#include <cstdio>
#include <cstring>
// PC adaptation (default off, MW_PC_FREE_CAMERA=1): mouse / right-stick orbit inside the
// game's own player follow camera. research/PC-CAMERA.md has the evidence.
// fn_802C9F3C(cPl0000*, Vec* center, f1 facing +0x74, f2) is the follow camera: yaw state
// +0x1AD4 chases facing + table offset +0x1AE0 (25..135 deg band, rate-limited), pitch
// +0x1AC4 lerps toward +0x1AD0; rot vector +0x1AC4/+0x1AC8 goes to MtxInitRotVec at
// 802CA618, then eye = dest + R*z*dist, Havok line checks, request to the camera manager.
// The hook replaces only the yaw that reaches that MtxInitRotVec (last yaw + user delta,
// within a camera area's own yaw limits) and adds a pitch offset there; everything after it
// (wall collision, camera manager, g_Camera) is original. Movement stays camera-relative
// because fn_802C6EEC derives +0x1134 from the shown g_Camera pos/dest next frame.
// Original camera: lock-on (+0x1254/+0x1251), LockCameraAngle (+0x1128 & 0x80), C recenter
// (+0x27C2 countdown), mode +0x11A == 2, designed shots whose yaw offset +0x1AE0 is not the
// behind-Jack pi (angle-table modes 3-8, 14, 15, 25, 26, 29-32, or such a camera area), and
// area/event cameras (fn_802C9A98 returns 0) or the instant-cut path fn_802CAB1C, which never
// reach that call, keep the original camera. Behind-Jack action modes (2, 18, ...) still orbit.
namespace {
bool enabled(){static const bool on=[]{const char* v=native_config_get("MW_PC_FREE_CAMERA");return v&&!std::strcmp(v,"1");}();return on;}
bool trace(){static const bool on=[]{const char* v=native_config_get("MW_PC_FREE_CAMERA_TRACE");return v&&!std::strcmp(v,"1");}();return on;}
// Rust host (native-platform pad.rs): user orbit since the last call, radians; flags bit0 auto-follow,
// bit1 orbit-while-locked (user toggle), bits 8..15 a signed lock-on target cycle (+1 = next target
// to the right on screen).
using Take=int32_t(*)(float*,float*,uint32_t*);
Take take(){
    static const Take fn=[]{void* p=native_platform_symbol("mw_pad_camera_take");if(!p)ppc_fatal("missing free-camera ABI mw_pad_camera_take");return reinterpret_cast<Take>(p);}();
    return fn;
}
constexpr uint32_t kRotCall=0x802CA61C;// return address of the main path's MtxInitRotVec
constexpr float kPi=3.14159265f;
// Total pitch limits (rad, original table default -0.026; death angle -0.209).
constexpr float kPitchMin=-0.9f,kPitchMax=0.45f;
// Auto-follow: after this many follow-camera frames (1.5 s at the game's 30 Hz) without look input,
// while Jack moves, ease the yaw back behind him (facing + +0x1AE0), then hand back to the original chase.
constexpr uint32_t kFollowIdle=45;
constexpr float kFollowRate=0.06f,kFollowMaxStep=0.035f,kFollowDone=0.09f,kMoveEps=0.005f;
// Target cycle candidates: the original search's widest reach (fn_8027621C cascade: 8 units within
// +-90 deg, 6 all round) and on screen (within this angle of the shown camera's view yaw +0x1134).
constexpr float kCycleReach=8.f,kCycleScreen=0.9f;
struct State {uint32_t player=0,idle=0,frames=0;bool manual=false,armed=false,applied=false,follow=false,was_locked=false;float yaw=0,dyaw=0,pitch=0,facing=0,px=0,pz=0;} g;
float wrap(float a){a=std::fmod(a+kPi,2*kPi);if(a<0)a+=2*kPi;return a-kPi;}
bool locked(uint8_t* m,uint32_t p){return ld32(m,p+0x1254)||ld8(m,p+0x1251);}
// With the orbit-while-locked toggle, a lock-on (player or game set) no longer owns the camera;
// LockCameraAngle and the C recenter countdown always do.
bool original_owns(uint8_t* m,uint32_t p,bool lock_orbit){
    return (!lock_orbit&&locked(m,p))||(ld32(m,p+0x1128)&0x80)||lds16(m,p+0x27C2)!=0;
}
bool guest_obj(uint32_t a){return a>=0x80000000u&&a<0x81800000u-0x400&&!(a&3);}
// Same filters as the original lock search fn_801DF43C over g_EmObjList 80694A84 (linked list:
// node *(list+0x14), object *(node+0), next *(node+8), end *(list+0x18)): not the player, lockable
// (vfunc +0x198), state bit 6 of obj+0x38C clear (fn_801DF2C0), within reach plus radius (vfunc +0x31C).
bool lockable(PPCContext& c,uint8_t* m,uint32_t obj,uint32_t vf){
    PPCContext saved=c;c.r[3]=obj;ppc_call_indirect(c,m,ld32(m,ld32(m,obj)+vf));
    uint32_t r=c.r[3];c=saved;return r!=0;
}
float radius(PPCContext& c,uint8_t* m,uint32_t obj){
    PPCContext saved=c;c.r[3]=obj;ppc_call_indirect(c,m,ld32(m,ld32(m,obj)+0x31C));
    float r=float(c.f[1].ps0);c=saved;return r;
}
// Lock-on cycle: order on-screen candidates by screen x (game yaw grows to the left) and lock the
// next one in `dir` through the original SetLockOn_Trg 802CBDC8 (+0x1254 target, +0x1250 = 1).
void cycle_target(PPCContext& c,uint8_t* m,uint32_t p,int dir){
    uint32_t current=ld32(m,p+0x1254);
    if(!dir||!ld8(m,p+0x1250)||!current)return;
    constexpr uint32_t list=0x80694A84;
    float view=ldf32(m,p+0x1134),jx=ldf32(m,p+0x64),jz=ldf32(m,p+0x6C);
    auto screen_x=[&](uint32_t o){return -wrap(std::atan2(ldf32(m,o+0x64)-jx,ldf32(m,o+0x6C)-jz)-view);};
    float cur=screen_x(current),best=0;uint32_t pick=0,wrap_pick=0;float wrap_best=0;int n=0;
    uint32_t end=ld32(m,list+0x18);
    for(uint32_t node=ld32(m,list+0x14),guard=0;node!=end&&guest_obj(node)&&guard<512;node=ld32(m,node+8),guard++){
        uint32_t o=ld32(m,node);
        if(o==p||o==current||!guest_obj(o))continue;
        if(!lockable(c,m,o,0x198)||(ld32(m,o+0x38C)&(1u<<6)))continue;
        float dx=ldf32(m,o+0x64)-jx,dz=ldf32(m,o+0x6C)-jz,reach=kCycleReach+radius(c,m,o);
        if(dx*dx+dz*dz>reach*reach)continue;
        float x=screen_x(o);if(std::fabs(x)>kCycleScreen)continue;
        n++;float d=(x-cur)*float(dir);
        if(d>0&&(!pick||d<best)){pick=o;best=d;}
        if(d<=0&&(!wrap_pick||d<wrap_best)){wrap_pick=o;wrap_best=d;}// farthest the other way: wrap around
    }
    uint32_t to=pick?pick:wrap_pick;
    if(trace())std::fprintf(stderr,"[pc-camera] cycle dir=%d candidates=%d from=%08x to=%08x\n",dir,n,current,to);
    if(!to)return;
    PPCContext saved=c;c.r[3]=p;c.r[4]=to;f_802CBDC8(c,m);c=saved;
}
}
PPC_FUNC(f_802C9F3C){
    // PC death cinematic (pc_death_cinematic.cpp, MW_PC_DEATH_CINEMATIC) owns the camera in state 95: no orbit.
    if(pc_death_camera_begin(c,m)){
        if(enabled()){float dy,dp;uint32_t fl;if(take()(&dy,&dp,&fl)!=0)ppc_fatal("Rust free-camera failure");}// drain, discard
        g.manual=false;g.pitch=0;g.armed=false;f_802C9F3C_recomp(c,m);pc_death_camera_end(c,m);return;
    }
    if(!enabled()){f_802C9F3C_recomp(c,m);return;}
    uint32_t p=c.r[3];
    float dyaw=0,dpitch=0;uint32_t flags=0;
    if(take()(&dyaw,&dpitch,&flags)!=0)ppc_fatal("Rust free-camera failure");
    cycle_target(c,m,p,int(int8_t(flags>>8)));
    bool input=dyaw!=0||dpitch!=0;
    bool lock=locked(m,p),owned=original_owns(m,p,flags&2);
    if(owned||(lock&&!g.was_locked)){g.manual=false;g.pitch=0;}// lock-on / recenter: the game's camera, as on the Wii
    else if(input)g.manual=true;
    g.was_locked=lock;
    g.idle=input?0:g.idle+1;
    float px=ldf32(m,p+0x64),pz=ldf32(m,p+0x6C);
    bool moving=std::hypot(px-g.px,pz-g.pz)>kMoveEps;g.px=px;g.pz=pz;
    g.follow=g.manual&&!lock&&(flags&1)&&g.idle>kFollowIdle&&moving;
    if(g.manual&&lock&&(flags&1)&&g.idle>kFollowIdle){g.manual=false;g.pitch=0;}// locked: the original lock chase reframes
    g.facing=float(c.f[1].ps0);
    bool normal=ld8(m,p+0x11A)!=2;
    g.player=p;g.yaw=ldf32(m,p+0x1AD4);g.dyaw=dyaw;
    g.pitch=std::fmax(-1.5f,std::fmin(1.5f,g.pitch+dpitch));
    g.armed=g.manual&&!owned&&normal;g.applied=false;
    f_802C9F3C_recomp(c,m);
    bool applied=g.applied;
    g.armed=false;
    if(trace()&&(++g.frames%30==0||input&&g.frames%5==0)){
        std::fprintf(stderr,"[pc-camera] f=%u manual=%d applied=%d owned=%d mode=%u lock=%08x yaw=%.3f view=%.3f facing=%.3f pitch=%.3f+%.3f eye=(%.2f,%.2f,%.2f) dest=(%.2f,%.2f,%.2f) pos=(%.2f,%.2f,%.2f) in=(%.4f,%.4f)\n",
            g.frames,g.manual,applied,owned,ld32(m,p+0x2758),ld32(m,p+0x1254),ldf32(m,p+0x1AC8),ldf32(m,p+0x1134),ldf32(m,p+0x74),
            ldf32(m,p+0x1AC4),g.pitch,ldf32(m,p+0x2798),ldf32(m,p+0x279C),ldf32(m,p+0x27A0),ldf32(m,p+0x27A4),ldf32(m,p+0x27A8),ldf32(m,p+0x27AC),
            ldf32(m,p+0x64),ldf32(m,p+0x68),ldf32(m,p+0x6C),dyaw,dpitch);
    }
}
// Hw::MtxInitRotVec(cMtx&, const cVec& rot, ROT_ORDER): the shared rotation builder; only the
// follow camera's own call (LR 802CA61C, rot = player+0x1AC4) is changed, once per frame.
PPC_FUNC(f_800BA4B0){
    if(pc_death_camera_rot(c,m))return;
    if(!g.armed||c.lr!=kRotCall||c.r[4]!=g.player+0x1AC4){f_800BA4B0_recomp(c,m);return;}
    g.armed=false;
    uint32_t p=g.player;
    // +0x1AE0 was just refreshed by fn_802C9A98 from the angle table or the camera area.
    if(std::fabs(wrap(ldf32(m,p+0x1AE0)-kPi))>1e-3f){f_800BA4B0_recomp(c,m);return;}
    float yaw=wrap(g.yaw+g.dyaw);
    if(g.follow){// gentle ease behind Jack; done -> the original chase owns it again
        float d=wrap(wrap(g.facing+ldf32(m,p+0x1AE0))-yaw);
        yaw=wrap(yaw+std::fmax(-kFollowMaxStep,std::fmin(kFollowMaxStep,d*kFollowRate)));
        g.pitch*=0.95f;
        if(std::fabs(d)<kFollowDone){g.manual=false;g.pitch=0;}
    }
    float center=ldf32(m,p+0x1B24),range=ldf32(m,p+0x1B28);
    if(std::fabs(range-kPi)>1e-4f){float d=wrap(yaw-center);yaw=wrap(center+std::fmax(-range,std::fmin(range,d)));}// camera-area yaw limits
    stf32(m,p+0x1AD4,yaw);stf32(m,p+0x1AC8,yaw);g.applied=true;
    float base=ldf32(m,p+0x1AC4);
    g.pitch=std::fmax(kPitchMin-base,std::fmin(kPitchMax-base,g.pitch));
    stf32(m,p+0x1AC4,base+g.pitch);
    f_800BA4B0_recomp(c,m);
    stf32(m,p+0x1AC4,base);// the original pitch lerp state is kept
}
