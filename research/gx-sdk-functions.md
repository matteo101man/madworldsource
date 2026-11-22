# GX SDK function names (MadWorld PowerPC core)

Data: `research/gx-sdk-functions.json`. It has 173 entries covering every function from `0x8036F6C8` (`__GXInitRevisionBits`) to `0x80376A88` (`GXClearGPMetric`).

- Confidence: 158 high (this includes the 14 existing anchors), 15 medium, 0 low, 0 unknown.
- Method: I bounded the files with the anchors and named functions by SDK object order (GXInit, GXFifo, GXAttr, GXMisc, GXGeometry, GXFrameBuf, GXLight, GXTexture, GXBump, GXTev, GXPixel, GXDisplayList, GXTransform, GXPerf). I checked each name against the recompiled body: BP/CP/XF register ids, `__GXData` (`r2-30296`) shadow offsets and argument shapes.
- `__GXInitGX` (`0x8036FCFC`) was the most useful evidence. Its call sequence and constant arguments line up one to one with the SDK `__GXInitGX`, and that pins about 100 names directly. For example, `GXSetTevOrder(i,i,i,4)` is called for stages 0..7 and `(i,255,255,255)` for 8..15, `GXSetGPMetric(35,22)` comes near the end, and `GXClearGPMetric()` is the last call.

## Busiest functions (game call sites)

| addr | n | name | conf |
|---|---|---|---|
| 0x80375414 | 187 | GXSetTevOrder | high |
| 0x803715E0 | 161 | GXSetVtxAttrFmt | high |
| 0x80374FA8 | 160 | GXSetTevColorIn | high |
| 0x80375028 | 154 | GXSetTevColorOp | high |
| 0x80374FE8 | 151 | GXSetTevAlphaIn | high |
| 0x80375080 | 144 | GXSetTevAlphaOp | high |
| 0x8037116C | 141 | GXSetVtxDesc | high |
| 0x80371A14 | 106 | GXSetTexCoordGen2 | high |
| 0x80375298 | 104 | GXSetTevSwapMode | high |
| 0x80373A3C | 84 | GXSetChanCtrl | high |
| 0x803758E4 | 81 | GXSetBlendMode | high |
| 0x80371C3C | 70 | GXSetNumTexGens | high |
| 0x80375570 | 69 | GXSetNumTevStages | high |
| 0x80373A18 | 68 | GXSetNumChans | high |
| 0x803719C4 | 66 | GXSetArray | high |
| 0x80375F2C | 60 | GXLoadTexMtxImm | high |
| 0x803715AC | 56 | GXClearVtxDesc | high |
| 0x8037598C | 55 | GXSetZMode | high |
| 0x80375E64 | 50 | GXLoadPosMtxImm | high |
| 0x80372794 | 45 | GXBegin | high |
| 0x803752D4 | 42 | GXSetTevSwapModeTable | high |
| 0x80373940 | 38 | GXSetChanMatColor | high |

## `__GXData` shadow offsets seen

| offset | field |
|---|---|
| +2 | bpSentNot |
| +4/+6 | vNum, vLim |
| +20/+24 | vcd lo/hi |
| +28.. | VAT A/B/C |
| +124 | lpSize |
| +128/+132 | matIdxA/B |
| +168/+172 | ambColor |
| +176/+180 | matColor |
| +264 | suTs0 |
| +296 | suTs1 |
| +328/+332 | suScis0/1 |
| +336 | tref |
| +376 | IndTexCoordScale (iref/ras1_ss) |
| +384 | tevc[16] |
| +448 | teva[16] |
| +512 | tevKsel |
| +544 | cmode0 |
| +548 | cmode1 |
| +552 | zmode |
| +556 | peCtrl |
| +560.. | copy src/dst, cpDisp (+572), cpTex (+588) |
| +596 | genMode |
| +1320..+1344 | projection |
| +1348..+1376 | viewport and z scale/offset |
| +1380 | tImage0[] |
| +1528 | inDispList |
| +1532 | dirtyState |

## Notes and surprises

- The block follows the SDK order with no displaced functions. The linker dropped unused SDK functions, so there are gaps, for example `GXSetVtxDescv`, `GXGetVtxDesc*`, `GXReadDrawSync`, `GXPeekARGB/Z` and `GXGetGPFifo`.
- `GXDrawDone` (`0x80372148`) contains `GXSetDrawDone` and `GXWaitDrawDone` inlined. There are no separate copies of those two.
- `GXSetZScaleOffset` (`0x803760D8`, RVL-only) sits between `GXSetViewport` and `GXSetScissor`.
- `__GXUpdateBPMask` (`0x80374EBC`) is an empty `blr`, as in the RVL SDK.
- I found no game-specific wrappers inside the block.
- The code after `GXClearGPMetric` does not belong to GX:
  - `0x80376A98` and `0x80376ABC` are callback and IRQ stubs. One of them writes `0xCC003000` (PI).
  - From `0x80376AE0` the code uses OS interrupt and callback patterns.
  - `research/GX-HLE.md` says the block runs to `0x80378258`. That is too wide: GX ends at `0x80376A98`.
- The 15 medium entries are fifo and abort internals with few or no game call sites, two light-init float helpers, `GXGetYScaleFactor`, `GXSetTevIndBumpXYZ` and `GXSetZScaleOffset`. Their names come from source order plus partial body evidence.
