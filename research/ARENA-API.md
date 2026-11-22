# Native RinRin arena lead

SCR: 16-byte header, 96-byte descriptors, relative MDB offsets and embedded TPL. Layout authority: MIT Kerilk/noesis_bayonetta_pc MadWorld.h, pinned in explorer THIRD-PARTY.txt. All c201 transforms are identity; nonidentity transforms fail staging pending basis verification.

EMS v20: 32-byte header and 128-byte records. Native cEmSet helpers 0x801ebb74/0x801ebb48 give header/record sizes. Create wrapper 0x801ea684 passes record u16 at0 through 0x801ea61c family table indexed by byte0x4d; family bases at0x8055fba8 are 0x20000,0x20000,0x10000,0x70000. Position at0x10 is passed to CreateObj at0x801ea938/0x801ea980. c20110 record0 is0x20440 at(2.3005285,17.822876,-68.66516). Original ev21 camera is in the same world region. Native collision geometry is indexed separately in collision.graph.jsonl; native visibility/contact behavior remains unverified.
