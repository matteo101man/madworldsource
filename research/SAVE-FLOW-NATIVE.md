# Native empty START and isolated NAND path correction

## Rust occupied slots and copy/delete continuation

The Rust title slot menu now loads native-format save.dat and option.dat from
`%LOCALAPPDATA%/MadWorld/Rust/save` (or an explicit MW_RUST_SAVE_DIR). A missing
pair is an in-memory empty profile; malformed or partial files produce a visible
error and block operations. F5 reloads the pair. The native core's user directory
is never chosen implicitly. No records are written just by opening the menu.

mw-data::save_store preserves native bytes and writes the record/options pair
through a checksummed write-ahead journal. Interrupted publication is completed
on reopening; a lock excludes simultaneous writers, and a stale disk snapshot
is rejected. Record copying preserves every unknown byte; option copying follows
8023DD84 and preserves destination padding byte1. Delete resets the record through
the staged native reset bytes and initializes its English option profile through
the translated8017FE88 behavior. New-game and B01 admission remain unported.

Original programs80/81/83 show occupied rows and START/COPY/DELETE. Copy selects
a different destination; program82 asks for confirmation, including a distinct
overwrite prompt. DefaultNo and Back selectingNo never write. An explicitYes
commits once; selected source/DELETE highlighting remains behind the dialog.
START reports that starting a saved game is unavailable, rather than writing a
new-game flag without its required opening scene. Title BGM, original asynchronous
save/error controllers and exact transition/flash timing remain incomplete.

The owned save-occupied-v1 native run completed190seconds but showed empty slots:
the fight shortcut bypasses ordinary E00 disk loading. It is not occupied-slot
evidence. The default-off MW_TEST_SAVE_RECORDS/MW_TEST_SAVE_OPTIONS fixture imports
explicit native files into isolated test RAM after E00 admission, disables the
fresh-record reset and performs no file writes. The original widget consumers
then produce the occupied view. save-occupied-v2 completed190seconds/9144
presentations, exit0. save-delete-dialog-v1 completed190seconds/6036
presentations, exit0, with the original defaultNo DELETE dialog. Both use
dedicated native-user directories. Source observations are in
ui-save-occupied-{rows,actions}-observed.json and
ui-save-delete-{confirm,actions}-observed.json; native screenshots are under
evidence/native-save-*.png. These are component fixtures, not a natural disk-load
or completed title-to-game playthrough.

The notes below preserve the earlier empty-START investigation.

2026-11-09. Pure Rust product target; owned native core is the reference. The
original native-core-user directory was not changed. All runs used copied NANDs.

The source branch8023C290 state4 invokes8023D5E8 with callbacks8023D3F4 and
8023D560 for an empty slot. New read-only diagnostics found OptLoadGame state12,
record80622CE0, callback8023D560 after START. Native RAM contained the default
record with only flag byte44E changed to3. Its full hash matches the Rust codec's
new-game initialization, not just its visible score/time/stage fields.

The apparent UI stall was in save I/O. fs-order native logs show CreateFile's
76-byte SDK attribute buffer, with the NAND path at+6. The runtime joined that
slash-separated path to a canonical Windows user directory with the extended
prefix. Its mixed `nand\tmp/save.dat` path could not be opened by fopen. The
normal temporary-path unit test passed, while the same test with the extended
prefix failed creating option.dat. Normalizing the complete host path with
make_preferred fixed it; the extended-prefix regression now passes.

Native ui-save-fs-fixed-oracle completed180seconds/6626presentations, exit0.
It creates and opens temporary option.dat/save.dat/banner.bin, persists the
32-byte options and201024-byte record files, then requests sceneB01. Screenshot
ui-save-fs-fixed-oracle.png shows the opening city cinematic. This is neither
an observed program84 confirmation nor a title main-menu capture.

The created record on disk equals the staged reset record with byte44E=3:
SHA2564528f6695021ead5cdaa59d78d4f74766d0f2b9d0faf711871ddb860eecd0894.
Other records remain defaults. See save-new-game-native-evidence.json for the
earlier concurrent RAM observation and controller fields. No raw generated
decompilation source was copied into Rust.

Implementation boundaries: mw-data::save supports native records and creation;
mw-play still has no filesystem/save-operation or B01 scene integration. Its
START action remains unimplemented. The native path correction enables further
original observations; it does not complete the Rust title flow.

Verification logs: evidence/ui-save-fs-prefix-before.log (expected failure),
ui-save-fs-prefix-after.log (pass), ui-save-fs-fixed-oracle.log and
event-intro-staging-tests.log (71 library tests). Both Rust binaries check.
The original four native user-file hashes still equal
ui-newgame-reference-user-hashes.json. Only port/build-record was rebuilt.
