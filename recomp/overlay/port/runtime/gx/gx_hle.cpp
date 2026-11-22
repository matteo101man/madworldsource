#include "native_config.h"
#include "hle.h"
#include "hle_funcs.h"
#include "os.h"
#include <cstdio>
#include <cstring>
#include <vector>
// GX HLE at the SDK boundary (research/GX-HLE.md, names from research/gx-sdk-functions.json).
// MW_GX_HLE unset or "legacy": every hook runs the original SDK body only.
// MW_GX_HLE=compare: the original still runs and drives the FIFO; each hook also encodes
// the call from its arguments and the SDK's __GXData shadow, and checks
//   1. the encoding against the bytes the original actually wrote to the FIFO pipe;
//   2. the pre-call shadow register against the BP register the FIFO had processed,
//      which differs when display lists or inline writes changed state behind the SDK.
// A summary is printed every 20000 checked calls and at exit.
void native_gx_capture(std::vector<uint8_t>* out);
uint32_t native_gx_bp(unsigned reg);
namespace {
bool compare(){
    static const bool on=[]{
        const char* v=native_config_get("MW_GX_HLE");
        if(!v||!std::strcmp(v,"legacy"))return false;
        if(!std::strcmp(v,"compare"))return true;
        ppc_fatal("MW_GX_HLE must be legacy or compare");return false;
    }();
    return on;
}
struct Stat {const char* name;uint64_t calls=0,byte_diff=0,shadow_diff=0;};
enum Fn {ColorIn,AlphaIn,ColorOp,AlphaOp,Blend,ZMode,FnCount};
Stat stats[FnCount]={{"GXSetTevColorIn"},{"GXSetTevAlphaIn"},{"GXSetTevColorOp"},{"GXSetTevAlphaOp"},{"GXSetBlendMode"},{"GXSetZMode"}};
uint64_t checked=0;
void report(){
    std::fprintf(stderr,"[gx-hle] compare after %llu calls:",(unsigned long long)checked);
    for(auto& s:stats)std::fprintf(stderr," %s=%llu/%llu/%llu",s.name,(unsigned long long)s.calls,(unsigned long long)s.byte_diff,(unsigned long long)s.shadow_diff);
    std::fprintf(stderr," (calls/byte-mismatch/shadow-vs-fifo)\n");
}
struct Reporter {~Reporter(){if(checked)report();}} reporter;
uint32_t gx_data(PPCContext& c,uint8_t* m){return ld32(m,c.r[2]-30296);}
// Run the original body with the FIFO captured and check one BP write of `expect`.
template<class Original>
void check(Fn fn,uint32_t shadow,uint32_t expect,Original original){
    Stat& s=stats[fn];++s.calls;++checked;
    unsigned reg=shadow>>24;
    if((native_gx_bp(reg)&0xffffff)!=(shadow&0xffffff)){
        if(++s.shadow_diff<=4)std::fprintf(stderr,"[gx-hle] %s shadow %08x != fifo bp[%02x] %06x\n",s.name,shadow,reg,native_gx_bp(reg));
    }
    std::vector<uint8_t> bytes;bytes.reserve(8);
    native_gx_capture(&bytes);
    original();
    native_gx_capture(nullptr);
    const uint8_t want[5]={0x61,uint8_t(expect>>24),uint8_t(expect>>16),uint8_t(expect>>8),uint8_t(expect)};
    if(bytes.size()!=5||std::memcmp(bytes.data(),want,5)){
        if(++s.byte_diff<=4){
            std::fprintf(stderr,"[gx-hle] %s bytes differ: want 61%08x got",s.name,expect);
            for(uint8_t b:bytes)std::fprintf(stderr," %02x",b);
            std::fprintf(stderr,"\n");
        }
    }
    if(checked%20000==0)report();
}
uint32_t set(uint32_t v,uint32_t field,unsigned shift,uint32_t mask){return (v&~(mask<<shift))|((field&mask)<<shift);}
uint32_t tev_op(uint32_t v,uint32_t op,uint32_t bias,uint32_t scale,uint32_t clamp,uint32_t out){
    v=set(v,op,18,1);
    if(int32_t(op)>1){v=set(v,op>>1,20,3);v|=3u<<16;}  // compare ops: scale field holds the compare mode
    else{v=set(v,scale,20,3);v=set(v,bias,16,3);}
    v=set(v,clamp,19,1);
    return set(v,out,22,3);
}
}

PPC_FUNC(f_80374FA8){  // GXSetTevColorIn(stage, a, b, c, d)
    if(!compare()){f_80374FA8_recomp(c,m);return;}
    uint32_t shadow=ld32(m,gx_data(c,m)+384+(c.r[3]<<2));
    uint32_t v=set(set(set(set(shadow,c.r[4],12,15),c.r[5],8,15),c.r[6],4,15),c.r[7],0,15);
    check(ColorIn,shadow,v,[&]{f_80374FA8_recomp(c,m);});
}
PPC_FUNC(f_80374FE8){  // GXSetTevAlphaIn(stage, a, b, c, d)
    if(!compare()){f_80374FE8_recomp(c,m);return;}
    uint32_t shadow=ld32(m,gx_data(c,m)+448+(c.r[3]<<2));
    uint32_t v=set(set(set(set(shadow,c.r[4],13,7),c.r[5],10,7),c.r[6],7,7),c.r[7],4,7);
    check(AlphaIn,shadow,v,[&]{f_80374FE8_recomp(c,m);});
}
PPC_FUNC(f_80375028){  // GXSetTevColorOp(stage, op, bias, scale, clamp, out)
    if(!compare()){f_80375028_recomp(c,m);return;}
    uint32_t shadow=ld32(m,gx_data(c,m)+384+(c.r[3]<<2));
    check(ColorOp,shadow,tev_op(shadow,c.r[4],c.r[5],c.r[6],c.r[7],c.r[8]),[&]{f_80375028_recomp(c,m);});
}
PPC_FUNC(f_80375080){  // GXSetTevAlphaOp(stage, op, bias, scale, clamp, out)
    if(!compare()){f_80375080_recomp(c,m);return;}
    uint32_t shadow=ld32(m,gx_data(c,m)+448+(c.r[3]<<2));
    check(AlphaOp,shadow,tev_op(shadow,c.r[4],c.r[5],c.r[6],c.r[7],c.r[8]),[&]{f_80375080_recomp(c,m);});
}
PPC_FUNC(f_803758E4){  // GXSetBlendMode(type, src, dst, logic op)
    if(!compare()){f_803758E4_recomp(c,m);return;}
    uint32_t shadow=ld32(m,gx_data(c,m)+544),type=c.r[3];
    uint32_t v=set(shadow,type,0,1);
    v=set(v,type==2,1,1);
    v=set(v,type==3,11,1);
    v=set(set(set(v,c.r[6],12,15),c.r[4],8,7),c.r[5],5,7);
    check(Blend,shadow,v,[&]{f_803758E4_recomp(c,m);});
}
PPC_FUNC(f_8037598C){  // GXSetZMode(enable, func, update)
    if(!compare()){f_8037598C_recomp(c,m);return;}
    uint32_t shadow=ld32(m,gx_data(c,m)+552);
    uint32_t v=set(set(set(shadow,c.r[3],0,1),c.r[4],1,7),c.r[5],4,1);
    check(ZMode,shadow,v,[&]{f_8037598C_recomp(c,m);});
}
