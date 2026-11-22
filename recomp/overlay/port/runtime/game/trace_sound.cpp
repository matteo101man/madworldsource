#include "native_config.h"
#include "hle.h"
#include "hle_funcs.h"
#include <cstdio>
#include <cstdlib>

namespace {
void trace(const char* name,PPCContext& c) {
    static bool enabled=native_config_get("MW_TRACE_SOUND")!=nullptr;
    static uint32_t count=0;
    if(enabled&&count++<256)
        std::fprintf(stderr,"[sound] %s r3=%08x r4=%08x r5=%08x r6=%08x lr=%08x\n",name,c.r[3],c.r[4],c.r[5],c.r[6],c.lr);
}
}
PPC_FUNC(f_8026A108){trace("PlayBGM_EX",c);f_8026A108_recomp(c,m);}
PPC_FUNC(f_8026B558){trace("PlayVoiceStream",c);f_8026B558_recomp(c,m);}
PPC_FUNC(f_802683D0){trace("PlayCustomSE",c);f_802683D0_recomp(c,m);}
PPC_FUNC(f_80268628){trace("PlayCustomSE_1Shot",c);f_80268628_recomp(c,m);}
