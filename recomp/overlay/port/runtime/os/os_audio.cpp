#include "native_config.h"
// Original DSP initialization runs when the optional Rust DSP bridge is enabled.
// Without it retain upstream's headless CPU-only bootstrap behavior.
#include "hle.h"
#include "hle_funcs.h"
#include <cstdlib>

// void __OSInitAudioSystem(void)
PPC_FUNC(f_802FA050) {if(native_config_get("MW_PLATFORM_DLL"))f_802FA050_recomp(c,m);}

// void __OSStopAudioSystem(void)
PPC_FUNC(f_802FA218) {if(native_config_get("MW_PLATFORM_DLL"))f_802FA218_recomp(c,m);}
