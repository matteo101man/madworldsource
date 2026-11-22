# Control-ready snapshot/restore for the native core (design)

Goal: start any test at stage-201 control-ready in well under a second instead of ≈95 s of boot + intro, without changing game logic. Test infrastructure only; the production frontend keeps booting the original way.

## Why boot cannot simply be sped up

`MW_TEST_TIME_SCALE` (test-only guest clock scaling) tops out at ≈2× during boot and falls back to ≈1× once the stage is live: `mw_dsp_tick` must resolve every elapsed hardware-scheduler deadline and the DSP interpreter runs only ≈2–3× real time (see PLAYER-DEATH-API.md, "Test acceleration"). The remaining option is to capture the whole machine once and restore it.

## State inventory

| Layer | Where | Snapshot form |
|---|---|---|
| Guest RAM | `uint8_t* m`: MEM1 24 MB, MEM2 64 MB (+ any host-side guest-visible buffers) | raw bytes (compress on disk) |
| Guest threads | `os/threads.cpp`: `g_threads` (addr → `GuestThread{fiber, PPCContext*, PPCTrace*, entry, exit_lr, exited, irq_frames}`), `g_cur`, `g_boot`, `g_dead` | per thread: PPCContext, PPCTrace, flags, **used stack bytes + saved host context** |
| Host stacks | Win32 fibers today (`CreateFiberEx`, 8 MB stacks at OS-chosen addresses) | must become **own stacks at fixed virtual addresses** with an own x64 context switch; the boot thread also moves onto such a stack in `os_threads_init` |
| Native code addresses in stacks | recompiled functions and runtime live in `madworld_core.dll`; platform calls live in `mw_native_platform.dll` | both DLLs linked with a fixed preferred base and no ASLR (`/DYNAMICBASE:NO`, `/BASE`); restore verifies module bases and sizes match the snapshot header or refuses |
| OS/interrupt model | `os/interrupts.cpp`: PI cause/mask, HW flags/mask, decrementer armed/deadline, `g_events` (timebase deadline + `std::function`) | closures replaced by a tagged enum + payload: periodic `ai::update`, `dsp::update`, `vi::retrace` (re-armed), `di` reply `{reply, off, len, out}`, `ipc` request id, bluetooth poll (re-armed) |
| Time base | `ppc_timebase()` = host clock since start | snapshot stores the guest TB at capture; restore sets an offset so TB continues from that value |
| VI / AI / DI / IPC / FS / ES / STM / bluetooth (C++ HLE) | `hw/vi.cpp`, `hw/ai.cpp` (`mwaudio::Clock`), `ios/*.cpp` | POD register state + queues; NAND file handles re-opened by path/offset; bluetooth controller state POD |
| GX command processor (C++) | `gx/device.h`: `cp[]`, `pe[]`, gather buffer, fifo pending state, counters | POD |
| Rust hardware (`gecko::System<WII>`) | DSP, scheduler, MMIO, VI, PE, PI, DI, SI, AI, MI, Starlet/Hollywood | gecko already has `savestate::{StateWriter,StateReader}` with `save_state/load_state` on dsp, dvd, exi, gx, scheduler, mmio, starlet, hollywood ipc/di/usb/wiimote; add a composite `System::save_state/load_state` and FFI `mw_platform_save_state/load_state` |
| Rust GPU (`backend-wgpu` RenderSink) | textures, EFB, pipelines | not snapshotted: GPU caches are rebuilt lazily from guest memory; EFB/XFB contents are lost at restore (acceptable for state tests, flagged in the result) |
| Audio sink | ring buffers | reset at restore |
| Frontend (Bevy) | input, presentation | stateless with respect to the guest; restarts normally |

## Capture point

The scene-update hook (`f_80215160`) after `MW_BOOT_BOSS` has fired and the intro demo nesting (`0x80604020`) first returns to 0, i.e. control-ready, with the player and Rin in ordinary states. At that moment every other guest thread is parked in the HLE scheduler and the running thread's native stack is runtime + recompiled frames only (no CRT or platform frames), which is what makes stack capture tractable. Trigger: `MW_TEST_SNAPSHOT_WRITE=<path>`; restore: `MW_TEST_SNAPSHOT_READ=<path>` taken at `native_host_start` before the DOL runs.

## Phases

1. **Own fibers** — replace Win32 fibers with an x64 context switch (`fiber_x64.asm`: save rsp/rbx/rbp/rsi/rdi/r12–r15/xmm6–15, TEB stack limits) on `VirtualAlloc`ed stacks at fixed bases (one 8 MB slot per guest thread slot). Acceptance: existing `thread_reuse`, `irq_snapshot`, `native_unwind` regression exes pass; a full boot to control-ready behaves as before.
2. **Serializable OS events** — tagged events instead of `std::function`. Acceptance: boot identical; `difftest` unchanged.
3. **Fixed module bases** — link flags + restore-time verification. Acceptance: bases logged and stable across launches.
4. **C++ device state save/load** — VI/AI/DI/IPC/FS/ES/STM/bluetooth/GX-CP PODs and queues; guest RAM; threads with stacks; TB offset.
5. **Rust `System` composite save/load + FFI**; GPU caches invalidated on restore.
6. **Harness** — runner flag `--from-snapshot`, writer run produces `evidence/snapshots/stage201-control-ready.bin` (kept on D:, not in Git); fixtures (`MW_TEST_PLAYER_HP1`, `MW_TEST_RIN_FINISH_WAIT`, …) work immediately after restore.

Acceptance for the whole feature: a restored run reaches the first `[world] update` within 1 s wall, the defeat→continue test reproduces its verified sequence from the snapshot, and the snapshot header records DOL SHA-1, core/platform DLL hashes and bases.

## Risks

- Any host pointer stored in guest-visible or runtime structures that is not covered by the inventory (search for raw pointers in HLE state before phase 4).
- CRT/kernel frames on a captured stack (avoid by capturing only at the hook with no I/O in progress).
- GPU-side state loss (EFB) is visible to rendering tests; state/logic tests are unaffected.

## 2026-11-08 continuation: owned stacks

Phase 1 is implemented in `runtime/os/host_fiber.{h,cpp}` and `fiber_x64.asm`.
MSVC builds assemble/link the switch. Guest and boot stacks use 256 fixed 8 MiB
slots from 0x40000000000, each with an inaccessible bottom 64 KiB. The embedded
supervisor retains its original owner stack. The CLI uses `host_fiber_call` for
boot; `os_threads_init` refuses an unowned stack. Entry wrappers own boot-stack
cleanup; scheduler shutdown owns worker cleanup. Retired slots can be reused.

Fixed the unfinished assembly's misaligned movaps accesses and initial RSP.
The switch uses movdqu, and the prepared frame matches its layout. MXCSR/x87
control words inherit the creator; TEB bounds and DeallocationStack follow the stack.

The first full boot crashed at `_IsExceptionObjectToBeDestroyed` (c0000005,
bevy-native-owned-fibers-boot): MSVC's static CRT exception chain was shared
between stacks. The implementation now switches independent VCRuntime/UCRT FLS
blocks. Their slots are discovered once by identity against the static runtimes'
own PTD accessors; no guessed PTD layout. The build requires cl/clang-cl and the
static MSVC CRT plus ml64 (verified with MSVC 14.39). GNU drivers now fail explicitly.

Final regression evidence: `native-owned-fiber-final-regressions.json` covers
512 switches from active catch blocks, independent errno/FP state, TEB bounds,
128 KiB stack probes, fixed slot reuse, exception cleanup, SDK detached/joinable
thread reuse, native task unwind and interrupts. All three embedded lifecycle
cases pass (`native-owned-fiber-embedded2.log`). Full-boot evidence is separate.

This is not snapshot support. Phase 4 must handle GuestThread, PPCContext/Trace,
vectors, CRT PTD allocations, exception chains and TLS pointers. Fixed stacks alone
do not preserve those heap pointers across processes. Rebind FLS indices per
process and reserve recorded stack slots exactly or refuse restore. Other modules'
FLS state remains uninventoried. No snapshot writer/reader is implemented.

Correction to the inventory: Gecko's composite System::save_state/load_state
already exists in savestate.rs:247/287; phase 5 needs FFI and cache handling.


Phase-1 boot acceptance: bevy-native-owned-fibers-boot2 passed 240 s at normal
speed, original intro and control-ready combat, full PC sequence release, cooperative
shutdown, 6031 RTX3090/DX12 presentations and host blit. Exit0; no window/readback.
Final build changed only assembly formatting/comments and a compile-time layout
assertion after that run; final four regressions and embedded lifecycle cases pass.
See native-owned-fibers-build-final.log and native-owned-fiber-embedded-final.log.
