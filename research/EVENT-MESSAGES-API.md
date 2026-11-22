# Original event MESS messages

EV21 has no MESS chunk. Its English source has CAM, OBJ, ID, EFCT, BGM, VCE
and SE tracks, with no MSD/MTD message data or message commands.800A4E74
allocates/registers the80112FEC message task only when it encounters MESS.
The VCE task80189F04 handles audio stream commands separately. Earlier notes
calling dialogue subtitles a missing EV21 consumer were incorrect.

Playable entry now loads a source-hashed optional message manifest and checks
it against the staged event source. EV21's manifest explicitly records
track:null. Sound's persisted subtitle option is connected to message admission
and visibility, while this particular intro has no original text to request.

## Source and command boundary

tools/events/stage_messages.py stages EV21's absence and the positive EV00
case. EV00 is the Man Darts introduction, not the B01 opening cinematic.
Its MESS contains MSD, MTD and SEQ.80112FEC aligns each MSD/MTD child header
plus39 down to32; treating the unaligned payload as the data would retain the
container padding. Staging preserves the exact tokens,35 original glyphs,
metric sizes, unmodified font page and counted sequence commands. Original
container/MESS/page hashes accompany the assets.

80113198 handles opcode18 (0x12), flags&3:

| Mode | Behavior |
|---|---|
| 0 | Clear; the message operand need not be a catalog index |
| 1 | Show operand&0xffff, admitted by80098178 only when subtitles are enabled |
| 2/3 | Ignored by this source consumer |

EV00 shows messages0/1/2/3 at source frames20/170/293/355 and clears at395.
The controller processes forward source frames, retains the current selection,
and clears on event cleanup. Turning subtitles on later does not retroactively
admit a show command skipped while they were off. Interactive page advance,
other message controls and other window/controller styles remain separate.

## Layout observations

The original type5 family3 event layout fixes its first baseline rather than
vertically centering the entire message. In the observed default window branch,
source position0,150 becomes viewport anchor320,330. Dimensions are20x20,
line spacing20*20/24=16.666666, and the glyph trailing-width coefficient is0.3.
Challenge family1 retains its separately verified centered rows/0.2 coefficient.
The implementation shares the layout core with explicit options for these
differences, preserving the existing challenge and Play Log comparisons.

Original80186DAC also permits a configured subtitle-window prototype to
override position/metrics. That branch and broader token families are not
established by the default-window observations here. These checks do not prove
native pixel/material/compositing parity or general cutscene completeness.

## Native component fixture

Default-off MW_TEST_MESSAGE_RECORD plus MW_TEST_MESSAGE_MESS loads the
original EV00 MESS bytes into private native storage and uses original
800980FC/800982A0/80098178 message registration/update/show APIs. The fixture
temporarily enables the profile subtitle bit only around show and restores
it immediately without saving. No actor HP, position or state is written.
Four controlled submissions12 updates apart are followed by original clear.
This is UI component evidence, not actual EV00 event playback.

event-message-v5 completed its190-second native PC sequence, exit0, released
phase and6749 presentations/host blit. Its52 rows contain65/62/41/41 glyphs
and zero after clear. tools/events/stage_message_evidence.py rejects missing
rows/empty captures and stages four snapshots with capture/catalog hashes.
All209 glyph positions, sizes, UV bounds and white RGBA agree with the Rust
layout. Position tolerance0.0003, size0.00001 and UV0.000001.

Earlier captures are not used: v1 rejected a valid MEM2 allocation under an
overly narrow fixture bound; v2-v4 completed ordinary native runs but the
fixture mistakenly invoked the page-advance reset and captured no glyphs.
The final fixture preserves the glyphs created by show/update and validates
the expected EV00 MSD/MTD format. Only the alternate build-record DLL changed.
Raw decompiled C remains external in TEMP/mw-havok-proxy.

## Runtime checks and remaining scope

mw-message-preview exercises the same asset/controller/layout code used by
playable entry. Original frame20/170 captures render65/62 glyphs and were
visually inspected. Frame395 and subtitle-off frame20 render zero. All four
bounded renderer processes exit0. For local inspection:

```powershell
$env:MW_MESSAGE_EVENT='00'
$env:MW_MESSAGE_FRAME='20'
cargo run -p mw-game --bin mw-message-preview
```

MW_MESSAGE_ENABLED=0 disables admission; M toggles the inspector's option;
Right steps its source frame. MW_MESSAGE_CAPTURE selects a bounded hidden
capture. This tool displays original text only, without the Man Darts scene.

The rebuilt playable EV21 run used an isolated subtitles=true config, skipped
at300, completed cleanup/fade/HUD admission and120 fight ticks, exit0. Source
absence checks held throughout.96 library tests (86 simulation/10 data), all
game binaries check, and the1 canonical/0 independent HP replay gate pass.

Remaining work includes configured subtitle windows, other commands/tokens,
voice/timing integration for events that actually contain MESS, full Man Darts
and B01/world admission, menu SE, complete save/main-menu flow and the other
items in NATIVE-PORT-GAPS.md. The full port goal remains active.
