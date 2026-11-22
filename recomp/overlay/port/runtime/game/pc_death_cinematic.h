#pragma once
#include "ppc_context.h"
// PC death cinematic (pc_death_cinematic.cpp) <-> free camera (pc_camera.cpp): one camera hook.
// begin/end bracket the original follow camera 0x802C9F3C; true from begin = the cinematic owns the
// camera this frame (free-camera orbit off). rot runs at the follow camera's MtxInitRotVec and
// returns true when it has called the original itself.
bool pc_death_camera_begin(PPCContext& c,uint8_t* m);
void pc_death_camera_end(PPCContext& c,uint8_t* m);
bool pc_death_camera_rot(PPCContext& c,uint8_t* m);
