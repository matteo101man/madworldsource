# Audio stream catalog

`audio-stream` contains original relative path, source SHA256 and unmodified vgmstream r2168 JSON metadata. Sample rate, channels, sample count and loops are decoder observations. Names are source names, not verified gameplay roles. Runtime event IDs and mixing remain separate requirements.

## EV21 effects and shared definition fix (2026-11-09)

The shared sound-definition table offset is+0x40(native804F835C), corrected
from the old+0x44 guess. Restaged1143 bindings/66 banks; see
SE-DEFINITION-TABLE-FIX.json. The owned scene_mix7522 bank regression checks
all7 IDs, including formerly missing sound6. The general one-shot interpreter
is still partial; corrected lookup alone does not establish complete SFX parity.

EV21 SE13 flags5 use numeric2D playback, not the actor attachment named in
unused fields. Native8026548C loads se/event/scene_mix7522.rsp. IDs7522001/6
use waves1/6 at180/866, note60, volume127. Native0x12 is key-on(note),0x60
effect-send flags,0x15 u16 wait. Sequencer804FDA28 runs at mean0.0104s/tick:
1500/350 ticks give15.6/3.64s. The two raw waves are mono24kHz and nonlooping.

The read-only SDK capture ev21-native-se-v2 confirms program bytes, selected
waves, note and step over576 active rows. Default dry left/right volume words
are23169 each; Rust applies23169/32768 to the non-spatial mono playback before
its PC SFX/master buses. tools/events/stage_effects.py validates these facts and
writes EV21-SE-COMPARE.json. Rust uses source hold times/crossed-cue seeks and
preloads both effects before starting the event clock. Native effect sends/
auxiliary reverb, release envelopes, DSP resampling/pitch and mixed PCM parity
remain unported/unverified; this is the dry path, not full native audio emulation.

## EV21 stream commands and mix (2026-11-09)

`tools/events/stage_audio.py` resolves BGM/VCE DOL name tables and stages source
EV21 stereo32kHz music(768012 samples) and English mono16kHz voice(465346).
SEQ requests EV21_RSD/0x3D and JKSNN_EVVOSTR_EV21/0x11021 at0, then
BS04C_RSD/0x1E at748. Native capture ev21-native-audio.jsonl confirms manager
names/IDs and startup states; research/EV21-AUDIO-COMPARE.json reports them.

80073DA8 mode2 calls8026A878 with fade-out/in, using music master100 rather
than command volume60. A zero14F4 slot field during startup is not silence:
80266968 starts at0 then calls805034D4 toward the master. 804F8478 converts
60Hz fade units into333 callbacks/s;96 output samples at32kHz per callback.
80073D70 leaves bit4 persistent music running at task exit.

80501E64 stores `(volume+1)*256-1` in12-bit fractional units (zero is mute).
805018A8 initializes per-channel level from0x80800 and the channel header;
80502320 combines stream header/master/channel by integer shifts before
804FB200's output-master/pan/AX mix. `mw_sim::event::stream_gain` translates
the stream scalar normalized by32768 at full output master. RSD header at
0x30 has volume at+0x21 and channel volume at+0x27+8*n. EV21 is70/127,
BS04C60/127, voice127/127; base gains are0.4376526,0.37600708,0.9999695.
Rust PC master and independently persisted Music/Cutscene buses multiply these.
Stereo routing/output-master/AX integer PCM parity has not been measured.

Opt-in Rust intro inspection preloads source assets, executes its cues and
retains the boss stream across its inspection handoff into the existing loop
player. Fixed-frame visual captures are silent. Native preceding-stage music
overlap, SE DSP effects/release, subtitles/ID/effects and full original event/room
lifecycle are still missing. This does not enable the intro in normal startup.

## Sound effects for the native slice (2026-11-08)
- **Ids [native]:** jkSound (PlayCustomSE 802683D0 -> 80268E18) takes decimal ids: block = id / 1000, sound = id % 1000
  (e.g. RinRin footstep 0x6B44F0 = 7030000 -> block 7030). Motion cues `_10.seq` carry (block, sound) as s16 pairs:
  op 29 SE, op 42 voice, op 38 a footstep family, op 37 footsteps.
- **Packs [native]:** `se/scene_mixNNNN.rsp` = PACK: member 0 = PACK of RedSpark wave banks (bank mode, one per
  block, header +0x14 = block), other members = RedSpark sound-definition banks (+0x14 = block). Headers are
  XOR-encrypted with vgmstream's RedSpark key schedule (`tools/audio/redspark.py`); data after the header is plain.
  RinRin's blocks (4203 SE, 4614, 7030/7031 footsteps, 8117 voice) are in scene_mix0221; Jack's common ones (0, 10-19,
  1100-1103, 1108, 8100 voice) in scene_mix0000; chainsaw 1104 and stage blocks in scene_mix0201.
- **Definition bank [native804F835C]:** +0x30 count, **+0x40** u32 offsets (0xFFFFFFFF unused);
  program start = bank+0x40+4*count+(offset&0x7FFFFFFF). +0 track count(mask0x7F),
  +1 volume,+2 wave-bank count,+0x10 u16 wave-bank IDs, then u16 track lengths
  (mask0x7FFF), then commands.804F6DBC selects the first available wave bank.
  The old+0x44 table read shifted all IDs and discarded last entries; it is fixed
  and the shared manifest restaged. SE-DEFINITION-TABLE-FIX.json records changes.
- **Verified handlers:**0x20 wave(804FF610),0x22 volume(804FF6EC),0x12 note/key-on
  (804FF2AC),0x15 u16 sequencer wait(804FF474),0x60 effect-send flags(80500B0C),
  0x00 end/key-off(804FE96C). The earlier0x12-pan/0x60-key-on labels were wrong.
  General track inspection still approximates one play per track and skips other
  opcodes;0x1F's interpreted delay and0x14 remain unverified here.
- **Staging:** `python -I tools/audio/stage_se.py` -> `assets/native/audio/se/{block}-{wave}.wav` + `se.json`
  (block -> sound -> plays). `mw-play` fires the cues crossed by Jack/Kreely and RinRin each game frame
  (`MW_SOUND_LOG=1` prints them); the player's op-37 footsteps use the stage ground block 14 and the delay unit is
  taken as60Hz ticks[both inferred]. Not yet: complete program playback,
  ADSR/pitch/effect sends, native pan/routing and3D curve.

## BGM, positioning and announcer (2026-11-09)
- **BGM table [native]:** index -> `BGM/*.rsd` at 805CB2C8 (0-14 stage `st*ab`, 20-25 `bs00-05`, 26-31 `bs00c-05c`,
  33-38 jingles, 39 silent, 40+ events). 801246D8(id) requests a track (0x14..0x19 skip while the matching c variant
  plays), 8026AC24 / 8026A4A4 stream it, 8026AC2C remaps stage tracks.
- **Stage 201 boss [native + observed]:** the boss-entry script 80147BBC requests 0x18 (`bs04.rsd`); the
  recompiled-core disc log (`evidence/bevy-native-pc-ui-disc.log`) streams `st06ab` in the stage, `ev21` for the
  cutscene and `bs04c` in the boss fight. `bs04` / `bs04c` share the loop (2420362 samples at 32 kHz); bs04c has the
  17.5 s intro. The c variant comes from the pre-boss event: 80147BBC plays event 0x21 (80123820), whose BGM timeline (ev21_us.dat SEQ, 80073DA8) names BS04C; the boss script's 801246D8(0x18) is then skipped by its own "c variant playing" check. No jingle is requested on the boss path.
- **Staging:** `python -I tools/audio/stage_bgm.py` -> `assets/native/audio/bgm/bs04c-{intro,loop}.wav` split at the
  RedSpark loop points; mw-play plays the intro, then loops the loop section, stops at Victory [the result jingle is
  not ported]. Newly staged native mix metadata supplies the gain; old manifests
  retain the inferred0.6 fallback. `MW_NO_BGM=1` turns it off.
- **Positioning [PC]:** sound effects are spatial emitters at the actor that cued them, heard from the camera
  (rodio stereo pan + 1/d^2 falloff, distances x0.4). Native3D curve, pan/routing,
  pitch and envelopes are not applied to the general fight SFX path.
- **Announcer [native]:** jkSound::PlayVoiceStream (8026B558) with id < 0x10000 streams
  `announce/<lang>/an_c<room>_pack_<lang>.bin`; the id is looked up (8026BB14) in `an_c<room>_info_<lang>.bin`
  (member 18 of case2/c201.dat for eng): `apd`, u16 count at +6, 0x108-byte entries, u16 id at +0x14, u8 group mask
  at +0x16, 8 groups x 8 u32 clip offsets at +0x18 (each clip a RedSpark stream). Per-id cooldown +0x1BB8 = 0x708
  (0x1005 18000, 0x1018 0x1518, 0x43 0x546). Group choice 8026BCE0: groups in the mask, least used in a 256-entry
  (id, group) history, random among them avoiding the last; clip choice 8026C008: random in the group. Same id while
  playing is dropped; others wait in a 4-slot queue for 0x5A frames.
- **Ported triggers [native]:** idle fidget 0x1117 (8027B2D8), giant swing spin 0x100A (802AF904, gated by +0x11AC
  [taken as set]) and throw 0x100B (802AF9F4), death 0x1144 (802C10D8), low HP 0x1018 (8027621C, HP < 0.3 x max). The DOL has 97 `bl 8026B558` sites (about 40
  in cPl0000, e.g. 0x1143 / 0x1018 in 8027621C, 0x1145-0x1147 in 80271814, 0x1106 in 8028B41C) not ported yet; ids not in the c201 table (e.g. 0x12F, 0x35, 0x129) are
  ignored by the lookup. `python -I tools/audio/stage_announce.py` stages the four ids (24 clips). The presenter's
  draws use their own cRand instance [native: the shared one]; the cooldown unit is taken as game frames [inferred].
