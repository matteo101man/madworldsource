#include "native_config.h"
#include "hle.h"
#include "hle_funcs.h"
#include "pc_death_cinematic.h"
#include <cmath>
#include <cstdio>
#include <cstring>
// PC feature hook (default on, MW_PC_DEATH_CINEMATIC=0 restores the original): Jack's death becomes a short
// cinematic. research/PC-DEATH-CINEMATIC.md has the evidence; research/PLAYER-DEATH-API.md
// the original flow. Everything runs only while the player is in state 95 (+0x11B == 0x5F):
// - 0x802C0F80 (state 95 update) runs unchanged; the wrapper only follows the sequence (phase
//   byte +0x11C reaches 2 when cByMotion::move reports the end of the death motion).
// - 0x8025AA04 (motion step, only caller cByMotion 0x80079388 at LR 800793F4): for Jack's own motion
//   (player+0x1AC) the step is scaled, so only the death motion plays slowly. No scene/global clock.
// - The camera goes through the free-camera hook in pc_camera.cpp (one camera hook): while the
//   cinematic owns the camera, the free-camera orbit is off, and at the follow camera's own
//   MtxInitRotVec (LR 802CA61C) the cinematic writes the follow camera's yaw/pitch/distance state
//   (+0x1AC8/+0x1AD4, +0x1AC4, +0x1AE4); collision, camera manager and g_Camera stay original.
// - 0x801248A4 ("player dead": HP +0xEC <= 0 or +0x1110, or game flag 9) answers 0 to the room
//   scenario update 0x801286FC only (LR 80128714) until the shot is done, so its dead-event task and
//   the continue / game-over sequence 0x80127DFC start after the cinematic instead of on the lethal hit.
// Separate toggle MW_PC_DEATH_NO_STATIC (default on, =0 keeps the static): the continue sequence's TV-static wipe (0x801241A0 at LR
// 80127E54: layout widget 19 start 0x802452DC + wait 0x802452F4) is skipped. Other scene transitions
// that use the same helper keep it.
namespace {
bool env_on(const char* name,bool fallback){const char* v=native_config_get(name);return v?!std::strcmp(v,"1"):fallback;}
bool enabled(){static const bool on=env_on("MW_PC_DEATH_CINEMATIC",true);return on;}
bool trace(){static const bool on=env_on("MW_PC_DEATH_TRACE",false);return on;}
float env_float(const char* name,float fallback,float lo,float hi){
    const char* v=native_config_get(name);if(!v)return fallback;
    float x=float(std::atof(v));return x>=lo&&x<=hi?x:fallback;
}
float slowmo(){static const float s=env_float("MW_PC_DEATH_CINEMATIC_SLOWMO",0.35f,0.05f,1.f);return s;}
constexpr uint8_t kStateDead=0x5F;
constexpr uint32_t kStepCall=0x800793F4;// cByMotion 0x80079388 -> 0x8025AA04
constexpr uint32_t kRotCall=0x802CA61C;// follow camera's MtxInitRotVec
constexpr uint32_t kDeadQuery=0x80128714;// room scenario update 0x801286FC -> 0x801248A4
constexpr uint32_t kStaticCall=0x80127E54;// continue sequence 0x80127DFC -> 0x801241A0
constexpr float kPi=3.14159265f;
// Shot timing in player (follow-camera) frames, ~30 per second.
constexpr uint32_t kPushFrames=40;// push-in and swing to Jack's front three-quarter
constexpr uint32_t kPullFrames=75;// pull back once the death motion has ended
constexpr uint32_t kOrbitMin=105;// orbit shown at least this long before the continue flow
constexpr uint32_t kMaxFrames=900;// safety: never hold the original flow longer than this
constexpr float kFrontYaw=0.95f;// camera yaw relative to facing (pi = behind Jack, 0 = in front)
constexpr float kOrbitRate=0.0105f;// rad per frame (~18 deg/s)
constexpr float kClose=1.45f,kFar=3.4f;// camera distance from the look-at point
constexpr float kClosePitch=-0.22f,kFarPitch=-0.62f;// negative = eye above the look-at point
constexpr float kLookHeight=0.45f;// look-at point above Jack's feet once he is down (original: dest ~1.5-1.9 up)
constexpr float kFallHeight=1.0f;// look-at height during the push-in, while he is still falling
constexpr uint32_t kFallFrames=100;// lowering to kLookHeight after the push-in (at slow-mo 0.35; scaled)
constexpr int kProbes=24;// yaw directions tested with the game's own line query at the start
constexpr float kClearPush=0.85f,kClearArc=0.7f;
struct State {
    uint32_t player=0,frames=0,cam_frames=0,motion_end=0,orbit=0,steps=0,sp=0,center=0;
    bool active=false,armed=false,released=false,full_arc=false;
    float yaw0=0,pitch0=0,dist0=0,facing=0,yaw=0,push=0,arc_lo=0,arc_hi=0,dir=1,height0=1.5f,close=kClose,far=kFar,drop=0;
} g;
float wrap(float a){a=std::fmod(a+kPi,2*kPi);if(a<0)a+=2*kPi;return a-kPi;}
float smooth(float t){t=std::fmax(0.f,std::fmin(1.f,t));return t*t*(3-2*t);}
float lerp(float a,float b,float t){return a+(b-a)*t;}
float lerp_angle(float a,float b,float t){return wrap(a+wrap(b-a)*t);}
bool dead(uint8_t* m,uint32_t p){return ld8(m,p+0x11B)==kStateDead;}
void finish(const char* why){
    if(g.active&&trace())std::fprintf(stderr,"[pc-death] end reason=%s frames=%u camera=%u motion-end=%u orbit=%u\n",why,g.frames,g.cam_frames,g.motion_end,g.orbit);
    g=State{};
}
}
namespace {
bool no_static(){static const bool on=env_on("MW_PC_DEATH_NO_STATIC",true);return on;}
bool cinematic_active(uint8_t* m,uint32_t player){
    if(!g.active||g.player!=player)return false;
    if(!dead(m,player)){finish("state-left-95");return false;}
    return true;
}
}
namespace {
// The follow camera's own wall query (802CA71C): fn_800ABAA8(world 0x80609964, &hit, 0, &from, &to,
// mask 27, 0, 0) != 0 when the segment hits. Read-only; returns the clear fraction of from -> to.
float clear_fraction(PPCContext& c,uint8_t* m,const float from[3],const float to[3]){
    PPCContext saved=c;
    uint32_t sp=(c.r[1]-0x100)&~0xFu;st32(m,sp,c.r[1]);c.r[1]=sp;
    st32(m,sp+8,0);st32(m,sp+12,0);
    for(int i=0;i<3;i++){stf32(m,sp+0x40+4*i,from[i]);stf32(m,sp+0x50+4*i,to[i]);stf32(m,sp+0x60+4*i,to[i]);}
    c.r[3]=0x80609964;c.r[4]=sp+0x60;c.r[5]=0;c.r[6]=sp+0x40;c.r[7]=sp+0x50;c.r[8]=27;c.r[9]=0;c.r[10]=0;
    f_800ABAA8(c,m);
    bool hit=c.r[3]!=0;
    float d=0,h=0;
    for(int i=0;i<3;i++){float t=to[i]-from[i],u=ldf32(m,sp+0x60+4*i)-from[i];d+=t*t;h+=u*u;}
    c=saved;
    return hit&&d>0?std::fmin(1.f,std::sqrt(h/d)):1.f;
}
// Camera offset for yaw/pitch at a distance (eye = dest + R*z*dist; matches the trace:
// x = sin(yaw)cos(pitch), y = -sin(pitch), z = cos(yaw)cos(pitch)).
void offset(float yaw,float pitch,float dist,float out[3]){
    out[0]=std::sin(yaw)*std::cos(pitch)*dist;out[1]=-std::sin(pitch)*dist;out[2]=std::cos(yaw)*std::cos(pitch)*dist;
}
float angle_between(float a,float b){return std::fabs(wrap(a-b));}
// Once, at the start: which directions around the fallen Jack are open. The push-in takes the open
// direction nearest his front three-quarter; the orbit stays inside the open arc around it.
void plan_shot(PPCContext& c,uint8_t* m,uint32_t p){
    float look[3]={ldf32(m,p+0x64),ldf32(m,p+0x68)+kLookHeight,ldf32(m,p+0x6C)};
    float clear[kProbes];
    for(int i=0;i<kProbes;i++){
        float yaw=wrap(-kPi+2*kPi*float(i)/kProbes),o[3],to[3];offset(yaw,kFarPitch,kFar,o);
        for(int k=0;k<3;k++)to[k]=look[k]+o[k];
        clear[i]=clear_fraction(c,m,look,to);
    }
    int best=0;float best_score=-1e9f;
    for(int i=0;i<kProbes;i++){
        float yaw=wrap(-kPi+2*kPi*float(i)/kProbes);
        float near=std::fmin(angle_between(yaw,g.facing+kFrontYaw),angle_between(yaw,g.facing-kFrontYaw));
        float score=(clear[i]>=kClearPush?2.f:clear[i])-near/kPi;
        if(score>best_score){best_score=score;best=i;}
    }
    g.push=wrap(-kPi+2*kPi*float(best)/kProbes);
    int lo=0,hi=0;
    while(lo>-kProbes/2&&clear[(best+lo-1+kProbes)%kProbes]>=kClearArc)lo--;
    while(hi<kProbes/2&&clear[(best+hi+1)%kProbes]>=kClearArc)hi++;
    g.full_arc=hi-lo>=kProbes-1;
    float step=2*kPi/kProbes;
    g.arc_lo=g.push+float(lo)*step;g.arc_hi=g.push+float(hi)*step;// unwrapped around push
    float c0=std::fmax(0.35f,clear[best]);
    g.far=std::fmax(1.6f,kFar*std::fmin(1.f,c0*0.95f));g.close=std::fmin(kClose,g.far*0.7f);
    g.dir=(g.arc_hi-g.push>=g.push-g.arc_lo)?1.f:-1.f;
    if(trace()){
        std::fprintf(stderr,"[pc-death] plan push=%.3f arc=[%.3f,%.3f] full=%d close=%.2f far=%.2f clear:",g.push,g.arc_lo,g.arc_hi,g.full_arc,g.close,g.far);
        for(int i=0;i<kProbes;i++)std::fprintf(stderr," %.2f",clear[i]);
        std::fprintf(stderr,"\n");
    }
}
}
// Called by pc_camera.cpp before the original follow camera (r3 = player, r4 = centre, f1 = facing).
// The follow camera gets a copy of the centre, lowered toward the fallen body during the shot.
bool pc_death_camera_begin(PPCContext& c,uint8_t* m){
    if(!enabled()||!cinematic_active(m,c.r[3]))return false;
    uint32_t p=c.r[3];
    if(!g.cam_frames){
        g.yaw0=ldf32(m,p+0x1AD4);g.pitch0=ldf32(m,p+0x1AC4);g.dist0=ldf32(m,p+0x1AE4);g.facing=float(c.f[1].ps0);g.yaw=g.yaw0;
        g.height0=ldf32(m,p+0x27A8)-ldf32(m,p+0x68);
        if(trace())std::fprintf(stderr,"[pc-death] camera start yaw=%.3f pitch=%.3f dist=%.3f facing=%.3f height=%.2f mode=%u\n",g.yaw0,g.pitch0,g.dist0,g.facing,g.height0,ld32(m,p+0x2758));
        plan_shot(c,m,p);
    }
    // Look-at: chest height while he falls (push-in), down to the body as the slowed motion lands.
    float f=float(g.cam_frames),fall=float(kFallFrames)*0.35f/slowmo();
    float look=lerp(g.height0,kFallHeight,smooth(f/kPushFrames));
    look=lerp(look,kLookHeight,g.motion_end?1.f:smooth((f-kPushFrames)/fall));
    g.drop=std::fmax(0.f,g.height0-look);
    g.sp=c.r[1];g.center=c.r[4];
    uint32_t sp=(c.r[1]-0x40)&~0xFu;st32(m,sp,c.r[1]);c.r[1]=sp;
    stf32(m,sp+0x20,ldf32(m,g.center));stf32(m,sp+0x24,ldf32(m,g.center+4)-g.drop);stf32(m,sp+0x28,ldf32(m,g.center+8));
    c.r[4]=sp+0x20;
    g.armed=true;
    return true;
}
void pc_death_camera_end(PPCContext& c,uint8_t* m){
    c.r[1]=g.sp;c.r[4]=g.center;
    bool ran=!g.armed;// the rot hook disarms; still armed = area/event camera or instant cut this frame
    if(trace()&&(g.cam_frames%15==1||!ran)){
        uint32_t p=g.player;
        std::fprintf(stderr,"[pc-death] cam f=%u ran=%d phase=%u mode=%u yaw=%.3f pitch=%.3f dist=%.3f eye=(%.2f,%.2f,%.2f) dest=(%.2f,%.2f,%.2f) jack=(%.2f,%.2f,%.2f) orbit=%u\n",
            g.cam_frames,ran,ld8(m,p+0x11C),ld32(m,p+0x2758),ldf32(m,p+0x1AC8),ldf32(m,p+0x1AC4),ldf32(m,p+0x1AE4),
            ldf32(m,p+0x2798),ldf32(m,p+0x279C),ldf32(m,p+0x27A0),ldf32(m,p+0x27A4),ldf32(m,p+0x27A8),ldf32(m,p+0x27AC),
            ldf32(m,p+0x64),ldf32(m,p+0x68),ldf32(m,p+0x6C),g.orbit);
    }
    g.armed=false;
}
// Called by pc_camera.cpp's MtxInitRotVec hook. Returns true when it ran the original itself.
bool pc_death_camera_rot(PPCContext& c,uint8_t* m){
    if(!g.armed||c.lr!=kRotCall||c.r[4]!=g.player+0x1AC4)return false;
    g.armed=false;
    uint32_t p=g.player,f=g.cam_frames++;
    float yaw,pitch,dist;
    if(!g.motion_end){// push-in on Jack while the slowed death motion plays
        float s=smooth(float(f)/kPushFrames);
        yaw=lerp_angle(g.yaw0,g.push,s);
        pitch=lerp(g.pitch0,kClosePitch,s);dist=lerp(g.dist0,g.close,s);
        g.yaw=g.push;// the orbit starts from the push-in direction (kept unwrapped, like the arc)
    }else{// pull back and orbit the defeated Jack, inside the open arc
        float s=smooth(float(g.orbit)/kPullFrames);
        float rate=kOrbitRate*(0.35f+0.65f*s);
        if(!g.full_arc){
            float room=g.dir>0?g.arc_hi-g.yaw:g.yaw-g.arc_lo;
            rate*=std::fmin(1.f,std::fmax(0.f,room)/0.35f+0.08f);// ease into each end of the arc
            if(room<=0.02f)g.dir=-g.dir;
        }
        g.yaw+=g.dir*rate;
        yaw=wrap(g.yaw);
        pitch=lerp(kClosePitch,kFarPitch,s);dist=lerp(g.close,g.far,s);
        g.orbit++;
    }
    stf32(m,p+0x1AD4,yaw);stf32(m,p+0x1AC8,yaw);
    stf32(m,p+0x1AC4,std::fmax(-0.9f,std::fmin(0.45f,pitch)));
    stf32(m,p+0x1AE4,dist);
    f_800BA4B0_recomp(c,m);
    return true;
}
// cPl0000 state 95 update (death). Original body every frame; the wrapper only follows the phases.
PPC_FUNC(f_802C0F80){
    if(!enabled()){f_802C0F80_recomp(c,m);return;}
    uint32_t p=c.r[3];
    if(!g.active||g.player!=p||(g.frames&&ld8(m,p+0x11C)==0)){// phase 0 runs once, on entry
        finish("new-death");g=State{};g.active=true;g.player=p;
        if(trace())std::fprintf(stderr,"[pc-death] begin player=%08x phase=%u countdown=%d slowmo=%.2f\n",p,ld8(m,p+0x11C),int32_t(ld32(m,p+0x1230)),slowmo());
    }
    f_802C0F80_recomp(c,m);
    g.frames++;
    if(ld8(m,p+0x11C)>=2&&!g.motion_end){g.motion_end=g.frames;if(trace())std::fprintf(stderr,"[pc-death] motion end frame=%u steps=%u\n",g.frames,g.steps);}
    if(!g.released&&(g.frames>=kMaxFrames||(g.motion_end&&g.orbit>=kOrbitMin))){
        g.released=true;
        if(trace())std::fprintf(stderr,"[pc-death] release continue flow frame=%u orbit=%u\n",g.frames,g.orbit);
    }
}
// "Player dead" query. Only the room scenario update's call is held back, and only during the shot.
PPC_FUNC(f_801248A4){
    f_801248A4_recomp(c,m);
    if(c.r[3]&&c.lr==kDeadQuery&&g.active&&!g.released&&dead(m,g.player))c.r[3]=0;
}
// Continue sequence's TV-static wipe (separate toggle). The helper only starts widget 19 and waits.
PPC_FUNC(f_801241A0){
    if(no_static()&&c.lr==kStaticCall){
        if(trace())std::fprintf(stderr,"[pc-death] static wipe skipped (continue sequence)\n");
        return;
    }
    f_801241A0_recomp(c,m);
}
// Motion step (0x8025AA04: motion+0x38 == 1 ? +0x34 : incoming f1 = +0x64). Only the call from
// cByMotion's frame advance for Jack's own motion, while the cinematic plays the death motion.
PPC_FUNC(f_8025AA04){
    f_8025AA04_recomp(c,m);
    if(!g.active||c.lr!=kStepCall||c.r[3]!=g.player+0x1AC)return;
    if(!dead(m,g.player)){finish("state-left-95");return;}
    if(g.motion_end)return;
    c.f[1].ps0=c.f[1].ps1=double(float(c.f[1].ps0)*slowmo());
    g.steps++;
}
