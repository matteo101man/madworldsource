// Original effect-request diagnostics (MW_TRACE_EFFECTS=1). Every wrapper calls the native body unchanged and only
// reads guest memory. Data managers: EffCore 8060412C (core2.eff), EffRoom 80604148 (stage bank).
#include "native_config.h"
#include "hle.h"
#include "hle_funcs.h"
#include <cstdio>

namespace {
bool enabled(){static bool on=native_config_get("MW_TRACE_EFFECTS")!=nullptr;return on;}
uint32_t budget=4096;
const char* bank(uint32_t manager){return manager==0x8060412Cu?"core":manager==0x80604148u?"room":"other";}
void vec(uint8_t* m,uint32_t at){
    if(at>=0x80000000u&&at<0x81800000u)std::fprintf(stderr," pos=[%.6g,%.6g,%.6g]",ldf32(m,at),ldf32(m,at+4),ldf32(m,at+8));
}
}
// setEffect(cEffDataManager&, eEstId, cArticular*, const cVec*, long, ul, cArticular*, ul, ul)
PPC_FUNC(f_801FD578){
    if(!enabled()||!budget){f_801FD578_recomp(c,m);return;}
    const uint32_t manager=c.r[4],id=c.r[5],owner=c.r[6],pos=c.r[7],part=c.r[8],lr=c.lr;
    std::fprintf(stderr,"[effect] setEffect bank=%s manager=%08x est=%u owner=%08x part=%d lr=%08x",bank(manager),manager,id,owner,(int32_t)part,lr);
    vec(m,pos);
    f_801FD578_recomp(c,m);
    std::fprintf(stderr," work=%08x\n",c.r[3]);--budget;
}
// CreateEff(eEff, const cVec&, ul): eEff is the core EST id
PPC_FUNC(f_801FE990){
    if(enabled()&&budget){std::fprintf(stderr,"[effect] CreateEff est=%u flags=%u lr=%08x",c.r[4],c.r[6],c.lr);vec(m,c.r[5]);std::fprintf(stderr,"\n");}
    f_801FE990_recomp(c,m);
}
// CreatePartsEff(ePartsEff, cArticular*, long, f, f, f, f, cMtx*, cVec*): ePartsEff is the core EST id
PPC_FUNC(f_801FE7AC){
    if(enabled()&&budget)std::fprintf(stderr,"[effect] CreatePartsEff est=%u owner=%08x part=%d f=[%.6g,%.6g,%.6g,%.6g] lr=%08x\n",
        c.r[4],c.r[5],(int32_t)c.r[6],c.f[1].ps0,c.f[2].ps0,c.f[3].ps0,c.f[4].ps0,c.lr);
    f_801FE7AC_recomp(c,m);
}
// cEmObjList::getBloodColor(long) const
PPC_FUNC(f_801E1680){
    const uint32_t which=c.r[4],lr=c.lr;
    f_801E1680_recomp(c,m);
    if(enabled()&&budget)std::fprintf(stderr,"[effect] getBloodColor(%d)=%08x lr=%08x\n",(int32_t)which,c.r[3],lr);
}
