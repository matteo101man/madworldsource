# GX HLE plan: native renderer at the GX SDK boundary

Goal (user, 2026-11-08): rendering is native Rust/Bevy, not an emulated GPU fed by
an emulation thread. The game's GX SDK calls drive a native wgpu renderer directly.
This file is the inventory and staged plan; nothing below is implemented yet except
stage 0.

## Where rendering goes today

1. Game code calls GX SDK functions (recompiled PPC) or writes GX commands inline.
2. Every 1–4 byte store to the write-gather pipe `0xCC008000` is an MMIO dispatch
   into `runtime/gx/device.cpp` (`gx.port`). The C++ `mwgx::Fifo` (`gx/fifo.h`)
   frames commands and flattens display lists (`0x40` is never forwarded).
3. Each framed command crosses into Rust (`mw_gx_command`), where gecko's
   `GraphicsProcessor` parses it again into emulated CP/XF/BP registers.
4. Draws go to the backend-wgpu worker thread. Since stage 0, vertex decode,
   transform and texgen run in a WGSL compute pass (see GPU-RENDERER.md).

So each GX call is encoded to FIFO bytes, dispatched store by store, re-framed, and
decoded twice before the renderer sees it. HLE removes steps 2–3 for SDK calls.

## Inventory (from `tools/ghidra-functions.tsv` and `port/generated/recomp_*.cpp`)

- The GX library is one contiguous block, `0x8036F6C8`–`0x80376A88`
  (`GXClearGPMetric`): 173 functions. Ghidra named 14; all 173 are now named in
  `gx-sdk-functions.json` (158 high confidence, 15 medium; summary in
  `gx-sdk-functions.md`). The call sequence inside `__GXInitGX` pins about 100 names
  directly. `GXDrawDone` has `GXSetDrawDone`/`GXWaitDrawDone` inlined, and unused SDK
  functions (e.g. `GXPeekZ`, `GXGetVtxDesc`) were dropped by the linker.
- Game code outside the block calls 123 distinct GX functions from 2,912 call sites.
  The busiest (calls, distinct callers): GXSetTevOrder `0x80375414` 187/34,
  GXSetVtxAttrFmt `0x803715E0` 161/43, GXSetTevColorIn `0x80374FA8` 160/25,
  GXSetTevColorOp `0x80375028` 154/25, GXSetTevAlphaIn `0x80374FE8` 151/22,
  GXSetTevAlphaOp `0x80375080` 144/22, GXSetVtxDesc `0x8037116C` 141/43,
  GXSetTexCoordGen2 `0x80371A14` 106/24, GXSetTevSwapMode `0x80375298` 104/26.
  GXBegin `0x80372794` (45 sites): callers pass the primitive (`0x98` = triangle
  strip) and then write vertices inline.
- **55 game functions write `0xCC008000` directly** (657 store sites): immediate-mode
  vertices after `GXBegin` (inlined `GXPosition3f32` and friends) and possibly raw
  register writes. No SDK hook sees these. They stay on a native command-stream path
  (stage 4).
- Display lists are called through the SDK (`GXCallDisplayList`) and flattened by the
  C++ FIFO today. Their contents are prebuilt GX command streams inside model files.

## Design

One native, ordered command list per frame, produced on the game thread and consumed
by the renderer thread:

- **Typed SDK commands.** A hooked SDK function appends a typed record (e.g.
  `SetTevColorIn{stage, a, b, c, d}`, `LoadPosMtx{index, m}`, `SetVtxDesc{...}`) and
  returns. No FIFO bytes, no MMIO, no emulated registers on the game thread.
- **Command-stream records.** Display lists and inline WGPIPE writes still contain raw
  GX commands, so the native renderer keeps a stream parser for BP/CP/XF/draw opcodes
  (gecko-derived, renderer side). It runs off the game thread and writes the same
  native state that SDK commands write.
- **Ordering.** Hooked calls and inline WGPIPE bytes interleave in game order, so both
  append to the same list. A hook can't take effect before earlier FIFO bytes.
- **Sync points** (game thread waits for the renderer): `GXDrawDone`/`GXSetDrawSync`
  tokens, PE finish, `GXPeekZ`/`GXPeekARGB`, copies the game then reads with the CPU,
  and the bounding box. Everything else is fire-and-forget.
- **Fallback.** `MW_GX_HLE=legacy` keeps today's FIFO path. `MW_GX_HLE=compare` runs
  the original function as well, re-encodes the HLE record to FIFO bytes, and logs any
  byte difference. That's the per-function correctness check before an XFB A/B.

## Stages (each ends compiling, with a working game and an XFB A/B)

0. **Done:** GPU vertex decode (raw FIFO vertices → WGSL compute → shared vertex
   buffer). Validated against the CPU decode at 0 mismatches over 6.9e9 words.
   Internal resolution: `MW_INTERNAL_RES=1..4|auto` (EFB/XFB scale already exists).
1. **Identify the SDK** (naming done; hook citations pending). Classify the functions from their recompiled bodies.
   Each GX setter writes a recognizable constant pattern to the pipe (`0x61` + BP
   register id, `0x10` + XF address, `0x08` + CP register) or to `__GXData` shadow
   fields. Match that, plus RVL SDK source-file order and sizes, to names. Output:
   `research/gx-sdk-functions.json` (address, name, evidence), and every hook in
   `recomp/hle.txt` cited to it. Prioritized by call count above.
2. **State HLE.** Hook state setters (vertex format/desc, matrices, TEV, blend/z/alpha,
   texture objects/TLUTs, channels/lights, texgens, viewport/scissor/cull, indirect).
   The renderer's state becomes Rust structs set by records, not register images.
   `compare` mode must show zero byte differences before each group is defaulted.
3. **Display-list cache.** Hook `GXCallDisplayList`. Key = (address, size, dirty-page
   generation of its pages; content hash only when the generation moved). On a miss,
   parse once into cached GPU vertex/index buffers (model-space attributes) plus a
   state-record list. Draw from the cache every frame. Transform moves from the
   decode compute pass to the vertex shader, reading matrices from a storage buffer,
   so cached geometry is reused across frames. Arrays referenced by index are cached
   by the same page-generation rule (skinned meshes use the matrix palette, not
   rewritten positions; verify per model).
4. **Immediate mode.** `GXBegin`/`GXEnd` hooks open and close a native batch. The 55
   inline writers keep writing `0xCC008000`, but the MMIO handler appends bytes
   straight into the batch (one copy, no framing, no gecko register parse). Hot
   writers (UI, effects) can be replaced by native functions later, one by one.
5. **Copies and readback as render-to-texture.** `GXCopyDisp` → present target;
   `GXCopyTex` → GPU texture keyed by destination address, with RAM written back only
   when the game's CPU reads it (backend-wgpu already defers EFB writebacks). Depth
   copies, `GXPeekZ`/`GXPeekARGB` are explicit sync points.
6. **Remove the emulated GP** from the default path once stages 2–5 are defaulted.
   Keep it behind `MW_GX_HLE=legacy` for regression A/B.

Resolution scaling is native throughout: render targets are window-sized (or N×), and
copies scale with them. The color pack keeps working because texture loads still go
through the renderer's texture cache (`LoadTexture` → xxhash64 → pack variant).

## Risks

- Naming is done, but a wrong name still means wrong state, so `compare` mode gates
  every hook. The 15 medium-confidence names are mostly FIFO/interrupt internals the
  game doesn't call.
- Inline raw register writes in game code bypass the SDK. Stage 4 keeps a parser for
  them; "no command parsing anywhere" isn't reachable without rewriting those 55
  functions.
- Display-list contents can be patched at runtime (e.g. material colors). The
  page-generation key catches CPU writes through tracked stores; untracked native
  writes force a recheck, as `mw_gx_command` already does for textures.

## Stage 2 compare results (2026-11-08)

Hooks are in `port/runtime/gx/gx_hle.cpp`, listed in `recomp/hle.txt`; the core was rebuilt
14:16. The default is legacy, where each hook only calls the original. With
`MW_GX_HLE=compare` the original still drives the FIFO. Each hook also encodes the call
from its arguments plus the `__GXData` shadow, and checks two things: the bytes the
original wrote to the pipe (captured in `gx/device.cpp`), and whether the pre-call shadow
matches the BP register image the FIFO had processed.

30 s boss-switch run, stage 201, challenge accepted by `MW_PC_DIAG_HOLD_ENTER` taps:

| hook | calls | byte mismatches | shadow ≠ hardware before call |
|---|---|---|---|
| GXSetTevColorIn | 82,458 | 0 | 0 (0%) |
| GXSetTevAlphaIn | 81,701 | 0 | 730 (0.9%) |
| GXSetTevColorOp | 82,458 | 0 | 19,308 (23%) |
| GXSetTevAlphaOp | 81,701 | 0 | 10,131 (12%) |
| GXSetBlendMode | 46,953 | 0 | 1,707 (3.6%) |
| GXSetZMode | 24,729 | 0 | 115 (0.5%) |

Conclusions:
- The encoders are exact. A hooked setter can emit full register values computed from
  the shadow plus its arguments.
- Display lists change TEV/blend/z registers without updating the SDK shadow. So native
  GX state must be one register model, written in order by both SDK records and
  display-list or inline streams. SDK records alone are not enough.
- All six setters together run at about 20k calls/s, against about 600k draws/s. Draw
  volume comes from display lists and inline writes, so stage 3 (the display-list cache)
  is the frame-rate lever.
