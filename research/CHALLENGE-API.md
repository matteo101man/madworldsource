# Boss-entry confirmation, stage201

Checkpoint 2026-11-09. This is the original TAKE THE CHALLENGE? controller,
not evidence for a general stage/challenge-selection list.

## Source boundary

- DOL80147BBC calls8012409C(0) before event21. False returns from boss entry;
  true proceeds to the intro and subsequent combat setup.
- 8012409C enters demo/guard scopes, opens the controller through80244ED0,
  yields while80244F30 is active, reads80244F58, then80244F74 removes it.
  Scene abort forces false. The caller owns the world return.
- 80243358 creates type0x79, program42, selects boss index3 for stage201.
  Native captures observe ranking0, vtable805ca5a8. 80243120 admits a boss
  when boss index >= ranking. The unavailable/ranking-error path is unported.
- 802434D0 allocates bank1/program42/layer0xD13, hides cursor9, selects Yes,
  sets widget21 UV index3 and starts the opening. The source is core_us IDT,
  40 widgets. Atlas group74/index3 is the RINRIN caption. The separately
  staged c201 atlas is broader stage HUD/results art, not this prompt's font.
- 802431AC binds stage message0x12010008 after the opening. Its position is
  widget5.resource root[0,20,0], not the widget's prepared zero-sized quad.

## Controller80242834

Observed world-update edges are preserved, including idle endpoint holds.
The runtime clock uses the same29.97Hz source cadence as the event entry.

1. Opening waits for Yes11's position envelope completion: row1 is source
   time0; row25 enters selection, after24 controller updates. Cursor9 is
   still hidden on that transition and appears on the next selection update.
2. Left selects Yes, right selects No. B selects No; it does not confirm or
   close the controller. A confirms the current selection. The cursor gets
   the selected entry's evaluated local position, copied by the controller.
3. The next update binds RGB channel5 from widget13 to cursor and selection.
   Each flash evaluates source frames0..4 and holds the endpoint before the
   next restart: five flashes, five world updates each.
4. Resolve sets result Yes/No. The following update clears message glyphs,
   binds root1's scale channel from widget2 and starts the close. Source2
   scales X only. The four closing observations are1,2/3,1/3,0 before removal.

Native accept capture changes phase at rows0/25/40/41/66/67/71(closed);
native decline changes at0/25/54/55/80/81/85(closed). Input recorder samples
pad-down after the controller, so recorded downs apply at the next edge.
Rust tests replay that ordering rather than shifting the native capture.

## Original message

`tools/ui/stage_challenge.py` extracts only message8 from owned
case2/idpac_c201_us.dat, retaining the original tokens,93 glyph metrics and
unmodified font page. Local catalog index0 maps explicitly to0x12010008.
It does not manufacture Unicode text or use replacement fonts.

801843C0 opens type5;801840EC enables horizontal/vertical centering.
80186DAC defaults give factors20/24 and line spacing20*20/24. The source
message sets scale65%, has three rows and ends with scale reset/end.
801870F4 sums2+0.7*metricWidth*factor per stage-family glyph and half-em
spaces. 80184490 advances half-width before emission and2+0.2*width after.
The quarter-scale native height correction is retained. The initial baseline
is anchorY - rowCount*0.5*lineSpacing*scale; later lines add lineSpacing*scale.
All48 prepared glyph centers, dimensions and UV bounds match native.
The separate common-family Play Log layout/numbering tests still pass.

The bounded message-manager probe confirms message/type/flags/anchor/factors.
The manager at80653f04 is a static object: the original caller uses addi,
not lwz. Decompiler labels alone were insufficient to distinguish this.

## Shared UI corrections

- 800C80C0 scales static child positions by the parent's separate child-scale
  vector; it adds animated offsets afterwards. Previously Rust scaled both.
- 800C7204 propagates child scale independently of visible quad dimensions.
  Active scalar envelopes multiply both inherited axes, with source flags
  controlling local factors. Without an envelope, source flags gate inherited
  quad dimensions. The challenge closing channel binding uses source2's axes.
- UiScene now supports scale/RGB envelope bindings and restarts directly.
  This uses the original channel data, not a hand-authored flash palette.

## Rust integration and remaining boundary

`MW_CHALLENGE=1 cargo run -p mw-game --bin mw-play` enables the inspection
path before the default intro. In PowerShell set `$env:MW_CHALLENGE='1'`
first. Arrow Left/Right or A/D chooses, Enter/left mouse confirms;
Backspace/Escape/right mouse selects No. Yes enters the existing intro,
cleanup/resource wait and room fade. No closes and exits this inspection.
It does not substitute an invented roaming-world return. Default startup
continues to begin at the intro until actual world/boss-switch flow is ported.
Direct-fight and fixed intro preview bypass the challenge inspection.

`MW_CHALLENGE_FRAME=30` with MW_CHALLENGE=1 freezes a settled prompt for
inspection. MW_CHALLENGE_SMOKE=accept|decline enables explicit test inputs.
Both are diagnostics. Input edges are latched until a source update, while
simulation and pause controls remain gated. The prompt itself is currently
silent: native menu SE admission/mixing remains to translate, as do ranking
save integration, abort/world return, results and wider challenge systems.

## Evidence and validation

- Native accept45s and decline30s owned isolated-user captures: exit0.
  `evidence/challenge-{accept,decline}-v1.jsonl`; probes default off, read-only,
  bounded to1024 rows/768 UI nodes/256 glyph slots. Test choices use original
  WPAD input, never controller writes. Only build-record DLL rebuilt.
- `tools/ui/stage_challenge_evidence.py` validates source offsets/IDs and
  preserves the compact fixture plus hashes in CHALLENGE-EVIDENCE.json.
- `cargo test -p mw-sim --lib ui_challenge` compares all controller edges,
  selected choice/result, visibility, every native-visible nondegenerate
  widget position/size/RGBA/UV, message lifetime and all48 glyph UV/metrics.
  Full simulation suite80 passes; data suite9 passes; Rust bins check/build.
- Rendered `evidence/rust-challenge-prompt.png` inspected. Native UI geometry
  is checked, not pixel equivalence. Existing TV noise/compositing limitations
  remain visible, as do the arena/renderer's broader fidelity limitations.
- Runtime accept -> intro skip at301 -> cleanup -> room fade -> fight tick30,
  and decline -> no intro/fight, exit0. Logs `evidence/rust-challenge-*.log`.
- Fight regression re-run:2533/1756 frames; one HP mismatch each remains;
  frame398 gate Z error3.04894e-5. All repaired category ceilings pass.
