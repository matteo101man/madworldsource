# Native hardware adapter

GPL-3.0-only. Gecko is pinned at91950c6364d8a464d6eb0a666dbf3a6dec24d5bb with its pinned chipi-spec submodule. Full license remains at ../tools/gecko-reference/LICENSE. Free Dolphin DSP ROM/coefficient replacements are open-source replacements, not console dumps; provenance and hashes are in ../research/dsp-upstream.json.

Build from this D directory with CARGO_TARGET_DIR=D:/GDecomp/build-cache/madworld-platform and cargo test --locked / cargo build --locked. MW_PLATFORM_DLL points the original native CPU core to debug/mw_native_platform.dll. All fourteen Rust hardware tests pass (evidence/native-motion-tests.log).

Original PPC gameplay is statically recompiled outside this crate. No Gecko CPU or JIT executes. Shared MEM1/MEM2 banks and dirty-generation arrays remain caller-owned. Every FFI call requires exclusive single-threaded access; the caller accesses guest RAM only between calls. Do not use CPU, JIT or save-state APIs on this hardware instance.

DSP executes the free IROM and original uploaded microcode; USB V0 carries explicit virtual remote/Nunchuk packets. GX consumes already framed commands and uses original live RAM. C++ native stores and Rust DSP writes share page-generation tracking, preserving cache invalidation without dirtying88MB on every draw.

MW_AUDIO_WAV captures signed16 stereo AID DMA PCM, with adjacent .audio.json counts. Stage201 produced nonzero32kHz audio; evidence/status.json records counts. The bounded capture supports14.4million stereo frames. Midstream sample-rate changes fail explicitly pending segmented capture. Native big-endian R,L is converted to host L,R. Periodic flushing survives the diagnostic watchdog except for the final pending chunk. No watchdog thread enters Rust. Bevy audio device output and original mix/timing comparison remain unfinished.

VI register writes now mirror into the Rust video interface. Deadline-driven half-line events latch the current XFB at active-field start and emit presentation at active-field end. The C++ CPU-facing VI reads/interrupts still use the existing simplified retrace model; their timing is not established as equivalent. A synthetic test verifies dimensions, raw guest YUV scanout, a mid-field buffer flip, next-field visibility, and unchanged guest memory. A bounded native stage201 run emitted640x480 GX-copy presentations without a reported crash. No rendered pixel or Bevy display claim follows from these events.

MW_GX_AUDIT records bounded source shader inputs and up to8 presentation records, with optional scene/TEV/perspective filters. Material/light audits contain finite native geometry but do not prove actor identity or pixel equivalence. Feature native-render builds the renderer against wgpu30.0.1, compatible with Bevy0.20; the Bevy device/display integration is still pending.

The host module installs rendering and audio sinks on the original CPU worker before mw_dsp_create. resolve_symbol exposes the same hardware C ABI directly from a Rust executable, avoiding a second platform DLL instance. Explicit controller packets are published atomically; only that worker uses host input. Standalone callers retain the input-file path. Tests cover actual AID DMA reaching the installed stereo sink and original GX triangle delivery. The standalone hardware DLL was rebuilt successfully (evidence/native-host-platform-build.log).

A shader compilation stall exposed the old 65,536-total-event catch-up limit. The adapter now bounds repeated events at one deadline, allowing valid chronological catch-up without dropping events. A 70,000-half-line test compares one late poll with timely polls: all7,000 raw frames and final scheduler state match. This fixes an adapter failure; original clock/IRQ/audio synchronization equivalence remains unverified.

The native GX renderer now passes a headless DX12 startup/host-blit test after replacing invalid verbose shader labels. This exercises the original CPU and real GPU backend; it does not exercise the Bevy render schedule/window or prove pixel fidelity. The frontend explicitly roots backend caches on D.

The optional PC motion bridge uses the formerly reserved packet bytes6/7 for sequenced directional events. Two independent pulses drive remote and Nunchuk acceleration through the original HID transport. Nunchuk sensor bits/button preservation/range saturation pass2 focused tests. Run cargo test with this manifest and -p gecko --lib native_motion_wire_tests (dependency features already disable JIT). The sparse upstream workspace is not required. Native action equivalence remains unverified; see research/PC-CONTROLS.md.


Held PC struggle uses profile9 on both arm channels: three-axis +/-1.45 wave, Z gravity retained, repeated while its token is active and canceled on release. Other motion profiles keep their finite-event behavior. Current hardware suite has15 passing tests, including70,000 held samples/cancel/restart. The hardware DLL and Bevy frontend are rebuilt; complete native competition behavior remains unverified.


Remote audio now has proper speaker state/status, conditional ACKs, isolated speaker/extension registers, gated Yamaha ADPCM/PCM decoding and a bounded mono-to-PC-stereo mixer. AID passes unchanged when the remote lane is silent. The native USB fixture verifies decoded/resampled output. Hardware suite21 and Gecko codec/protocol/snapshot suite8 pass. Speaker snapshots use version5 with version1..4 defaults. MW_AUDIO_WAV captures the PC stereo output (AID plus any active decoded remote lane). Real game initialization configures6kHz/volume88 then mute; continuous0x18 packets do not imply audible output. See ../research/REMOTE-SPEAKER-API.md for exact limits/reference provenance.
