# Handoff - snapshot save/load (2026-11-08)

**Paused at the user's explicit request: "stop here and make another handoff".**
Do not start another test or continue development until the user resumes.
Continues `HANDOFF-2026-11-08-OWNED-STACKS.md` and the two earlier 2026-11-08 handoffs.
All prior constraints remain: everything new on D:, CLI only, no Computer Use,
no subagents, no commits, toolkit Git untouched, typed graphs for game logic,
verify exact process identity, no rebuild while an owned live test holds the DLL,
no blanket animation-speed multiplier. **Nothing committed.**

## State at pause

No owned game test is running. At the stop request, both final tests had already
finished normally. Verified historical PIDs 20056 (writer) and 77060 (reader)
were gone; no termination was necessary. Their runner sessions 25950 and 47571
returned exit0 and wrote final JSON. No unrelated processes were touched.

**Phases 2 through 6 have an implemented save/load path, and fresh-process
control-ready restore has passed a 15-second native GPU test.** This is not full
snapshot acceptance yet: defeat->continue from a restored checkpoint, explicit
corrupt/mismatched-file rejection tests, and repeated restore runs remain.
No victory, visible rendering, full audio or full-port completion claim.

## Latest successful evidence

- `evidence/bevy-native-snapshot-write4.json/.log`: normal-speed160s writer,
  `--motion-probe` (shared real PC bindings; no game-state fixture writes).
  Original intro completed, control-ready opened, capture succeeded, motion probe
  released, shutdown passed.4086 RTX3090/DX12 presentations, host blit complete,
  exit0. Capture payload20,531,321 bytes; write722.651ms.
- `evidence/bevy-native-snapshot-read2.json/.log`: **fresh process**,15s,
  `--from-snapshot evidence/snapshots/stage201-control-ready.bin`, no PC sequence.
  Restore678.476ms; first resumed world update at **692.851ms measured from
  native_host_run entry**, scene201, demo counter0, stop-flags word0=0.
 353 presentations, host blit complete, cooperative shutdown, exit0. Original
  event21 was not replayed. The frontend's process/GPU startup is outside that
  native-start measurement; do not claim the whole app launches in<1s.
- Restore logs `[snapshot-actors] player=815a51a0 rin=815b8d80`.
  `tools/read_native_core.py` understands that marker; it no longer requires
  original actor-creation/onInit logs to inspect a restored run.
- `evidence/native-snapshot-core-regressions.json`:9 pass - hardware_events,
  ipc_transport, di_async, host_fiber, thread_reuse, native_unwind, irq_snapshot,
  gx_transport, ai_clock. These precede the final capture gate/actor/clock
  instrumentation changes; do not describe them as rerun on the final build.
- `evidence/native-snapshot-platform-tests2.log`:23 Rust tests pass, including
  actual borrowed MEM1/MEM2 + hardware state roundtrip and controller/motion state.
- `evidence/native-snapshot-difftest.log`:5280 eligible functions,
 11711/11711 terminating Unicorn differential cases match. Final actor-observer
  and capture-diagnostic edits came afterward; arithmetic was unchanged.
- `evidence/native-snapshot-embedded.log`:3 lifecycle cases pass (stop, missing
  partition, missing platform symbol, all reject reuse), before final diagnostics.
- Final core build: `evidence/native-snapshot-actor-build.log`, exit0.
  Frontend build: `evidence/bevy-native-snapshot-build.log`, exit0.

## Current checkpoint and exact host

`evidence/snapshots/stage201-control-ready.bin` is the valid control-ready image.
It contains owned game memory; keep it local on D:, out of Git.
The image is deliberately tied to exact binary hashes and addresses. A core or
frontend rebuild invalidates it; recapture after rebuilding, no silent fallback.

Current test executable:
`D:/GDecomp/build-cache/bevy-0.20/debug/deps/madworld_native-ce2dbea931a076cb.exe`.
The canonical runner build log `evidence/bevy-native-gpu-test-build.log` has its
single-line path (PowerShell-wrapped cargo stderr still breaks the old parser).

Checkpoint header facts:

```json
{
  "file": "D:\\Games\\gamesfordecomp\\ports\\madworld-rinrin\\evidence\\snapshots\\stage201-control-ready.bin",
  "fileBytes": 20531545,
  "sha256": "15820f0e95b4f9cb9003f053c0fec79a04fdf307338c2f3470e19fe17784220d",
  "magic": "0x3150414e53574d",
  "version": 1,
  "clockScale": 1,
  "coreBase": "0x1a0000000",
  "coreImageSize": 49848320,
  "coreSha256": "2f4f4450cc3acbd2189552909dce6dbfed266066014b38b103f4e2ac4078469b",
  "platformBase": "0x1b0000000",
  "platformImageSize": 60125184,
  "platformSha256": "9039d21a14e3fb8b71c23a72d6a1d8cbfd52f996ff31a2a567fe3244e18aee92",
  "dolSha1": "b1a42fd6acb9dfe076b583b03b610ca44a65f532",
  "guestBase": "0x20000000000",
  "payloadBytes": 20531321
}
```

The platform module for this image is the **statically linked Rust test EXE**,
resolved through mw_native_bind_platform. It is not the separate platform DLL.
The standalone `D:/GDecomp/build-cache/madworld-platform/debug/mw_native_platform.dll`
has NOT been rebuilt for the new exports; `cargo test --lib` built the Rust tests,
not that cdylib. Build it before attempting DLL-hosted snapshots and create a
separate checkpoint for that host. Normal ABI remains1, with additional symbols.

Existing source/binary identity manifests were **not refreshed** after this work.
The source manifest also needs new files added (snapshot.h/.cpp, hardware_events
regression, the Rust snapshot module and both build.rs files, plus affected
platform/Gecko sources). Header hashes above identify the validated image.
Do not refresh only a subset and imply the old manifest lists cover all changes.

## Implementation map

### Phase 2: typed events

`runtime/os/os.h`, `os/interrupts.cpp`: OSEventKind and OSEventData replace all
std::function event closures. Registered process-local callbacks stay outside
snapshots. Deadlines and monotonic sequence numbers retain equal-time ordering.
Kinds: AI, DSP, VI, DI read(reply,len,out,64-bit offset), IPC ACK(latched request),
Bluetooth(numeric state id). Device files register handlers and keep the original
rearm intervals. Weak Bluetooth lifetime is now an id->weak_ptr registry; an
expired event still does nothing. `tests/hardware_events.cpp` proves ordering,
latched64-bit values, rearming and event-stream roundtrip; IPC/DI tests were
adapted to execute typed payloads.

### Fixed addresses and stack/runtime handling

- Core `/DYNAMICBASE:NO /BASE:0x1A0000000`.
- Rust host test bin and platform cdylib build.rs use `/DYNAMICBASE:NO` and
  `/BASE:0x1B0000000`. PE flags were verified on the actual test EXE/core DLL.
- Snapshot mode guest memory window at0x20000000000; ordinary runs retain the
  prior OS-placed window. Owned fiber stacks remain phase1's fixed arena.
- `os/threads.cpp`: GuestThread/PPCContext/PPCTrace storage is a fixed256-record
  array in the core DLL, replacing heap metadata pointers that were held on stacks.
  Maps/IRQ vectors/dead-thread lists rebuild explicitly; all thread contexts and
  live stack bytes/saved RSPs serialize.
- `os/host_fiber.cpp`: restores recorded slots exactly or refuses occupied slots;
  rejects active host C++ exceptions and walks each saved stack with Windows
  unwind metadata, refusing foreign-code frames. Recreates static MSVC CRT FLS
  blocks, preserves errno and saved FP controls, never copies old PTD heap pointers.
  Private CRT accessors remain tied to this pinned static MSVC runtime.
- `game/native_unwind.cpp`: copies the matching continuation decision and leaves
  catch before invoking f_8021F3A4. A long-running task continuation must not keep
  the caught heap exception alive. Regression now checks current_exception is null
  in the continuation and still verifies native registers/SP/destructors.
- `app/embedded.cpp` custom `mw_native_dll_entry` selects a reproducible /GS
  cookie ONLY for explicitly set MW_TEST_SNAPSHOT_READ/WRITE, **before** CRT
  initialization/encoded-pointer setup, then calls _DllMainCRTStartup. Ordinary
  runs retain normal CRT cookie initialization. Changing cookie in DllMain was
  too late and crashed startup; do not reintroduce that approach.
- Embedded boot HostFiber metadata is now static; the supervisor/root stack is
  never captured. Main clears its DOL vector before entering the persistent
  game boot frame so snapshots do not retain a live DOL heap allocation.

### Explicit machine state and restore

`cpu/snapshot.h/.cpp`: bounded StateArchive, versioned header, hashes with BCrypt,
atomic publish through temporary file + MoveFileEx. Header checks version,
module SHA256/base/image-size, DOL SHA1, PC prompt hash, guest base, cookie and
clock scale; payload SHA256 checked before restoring any machine state.

Stream includes Rust hardware/guest banks, C++ locked cache, CPU interrupt-poll
phase, fallback MMIO registers, AI/VI/EXI, GX CP/PE/gather/FIFO and queued data,
IPC replies, DI transfer timeline, IOS fd/path/device state, NAND file mode/offset
(reopen), Bluetooth pending requests and controller protocol/motion state,
OS event queue, threads/stacks. Timebase is rebased after serialization to exclude
snapshot I/O. Original RSO observer function pointers are rebound before resume.

`cpu/embed_host.cpp`: checkpoint parks the guest onto its supervisor; only that
supervisor writes/overwrites guest stacks. Read is requested after host memory /
devices / boot metadata init and before original DOL entry executes. Loading
replaces those parked stacks and resumes the captured current thread.

`game/trace_boot.cpp`: first capture requires boss setup, intro demo observed,
demo counter0, game stop-flags word0=0, live Jack/Rin identities and Jack HP>0.
The earlier zero-counter-only gate caught a transient before control-ready.
`trace_objects.cpp` records Jack; Rin bypasses that actor creation wrapper, so
`cpu/rso.cpp` records her from the existing original move observer.
Restored setup guards prevent a second diagnostic BossSw/first-world override.
`[snapshot-world]` reports first resumed scene/demo/native-start wall time.

### Rust hardware and runner

`native-platform/src/snapshot.rs`: System save/load, Bluetooth/controller save/load,
and returned-byte allocation/free C ABI; host resolver exposes these symbols.
Motion pulse token/step/delay serialize explicitly. Gecko's composite save/load
already existed; `gecko/src/mmio.rs` now serializes **borrowed native banks**,
not its detached backing vectors. CPU/JIT execution on borrowed banks is still
forbidden. Hardware load invalidates renderer caches and resets EFB; dirty pages
are refreshed. Fresh frontend audio sinks start empty. This is state-test
infrastructure, not visual/audio continuity or pixel equivalence evidence.

`tools/run_owned_gpu_test.py`: --write-snapshot and --from-snapshot; D:-only paths,
mutually exclusive, capture/restore marker required for runner success. Restored
runs can be sampled without onInit. Same existing PC bindings/test code.
The preexisting --sustain final-phase==released assertion remains incompatible
with ending while sustained/dead-confirm/finish-confirm; prefer a non-sustained
125s sequence for the next defeat/retry acceptance run.

## Next steps when resumed

1. Exercise verified defeat->continue from this restored image with default-off
   HP1 fixture and real Enter pulses. Example (no new builds needed first):

```powershell
$env:MW_TEST_PLAYER_HP1='1'
$env:MW_GPU_TEST_PC_CONTINUE_CONFIRM='1'
$env:MW_TRACE_RPN='1'
python tools/run_owned_gpu_test.py --seconds 125 --label bevy-native-snapshot-continue --pc-steering --from-snapshot evidence/snapshots/stage201-control-ready.bin
```

   Require native death95, continue window, confirm, restored HP100, state94
   respawn, ordinary fight and balanced demo nesting. Inspect read-only actor
   history and RPN logs; a GPU pass alone is insufficient. Fixture must remain
   explicit; no natural-death claim.
2. Test wrong module/header and corrupted payload rejection in the SAME test EXE,
   with separate copied bad files; require rejection before any resumed world.
   These paths exist but were not exercised. Do not overwrite the valid image.
3. Repeat fresh-process restore and report native-start->first-world timing;
   first valid control-ready read is692.851ms. Keep process/GPU init time separate.
4. Run final focused regressions after the last observer/diagnostic edits, refresh
   complete source/binary manifests, reconcile research design/ledger evidence.
   Do not rebuild the core during an owned live test; any rebuild invalidates the
   current checkpoint even for minor changes.
5. Then resume the earlier end-state work. Victory still awaits motion input at
   Jack68/13/selector3; Enter starts but does not drive it. See PLAYER-DEATH-API
   and player-rin-finish-pcode graph/query. This turn did no new victory work.

## Earlier attempts retained

- write1: DLL startup c0000005 at guard dispatch after changing cookie too late.
- write2/read1: portable transient (not control-ready) image;19,907,679-byte payload,
  write714.524ms/read612.633ms; retained as stage201-transient-v1.bin. It is tied to
  an earlier core hash and will now be rejected. Do not use for acceptance.
- write3: exact verified owned PID65556 intentionally stopped to add Rin's actual
  RSO observer to capture identity tracking; partial evidence, not a pass.
- write4/read2: authoritative corrected control-ready writer/restore evidence above.

No further tests or development were started after the user asked to stop.

## Step 1 result: defeat->continue from restored checkpoint — PASS (2026-11-08)

Run by a side session with the exact step-1 command (HP1 fixture, continue-confirm, RPN
trace, `--pc-steering --from-snapshot`, no `--sustain`). Owned PID 53900 exited normally;
exit0, 1127 RTX3090/DX12 presentations, host blit complete. Core hash unchanged
(`2f4f4450…`); no rebuilds, no source changes. Evidence:
`evidence/bevy-native-snapshot-continue.json/.log`, `-state1..8.json`, `-final-state.json`,
`-feedback*.json(l)`, `-runner-stdout.log`, summary `-acceptance.json`.
Times are control-ready seconds (reader sample times are ~16.7s later, process-relative).

| Requirement | Result | Evidence |
|---|---|---|
| Fresh-process restore | pass | `[snapshot] restored … ms=4781.052`; `[snapshot-world] scene=201 demo=0`; event21 not replayed |
| Native death 95 | pass | fixture HP100->1 on first resumed update; ~64.5s original hit -> `onPlayerDead` (8006a3c4), `onDead` (802c11b4), `onPlayerDead` (802c11dc); samples HP0 state95 phase1 -> 2 |
| Continue window | pass | demo begin 0->-1 caller 80127e50; `[continue-ui] window mode=0 widget=90ec7480 continues=2` |
| Confirm | pass | dead-confirm Enter pulses (pad-down 0x10 edges); `onContinueRestart` broadcast (80128010) ~85.5s |
| Restored HP100 | pass | player 100/100 at first post-restart sample |
| State94 respawn | pass | state 94 phase1 for ~12s of samples |
| Ordinary fight resumes | pass (short) | state 39 at HP100 with sustained-combat inputs until release at 110.0s; Rin states 3->11->14, distance 2.5-4.7. Only ~12s of post-respawn combat; no further HP exchange |
| Balanced demo nesting | pass | 0->-1 (80127e50), -1->-2 (80086ea0), -2->-1 (8005a664), -1->0 (80086f04); reader: demo -1 during window, final demo 0, scene speed back to 1.0 |

The post-death sequence (RPN senders/callers, continue-ui line, demo begin/end order,
throw/unwind to 8021f3a4) is identical to the earlier boot-path `bevy-native-death-fixture`
run. Fixture is explicit; no natural-death claim.

Caveats: restore took 4781ms vs 678ms in read2 — the host was busy with a parallel session
and an unrelated `madworld-native.exe`, so this is not a timing data point (step 3 should
re-measure on a quiet host). The harness sent one dead-confirm Enter packet at control-ready
t=0 because steering `last_hp` defaults to 0 before the first sample; the first resumed update
showed pad-down 0 and there was no observed effect, but a harness fix (gate on a real sample)
would need a frontend rebuild, which invalidates the checkpoint.

## Results - steps 2 and 3 (side session, 2026-11-08)

No source edits, no rebuilds, no cargo. Same test EXE
`D:/GDecomp/build-cache/bevy-0.20/debug/deps/madworld_native-ce2dbea931a076cb.exe`
(runner-resolved) and core DLL SHA256 `2f4f4450...8469b` (matches header).
`stage201-control-ready.bin` untouched: SHA256 still `15820f0e...4220d` after all runs.

### Step 2 - rejection: 17/17 pass

Copies in `evidence/snapshots/bad/reject-*.bin` (manifest with per-file SHA256 and
exact corruption: `evidence/bevy-native-snapshot-reject-manifest.json`; header is 224 bytes,
offsets from `cpu/snapshot.cpp` `Header`, validated against the real image first).
Summary: `evidence/bevy-native-snapshot-reject-summary.json`; per-case
`evidence/bevy-native-snapshot-reject-<case>.json/.log`.

| Cases | Rejection line |
|---|---|
| magic, version 1->2, clock-scale 1->2, core base/size/SHA, platform base/SHA, DOL SHA1, prompt hash, guest base, /GS cookie | `Snapshot identity mismatch (version/modules/bases/DOL/prompts/RAM/cookie/clock)` |
| header payload_size+1; file truncated by 4096 bytes | `Snapshot payload size mismatch` |
| file truncated to 100 bytes | `Snapshot file size invalid` |
| payload middle byte flipped; header payload hash flipped | `Snapshot payload checksum mismatch` |

Each: exactly one `[snapshot] rejected:` line right after `[host] booting`; no
`[snapshot] restored`, `[snapshot-actors]` or `[snapshot-world]`; no access violation.
`native_host_run` breaks out, runs normal shutdown; frontend status becomes
"Unable to start MadWorld"; the GPU test's own assertion (native_gpu.rs:121) then fails, so
exit code is 101 (expected test-harness failure, not a crash). ~12 s per case.
Gaps / possible follow-ups (not bugs): the identity check is one combined message, so the
log does not name which field mismatched; the specific error string goes to stderr and
`mw_native_last_error`, but the frontend status only shows the generic text. A rehashed
structurally-bad payload (StateArchive tag/truncation inside `machine()`) was NOT tested:
it would fail after partial state restore, outside the "before any machine state" guarantee.
Missing-file is blocked earlier by the runner's `is_file` assert, so it was not tested in core.

### Step 3 - repeated fresh-process restore: 4/4 pass

`evidence/bevy-native-snapshot-read3..read6.json/.log`, combined in
`evidence/bevy-native-snapshot-read3plus-timing.json`. 15 s, no PC sequence, run only with
no `madworld*` process present (checked before each run; parallel continue test had ended).

| Run | restore ms | native_host_run entry -> first resumed world ms | presentations |
|---|---|---|---|
| read2 (earlier) | 678.476 | 692.851 | 353 |
| read3 | 725.252 | 736.921 | 300 |
| read4 | 718.643 | 731.887 | 290 |
| read5 | 709.156 | 722.132 | 288 |
| read6 | 693.924 | 709.593 | 280 |

read3-6 first-world: min 709.593, max 736.921, mean ~725 ms. Restore itself (file read +
SHA256 + machine load) is ~97% of it; the rest of boot to first world is 11.7-15.7 ms. All: scene 201, demo 0,
event21 not replayed, RTX3090/DX12, host blit complete, `test result: ok`, exit 0.
Process start, Rust harness and wgpu/GPU device init are outside this measurement; full
runner wall time was ~24.4-25.6 s for a 15 s bounded test incl. shutdown. Do not claim
whole-app <1 s launch. Presentation counts vary 280-353 and are not a benchmark.
