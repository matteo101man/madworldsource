# Current Rust integration - 2026-11-09

Play Log is now integrated in mw-play: original glyphs, ten-page wrap, source
counters and entry/back flow. All46 values remain initial zeros pending actual
statistic systems. This is functional menu presentation/navigation, not complete
live statistics.60 library tests and actual page/freeze/resume smoke pass.

Native message geometry and UVs are verified for all75 first-page glyphs using
research/ui-playlog-glyphs-observed.json, from the separate fixed message pool.
Original120s/4700presentations/exit0 ui-playlog-text-oracle supplied the observation.
All92messages validate/layout. Captures evidence/rust-playlog-page.png and
rust-playlog-last-page.png were inspected; counters1/10 and10/10 are correct.

stage_playlog.py now stages font metrics and digits from MTD chunk128,payload160,
in etc/idpac_core_us.dat. Its length12512 excludes the32-byte chunk header.
The core.mtd in core2_us.dat is a different font and failed native UV/size checks;
do not use it for these numbers. Catalogv2 has37glyphs,3atlas pages,92messages.
Native CORE registration8026FC9C maps this main font independently of EM labels.

Placement uses80184490: half metric width before draw; after draw2+.3metric width.
Space advances half nominal width.80186DAC binds dimensions[18,18,16] from each
row's source header+40; command8010size80% gives.6glyph scale.801877A8 centers
single line by subtracting half lineHeight*size; native correction then subtracts
.25*size*(baseAdvance-glyphHeight). First label center114.8,94.2.8093 formats decimal
with999999cap, shifts start left half nominal width per extra digit.8073style0,
8001space,8002end and8010size are supported; other commands fail explicitly.
Full message engine, style variants, typewriter and multiline events remain pending.

The controller research below is historical where it describes these as unported.

---

# Original Play Log controller and source bindings - 2026-11-09

Status: native capture and layout provenance recovered. Play Log is NOT yet
integrated into mw-play. This is the next UI implementation task, not a completion
claim. Underlying full-stage statistics also require gameplay systems not yet ported.

## Evidence

Original owned-core fixture ui-playlog-oracle passed120s/4563presentations/exit0,
including host blit, actual pause/A packets, capture and cooperative shutdown.
No actor/health/state injection. Only alternate build-record DLL used. Files:
- evidence/ui-playlog-oracle.png / .ppm / .json / .log
- evidence/ui-playlog-page-state.json (bounded read-only own-process snapshot)
- research/ui-playlog-map-observed.json (159 bound widgets,158 exact source headers)
- assets/native/ui/playlog-us/catalog.json (92 messages,27 glyphs, original font PNG)

Captured first page is SIGN IMPALINGS, OIL DRUM SLAMS, BULL HEAD SLAMS,
FLAT TIRE SLAMS, GARBAGE-STINK-VOMITS, each0. Native counter1/10.
Map contents/player marker are hidden; original legend and HP/lives/score panel
remain. Selection has original PLAY LOG inversion and yellow confirmation fill.
PAGE + Return prompts and TV scan/noise remain visible. PC Return glyph patch is
host-specific and is not yet shared by the pure Rust presenter.

## Controller translation points

- ctor8024974C resets handles/state, page0; update80249778 loads English MTD/MSD
  and registers EM catalog. Page count=(messageCount/2)/5+1:92 ->10.
- 80249A30 wraps Left/Right through pages. Back removes active messages, requests
  parent return mode3 and tears down page. Native message slots+44..+6C hold IDs.
- 80249B74 emits5 label/value pairs, message IDs0x21000000+(page*10+row*2).
  It publishes each row's layout widget pointer via global8069E2FC before calling
  message manager80182AE0. 80249CB8 updates page number via parent virtual+40,
  widget ID0x26,count2. Total page number uses ID0x24,count2.
- Map init80247624 uses80248020 to set10 layout pointers in parent+6C:
  label/value widget IDs[91,92],[96,97],[106,107],[111,112],[116,117].
  Group roots90/95/105/110/115 have center(240,100/130/160/190/220), anchors are
  child(-130,0) and(+130,0), yielding logical X110/370. These source anchors have
  flag4000/unprepared native matrices; use source transforms, not zero matrices.
- Page activation hides map100/71/player arrow via80248A64(0), shows groups30/35,
  stores TV-frame widget6 on page+38 and pointer-table parent+6C on page+40.
  Exit80248B28(mode3) hides16/30/35, restores map/arrow and root parent page state.
- 802475D0 toggles selection/inversion;8024894C controls confirmation fill.
  Need include these exact bindings and transitions in the Rust controller.

## Messages still to translate

All catalog control tokens are0x8001(space),0x8002(end),0x8010(size80),
0x8073(style0),0x8093(dynamic integer index). Font glyph IDs0x1000..0x101A
reference original256x128 MTD atlas. Do not replace labels with a system font.
Space/end interpretation follows observed message content; glyph sizing, advance,
alignment, number formatting and selected style need their native consumers.
Numeric insertions refer to46 original statistic indices; do not manufacture
nonzero progress or call a page full-featured while actual stats are untracked.

Read-only decompilation remains in %TEMP%/mw-havok-proxy:
ui-options-playlog.c, ui-page-bindings.c, ui-map-playlog-bindings.c,
ui-playlog-layout.c, ui-playlog-messages.c.80182AE0 selects message object then
80183F98/801843C0 parse/layout it. Follow those functions and8069E2FC consumption
next. No raw native C has been copied into the Rust source or this research file.
