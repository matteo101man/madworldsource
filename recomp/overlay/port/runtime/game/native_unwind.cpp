#include "native_config.h"
// Preserve native exception matching/destructors/register restoration. Only the
// final machine nonlocal branch crosses the corresponding host call frames.
#include "hle.h"
#include "hle_funcs.h"
#include <cstdio>
#include <cstdlib>

struct NativeUnwind { uint32_t target; uint32_t stack; };

void ppc_native_unwind(PPCContext& c, uint8_t*, uint32_t target) {
    if (NATIVE_OPTION("MW_TRACE_WORLD"))
        std::fprintf(stderr, "[unwind] native jump target=%08x sp=%08x r30=%08x r31=%08x\n", target, c.r[1], c.r[30], c.r[31]);
    throw NativeUnwind{target, c.r[1]};
}

PPC_FUNC(f_8021F368) {
    uint32_t entry_stack = c.r[1];
    bool resumed = false;
    try {
        f_8021F368_recomp(c, m);
    } catch (const NativeUnwind& jump) {
        if (jump.target != 0x8021F3A4u || ld32(m, jump.stack) != entry_stack)
            ppc_fatal("native task unwind needs a matching host continuation: target=%08x sp=%08x entry=%08x backchain=%08x",
                      jump.target, jump.stack, entry_stack, ld32(m, jump.stack));
        resumed = true;
    }
    // The continuation can suspend indefinitely. End the catch first so no
    // heap-allocated C++ exception survives on a parked guest stack.
    if (resumed) f_8021F3A4(c, m);
}
