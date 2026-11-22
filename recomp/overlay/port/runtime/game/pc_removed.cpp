#include "hle.h"
#include "hle_funcs.h"
// Removed from the PC port: the original boot wrist-strap/safety screen. Boot
// 8026EF0C calls 8026F538 once and ignores its result; it shows strap/strap_%s.tpl
// (fade in, up to 600 frames or a press, fade out). Later boot depends on the GX
// work of that screen (skipping it, even with 150 blank frames, ends in a truncated
// display list), so keep exactly the original's first frame with nothing visible:
// load the strap texture (8021FB84), GX setup (80371A04/80374474), its two quads
// (8026F838) at alpha 0, end of frame (8025BEB4/8026F324), free (8021FD18 twice,
// as the original). No timers, fade, wait for a press or loading poll.
PPC_FUNC(f_8026F538){
    PPCContext saved=c;
    uint32_t sp=c.r[1]-160;
    st32(m,sp,c.r[1]);c.r[1]=sp;
    st32(m,sp+80,0);
    c.r[3]=sp+16;f_8021FB84(c,m);
    st32(m,sp+8,0xffffff00);st32(m,sp+12,0xffffff00);
    f_80371A04(c,m);f_80374474(c,m);
    c.r[3]=sp+8;c.r[4]=sp+48;c.r[5]=0;f_8026F838(c,m);
    c.r[3]=sp+12;c.r[4]=sp+16;c.r[5]=0;f_8026F838(c,m);
    f_8025BEB4(c,m);f_8026F324(c,m);
    c.r[3]=sp+16;f_8021FD18(c,m);
    c.r[3]=sp+16;f_8021FD18(c,m);
    c=saved;
}
