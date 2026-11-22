# Native hit-state vocabulary

`hit-state-pcode.graph.jsonl` records six bounded original functions; `hit-state-pseudocode.json` exposes dependency queries. The base isEnableHitAttack method returns1; reserveDamageReaction returns0 and getBlowAtkType returns0x127. These are observed base methods; they do not imply every boss state accepts every attack.

cEnemyBase::reserveAtkHit checks pending byte at this+0x9CD before storing: attack type32 at +0x9DC; pending byte1; twelve-byte vectors at +0x9E4/+0x9F0; argument r8 at +0x9E0; source object32 at +0xA28; zero32 at +0x9D0; and argument r10 at +0xA00. The concurrent reader records these raw fields for Rin. An equal source pointer is evidence of a stored actor reference, not independent proof of a new attack/contact/damage event. The fields may be consumed or reused between samples; zero samples cannot exclude a transient event.

cEmBoss::registAtkHit builds original attack shapes using setAtkType and sphere/capsule APIs, and calls native Havok attack queries. It concerns outgoing boss attacks; do not mislabel it as applying incoming player damage. onHitChk contains positional separation logic and should not be equated to damage application merely from its name.

Pcode includes explicit FPSCR/CALLOTHER uncertainty around floating-point operations. No scalar or branch equivalence is inferred from those fragments. The native core still executes original recompiled functions; this graph is diagnostic only.
