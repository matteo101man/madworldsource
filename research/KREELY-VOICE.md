# Kreely voicelines (plan)

Added, not native: Kreese Kreely talking in the fight as the fighter, not from the commentary booth. He's crude, macho,
boozy and quick with an insult, and his war stories are always a little bit made up (see [KREELY.md](KREELY.md)).
Every line below is original. None is copied from MadWorld's commentary.

**Selected voice:** contextual-v4, VoxCPM2 / VoiceStudio profile `04ebae74` (Kreely - approved).
The user selected its five test performances and requested removal of the other generated versions.
The full bank uses that setup: original user-recorded Bender impressions, scene-matched taunt/challenge/pain
references, raw output, natural timing, 32 steps, guidance 2.0 and seed 21751. No duration is forced.

Per-line scene and delivery direction is in `KREELY-VOICE-DIRECTION.json`.
`tools/audio/prepare_kreely_impression_refs.py` records the exact original clips and their preparation in
`assets/native/audio/kreely/review/contextual-v4/references/provenance.json`. These are concatenated references,
not numerical voice weights. Original recordings remain untouched.

The selected five-line reel and full render provenance are in
`assets/native/audio/kreely/review/contextual-v4/contextual-review.wav` and `review.json`.
The remaining lines use the selected setup and receive wording/format checks; a transcription match does not
grade acting. `KREELY-VOICE-CLEANUP.json` records removal of older versions. The default runtime profile is
`D:\VoiceStudio-runtime\kreely\voice-profile.json`; VoxCPM2 is also selected as VoiceStudio's TTS engine.

## Triggers

| Set | Fires on (mw-sim `FightEvent`) | Chance | Cooldown | Priority |
|---|---|---|---|---|
| `defeat` | `Victory`, or a street enemy killed | 100% (Victory) / 35% (street) | 6 s | 1 |
| `knockdown` | `RinState` into a down state caused by his hit (sweep, spin kick), `SuplexSlam { on_rin: true }` | 60% | 8 s | 2 |
| `hurt` | `PlayerHit` with `damage` >= 8, or a hit that knocks him down | 50% | 7 s | 3 |
| `near_death` | HP crosses down through `play_hurt::LOW_HP` (30) | 100%, once per crossing | none | 1 |
| `teeth` | the teeth knock-out (`play_hurt::step`, a low-HP face hit) | 100% | 10 s | 1 (replaces `hurt`) |

Rules:
- One Kreely line at a time. A higher-priority line cuts a lower one; equal priority waits.
- Don't repeat the same line until half of its set has played (a shuffled bag per set).
- Duck under the announcer (`FightEvent::Voice`) and never start while one is playing.
- Nothing during the intro, the pause menu, the death or the continue screen.
- `defeat` waits for the end of the finisher, not for its first hit.

Files: `assets/native/audio/kreely/<set>_<nn>.wav`, 44.1 kHz mono, listed in `kreely.json`
(`{set: [{file, text}]}`), staged by `tools/audio/stage_kreely_voice.py`. Rendering resumes from matching completed
files; `--only <set_nn> --force` regenerates a selected line. Run with the VoiceStudio Python environment and local
backend running. `generation.json` records the source text, acting cues, settings, hashes and VoiceStudio history IDs.

## Lines

Delivery notes in [brackets] inform the per-line acting instructions and are never spoken literally.
Authored phonetic spellings remain in the game manifest. For `teeth_06`, synthesis uses “Hey! I was using those!”
for intelligibility; the pronunciation override is recorded in `generation.json`. `knockdown_03` uses “Timber!”.

### `defeat`: he beat an enemy
1. [laughs] And THAT'S how we did it on Death Watch. Somebody get me a drink.
2. Stay down, sweetheart. Uncle Kreese has a bar tab waiting.
3. [cracks knuckles] Ah, I needed that. Better than a hot shower and a cold beer.
4. I've had hangovers that put up a better fight than you.
5. Somebody sweep that up. [spits] I'm not touching it.
6. [breathing hard] Woo! Still got it. Never lost it. Never even misplaced it.
7. That's one more for the wall. I'm gonna need a bigger wall.
8. Tell the devil Kreese says hi. He owes me twenty bucks.
9. Retired, my ass.
10. Lights out, princess.

### `knockdown`: he put someone on the floor
1. [grunts] Down you go!
2. Floor's comfy, huh? Stay a while.
3. Timber!
4. Get up! I ain't done with you.
5. [laughs] Did that hurt? It looked like it hurt.
6. That's a Kreely special. Free of charge.
7. Kiss the canvas, sweetheart.
8. And she drops like my third marriage!

### `hurt`: he took a serious hit
1. [grunts] Gah! You little...
2. Ow! Son of a... that's coming outta your hide.
3. [groans] Okay. Okay. Now I'm mad.
4. That all you got? [coughs] ...Don't answer that.
5. Hit me again. I dare ya. Actually, don't.
6. Ugh! I felt that one in my liver. What's left of it.
7. [spits] Lucky shot!
8. Damn it, that was my good side!

### `near_death`: his HP just fell into the danger zone
1. [wheezing] Oh, that ain't good... I'm leaking.
2. [groans] I'm seeing two of you. Neither one of you is pretty.
3. Not like this... not sober...
4. [breathing hard] Howard was right. I should've stretched.
5. Okay, time out. Time out! ...Nobody's calling time out.
6. If I die out here, my liquor cabinet goes to nobody. NOBODY!
7. [coughs] I've been worse. I can't remember when, but I've been worse.
8. Ain't the first time I've been almost dead. Won't be the last.

### `teeth`: a low-HP face hit knocks his teeth out
1. [muffled] My teef! Thoze were my good teef!
2. [spits] Aw, come on! I just paid those off!
3. [lisping] Thas... thas gonna cost ya, lady.
4. [spits] Well, there goes dinner. Soup it is.
5. [grunts, spits] Keep 'em. Something to remember me by.
6. [muffled] Hey! I wath uthing thothe!
