# Boss-unlock parameter evidence

Index: boss-unlock.json. Typed graph: boss-unlock.graph.jsonl. Rebuild with tools/index_boss_unlock.py. Original decoded operations remain in boot-pcode.graph.jsonl; query bounded address windows with tools/query_boot.py:summarize(root,function,window=(start,end)). Do not expand an entire large stage function just to inspect a small call region.

Vocabulary: native-config-key is an original INI key; native-function is an original code entry; parses-storage relates a parser to its parameter field; copies-parameter records original field transfer; reads-parameter records a proven getter/index; calls-lead is only a bounded syntactic argument lead and does not prove eligibility. Addresses are guest addresses, never host pointers.

Room parser801E5FE8: copy120 original key pointers from8055ECE0; search the incoming key; ordinary point branch converts the value and stores at parserThis+0x38+4*keyIndex. Unlock5 has index36, giving parserThis+0xC8. Other key categories have other branches and must not inherit this rule.

Enemy-list initialization801DBB14: room parser object is listThis+0xB8. The original100-value merge copies from listThis+0x120 to listThis+0x7A0, subject to its original zero-handling branch. Unlock5 starts at parsed list offset0x180, corresponding to merged enum24 and list offset0x800. No field is changed by this analysis.

Getter801E3AAC: return load32(listThis+0x7A0+4*enum). Enum24 therefore reads offset0x800. The25s native trace confirms listThis80694A84, enum24 and value2000000, read by stage201 setup at80146340.

Stage201 setup80145934: registers target BossSw with message invoke and callback801327A4; constructs native message/condition objects; reads enum24 at8014633C; also references c20101.ems and c20104.ems. This proves a stage201 association and threshold read. The score comparison still needs its own eligibility proof. Later runtime evidence below confirms the explicit switch request, c20110.ems load and native Rin initialization. Eight syntactic enum24 call sites are indexed; their function identities remain bounded leads except for the focused stage201 facts above.

Original c201em.ini assigns2000000 to Unlock5 with a boss comment. Display scaling, live progression state and the complete encounter remain unresolved. The diagnostic PLP point5 only places Jack on the roof.

Runtime stage201 setup executes at object925572E0 and returns normally. BossSw/invoke callback801327A4 passes signed argument-97 through80123B7C to the named execRoomEvent message dispatcher. The native binding and its effects are indexed below; this early callback observation alone was not sufficient proof of battle entry.


Native entry closure is now indexed in stage201-native-logic.graph.jsonl and stage201-native-logic.json. Original stage constructor801447EC initializes19 room events, each40 bytes, starting at this+0xE8. The native runtime snapshot confirms event-97 binds callback801470C4. Its direct call path reaches801479E0 -> boss constructor80147A4C. The native callback descriptor references80147B7C, which runs80147BBC. These links do not replace native task scheduling.

The240s diagnostic run uses the original BossSw/invoke callback as an explicit standalone-start request. It observes80147BBC executing, event21 stepping through all four cuts with total1080, event-request return, subsequent original PLP point5 reset, and BGM selector24. A startup-only virtual A pulse is logged separately; controls are neutral during the intro. The cinematic actor requestD000A/variant20440 is distinct from a gameplay Rin actor20440; the generic scene-factory logger does not cover enemy creation. The later class-specific lifecycle trace confirms gameplay Rin initialization and updates. This is headless evidence, not rendered/cutscene/audio equivalence.

Direct enemy evidence: native-rin-lifecycle.graph.jsonl binds RSO cEm0440 allocation81120BE4, onInit810C9CF0(return1, object ID20440) and onMove810CAC70 to the observed object815B8D80. At least240 native update calls are observed with finite changing positions. Late read-only state reports demo counter0; no corrective mode/score bit patch is needed. Health fields EC/EE are only a layout lead until an actual health snapshot is captured. Bevy rendering, interaction/win-loss tests and original fidelity remain unfinished.
