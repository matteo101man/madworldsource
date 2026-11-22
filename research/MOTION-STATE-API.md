# Bounded native motion state

`motion-state-pcode.graph.jsonl` contains thirteen original functions. All thirteen report complete decompilation without errors in `motion-state-pcode-report.json`. Query through `tools/query_boot.py --graph motion-state-pcode.graph.jsonl`; `motion-state-pseudocode.json` records the bounded dependencies. This is layout and call evidence, not behavioral equivalence.

Original `cPl0000::frameUpdate` calls cByMotion::set with player+0x1AC. The recovered set function stores its resource argument at motion+0x54 (32 bits). clear writes zero there. setFrame writes its float argument at motion+0x58. set stores the frame argument at +0x58/+0x5C and first float argument at +0x60 and third at +0x64. The read-only reader exposes these fields as current motion pointer, frame, previous frame and scalar60/scalar64. Previous-frame and scalar semantics still require runtime temporal confirmation; no playback-rate meaning is inferred from the first float argument alone.

SetLockOn_Trg stores target at player+0x1254 (32 bits) and one at player+0x1250 (8 bits); Reset clears both. LockCameraAngle updates player+0x1128 bit0x80; this is distinct from the target pointer and must not be conflated with target acquisition.

`player-motion-live-index.json` indexes all248 catalog MOT resources in pl/pl0030 DAT banks. A64-byte header SHA256 selects candidates only. The reader then hashes the entire bounded live resource and requires exact equality with the source DAT member SHA256 before naming a resource match. Ambiguous duplicates are retained as multiple matches. Missing matches remain explicit; no source filename or action name is guessed from frame counts. Runtime reads are concurrent, not frame-atomic.

The headless steering probe observes only this task native process. It learns W/D world displacement while holding the mapped camera key, then chooses ordinary WASD inputs toward Rin. Every game-facing action passes through the same PcControls mapper used by the frontend. No actor, health, motion, game flag or camera memory is written. Calibration is test behavior, not production camera control. It does not establish attack recognition, timing or full battle closure.

The original enemy-module isNowAttackFrm method loads three8-bit fields at player+0x1280, +0x1281 and +0x1A94. The reader records them as raw playerAttackFlagBytes. Nonzero fields are not proof of contact, damage or correct collision timing. getArmType is also in the graph; its indexed arm-slot layout is not yet exposed as a decoded runtime field.

Early diagnostic records used the name rate for the raw float at motion+0x60. That semantic name was unverified and is superseded by scalar60/scalar64; no change was made to game state.
