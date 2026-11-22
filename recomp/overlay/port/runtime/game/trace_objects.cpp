#include "native_config.h"
#include "snapshot.h"
#include "hle.h"
#include "hle_funcs.h"
#include <cstdio>
#include <cstdlib>
#include <limits>
namespace {
void trace(const char* kind,uint32_t id,uint32_t variant,uint32_t pointer,uint32_t caller,const float* position) {
    static bool enabled=NATIVE_OPTION("MW_TRACE_OBJECTS")!=nullptr;
    static unsigned actors=0,props=0;
    unsigned& count=kind[0]=='a'?actors:props;
    if(enabled&&(count++<256||id==0x20440||id==0x7015d))
        std::fprintf(stderr,"[object] kind=%s request=%08x %s=%08x result=%08x caller=%08x position=(%.6g,%.6g,%.6g) name=%s\n",kind,id,kind[0]=='a'?"variant":"argument6",variant,pointer,caller,position[0],position[1],position[2],ppc_func_name(caller)?ppc_func_name(caller):"unknown");
}
}
PPC_FUNC(f_8026FE74) {
    uint32_t id=c.r[3],caller=c.lr,variant=c.r[6],vector=c.r[4];
    bool mapped=(vector>=0x80000000&&vector<0x81800000-12)||(vector>=0x90000000&&vector<0x94000000-12);
    float position[3]{std::numeric_limits<float>::quiet_NaN(),std::numeric_limits<float>::quiet_NaN(),std::numeric_limits<float>::quiet_NaN()};if(mapped)for(unsigned i=0;i<3;++i)position[i]=ldf32(m,vector+4*i);
    // Wrapper arguments are not the actor factory ABI; do not label r4 as a position.
    for(float& value:position)value=std::numeric_limits<float>::quiet_NaN();
    f_8026FE74_recomp(c,m);trace("prop",id,variant,c.r[3],caller,position);
}
PPC_FUNC(f_8026FED4) {
    uint32_t id=c.r[3],caller=c.lr,variant=c.r[6],vector=c.r[4];
    bool mapped=(vector>=0x80000000&&vector<0x81800000-12)||(vector>=0x90000000&&vector<0x94000000-12);
    float position[3]{std::numeric_limits<float>::quiet_NaN(),std::numeric_limits<float>::quiet_NaN(),std::numeric_limits<float>::quiet_NaN()};if(mapped)for(unsigned i=0;i<3;++i)position[i]=ldf32(m,vector+4*i);
    f_8026FED4_recomp(c,m);native_snapshot_actor(id,c.r[3]);trace("actor",id,variant,c.r[3],caller,position);
}
PPC_FUNC(f_80216534) {
    uint32_t point=c.r[4],slot=c.r[5],caller=c.lr,world=c.r[3];
    f_80216534_recomp(c,m);
    if(NATIVE_OPTION("MW_TRACE_OBJECTS"))
        std::fprintf(stderr,"[player-point] world=%08x requested=%u stored=%u slot=%u caller=%08x name=%s\n",world,point,ld32(m,world+0x38),slot,caller,ppc_func_name(caller)?ppc_func_name(caller):"unknown");
}

PPC_FUNC(f_801EAAE8){
    uint32_t object=c.r[3],data=c.r[4];
    bool on=NATIVE_OPTION("MW_TRACE_OBJECTS")!=nullptr;
    bool mapped=(data>=0x80000000&&data<0x81800000-16)||(data>=0x90000000&&data<0x94000000-16);
    if(on&&!mapped)std::fprintf(stderr,"[enemy-set] load this=%08x data=%08x (empty/unmapped; original loader decides)\n",object,data);
    if(on&&mapped)std::fprintf(stderr,"[enemy-set] load this=%08x data=%08x type=%04x count=%u args=%08x/%08x/%08x\n",object,data,ld16(m,data+4),ld16(m,data+6),c.r[5],c.r[6],c.r[7]);
    f_801EAAE8_recomp(c,m);
    if(on)std::fprintf(stderr,"[enemy-set] loaded this=%08x result=%08x flags=%08x type=%04x generators=%08x\n",object,c.r[3],ld32(m,object+4),ld16(m,object+0x2c),ld32(m,object+0x34));
}
PPC_FUNC(f_801EA61C){
    uint32_t mode=c.r[3],rank=c.r[4],id=c.r[5];
    f_801EA61C_recomp(c,m);
    static unsigned rin=0,other=0;
    if(NATIVE_OPTION("MW_TRACE_OBJECTS")&&(((id==0x440||id==0x20440)&&rin++<16)||(id!=0x440&&id!=0x20440&&other++<8)))std::fprintf(stderr,"[enemy-identity] type=%04x rank=%u source=%08x selected=%08x\n",mode,rank,id,c.r[3]);
}

PPC_FUNC(f_801EB484){
    uint32_t object=c.r[3];
    static uint64_t last=0;
    bool trace=NATIVE_OPTION("MW_TRACE_OBJECTS")&&object==0x80695434&&ld32(m,object+8)&&ppc_timebase()-last>=60750000;
    if(trace){
        last=ppc_timebase();uint32_t flags=ld32(m,0x80697e64);
        bool mapped=(flags>=0x80000000&&flags<0x81800000-4)||(flags>=0x90000000&&flags<0x94000000-4);
        std::fprintf(stderr,"[boss-set-update] flags=%08x mode=%04x demo-counter=%d game-flags-pointer=%08x word0=%08x update-arg=%u\n",ld32(m,object+4),ld16(m,object+0x2c),int32_t(ld32(m,0x80604020)),flags,mapped?ld32(m,flags):0,c.r[4]);
    }
    f_801EB484_recomp(c,m);
    if(trace)std::fprintf(stderr,"[boss-set-update] returned=%08x\n",c.r[3]);
}
PPC_FUNC(f_801EB298){
    uint32_t object=c.r[3],generator=c.r[4],camera=c.r[5],player=c.r[7];
    uint32_t record=object==0x80695434?ld32(m,generator):0;
    f_801EB298_recomp(c,m);
    static unsigned count=0;
    if(NATIVE_OPTION("MW_TRACE_OBJECTS")&&object==0x80695434&&count++<16)std::fprintf(stderr,"[boss-eligibility] source=%08x result=%u camera=(%.5g,%.5g,%.5g) player=%08x\n",ld32(m,record),c.r[3],ldf32(m,camera),ldf32(m,camera+4),ldf32(m,camera+8),player);
}
