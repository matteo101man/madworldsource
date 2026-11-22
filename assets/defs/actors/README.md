# Actor definitions

One JSON file per enemy, boss or NPC, named by its id (`em0101.json`). `crates/mw-game/src/play_actors.rs` reads them
all at start-up; restart the game after an edit. `MW_ACTOR_LOG=1` prints what loaded, and a file that does not parse is
reported and skipped (save as UTF-8). Every field is optional: anything left out keeps the module's built-in (native)
value. Keys starting with `_` are ignored (use them for notes).

| field | meaning |
|---|---|
| `id`, `name`, `kind` | id (the file name if empty), a label (`"Elise (c303 boss)"`: the editor shows the part before ` (` as the title), `"zako"`, `"boss"` or `"npc"` |
| `models`, `motions` | GLBs and the motion bank under `assets/` (the editor lists every clip in `motions`) |
| `hp`, `power`, `sense`, `radius` | max HP, base hit power, perception (m), body radius (m) |
| `variants[]` | per outfit: `hp`, `near` / `far` attack chance (% a frame), `power` (scale), `evades`, `evade` (%), `attacks` (names it may pick) |
| `anims` | role -> clip (`"idle": "em0101_0000.mot"`) |
| `pools` | name -> clips picked at random (`laughs`, `threats`) |
| `attacks[]` | `name`, `clip`, `windows` (`[[from, to]]` clip frames that hit), `combo` (frame it may chain into the next attack in the list), `range` (m), `yaw` (degrees), `power`, `atk` (native ATHIT id: the player's reaction), `voice` |
| `voices` | event -> announcer voice id (decimal) |
| `tune` | free named numbers the module reads |
| `editor` | its page in the actor editor (below) |
| `names` | what things are, in words, for the editor: `clips` (clip -> name), `voices` (decimal id -> `name`, `trigger`: the game event that plays it), `cameras` (camera clip -> `name`, `actor` / `player`: the clips it films, `entry`: the editor entry that plays that scene), `phases` (`state` -> each state's name; a state's numbered phases, e.g. `Keen` -> `"2": "The struggle"`) |

What each module reads today:

- **em0101** (play_street.rs, the kung-fu grunts and the placeholder zako): `power`, `sense`, `variants`, `pools.laughs`,
  `pools.threats`, `attacks` (order matters for combos; `fire` and `dash` are found by name), `tune.alive` (grunts at
  once on the street).
- **em0630** (play_elise.rs, Elise): `hp`, `tune.csv0`..`csv6` (em0630.csv), `tune.power0`..`power4` (her attack
  powers), `tune.summon_*` (the hide-phase zako drop), `voices` by event name (`wing_strike`, `bats_answer`, `landing`,
  `dragon_hint`, `hurt_strong`, `hurt_light`, `clash_start`, `clash_won`, `bite`, `victory`, `clear`).
- **em0440** (RinRin, mw_sim::fight), **em0623** (the Baron) and **em0610** (Mathilda, play_baron.rs): their editor
  pages only, so far.

Adding a zako: until it has its own module, copy `em0101.json` to the new id for its numbers; the grunt AI
(play_street.rs) is em0101's. Spawning one from another stage: push `(position, yaw, outfit)` onto
`Street::queue` (play_elise.rs does this for the bat drops). A new model or moveset means a new id here plus the code
that loads its GLBs and bank and reads its file, as play_street.rs does.

## The actor editor

In game: the stage select's ACTOR EDITOR list (every definition with an `editor` page), or
`MW_STAGE=<its stage> MW_EDITOR=<id>`. The actor stands in front of the player with its AI held; a menu in the
game's own lettering lists the page's groups, then three the editor adds itself: every **clip** in `motions` (split by
`clip_groups`), every **voice line** in the stage's announcer table (named from `voices`) and each recording of them
(**voice takes**), and every **camera** clip in `cameras`. Entries play through the actor's own module, so a clip's
sound and effect cues, hit blood and stains, voice lines and pair cameras all run as in a fight.

Keys: Up / Down pick, Left / Right group, Enter plays, R resets, Space pauses, `-` / `=` speed (1, 0.5, 0.25, 0.1),
V camera (orbit / game / camera clip), right mouse drag and the wheel orbit, I the actor's AI on / off, H hides the
menu, F2 gives the keys to the player (to fight it, or to play a pair yourself) and back, F9 the stage select.

### The page

```json
"editor": {
  "stage": "elise",                 // the MW_STAGE that hosts it
  "spot": [0, 0, 6], "player_spot": [0, 0, 9.5],   // where R stands them (default: 3.5 m in front of the player)
  "focus": 1.3,                     // the orbit's look height (m)
  "cameras": ["elise-camera.json"], // camera MOT files under assets/native
  "camera_anchor": "player",        // a camera clip with no scene is previewed on the player or (default) the actor
  "groups": [ { "name": "Attacks", "entries": [ { "label": "Wing strike", "do": "move", "arg": "attack", "note": "0111" } ] } ],
  "clip_groups": [ { "name": "attacks", "prefixes": ["em0630_01"] } ]
}
```

Entry kinds (`do`):

| `do` | `arg` | what plays |
|---|---|---|
| `clip` | a clip name (`looped`: true loops it) | the raw clip on the actor, with its cues |
| `move` | a move name the module knows | a native state: an attack, a taunt, a guard, the hide... |
| `hurt` | `light`, `heavy`, `front`, `back`, `left`, `right`, `shot`, `down`, `launch`, `sweep` | a blow that lands (no guard or dodge roll), with its blood |
| `kill` | | the death (bosses: down to finish-ready) |
| `pair` | `clash`, `finisher`, `grab`, `pin`, `finale`, `bat` (per module) | a pair with the player; `win`: true / false forces the outcome, absent waits for the player's own input (F2) |
| `execution` | | the last-life finisher on the player (the boss's own) |
| `voice` | an event in `voices`, or a number (`0x9D`) | the announcer line |
| `camera` | a camera clip name | its scene: the `names.cameras` entry that films it (played as in a fight), else its `actor` / `player` clips played together on its clock, else the camera alone on `camera_anchor` (V leaves it) |
| `input` | `punch`, `swing`, `grab`, `taunt`, `jump`, `dodge` | the player presses it once |
| `reset` | | both back at their spots |

A module tells the editor when it has no such move or pair; the status line shows its answer (the names it does
know), and "no module answers" when nothing on this stage edits that id.

Each module's names, today:

- **em0630** Elise: moves `attack`, `somersault`, `volley`, `volley_b`, `dragon`, `guard`, `takeoff`, `land`, `hover`,
  `hide`, `summon`, `laugh`; pairs `clash`, `finisher`.
- **em0101** grunt: moves = its `attacks` names (`claw1`..`claw3`, `fire`, `dash`), `laugh`, `threat`, `spotted`,
  `dodge`, `guard`, `stun`, `fear`, `outfit_a`, `outfit_b`; pair `grab`.
- **em0440** RinRin: moves `uraken`, `tetuzankou`, `slalom`, `fans`, `taunt`, `guard`, `right_step`, `left_step`,
  `back_step`; pairs `clash`, `finisher`.
- **em0623** the Baron: moves `combo`, `mach`, `dropkick`, `shoryuken`, `antiair`, `bhupper`, `godhand`, `dempsey`,
  `riderkick`, `angry`, `threat`, `guard`, `avoid`; pairs `clash`, `finale`, `bat`, `finisher`.
- **em0610** Mathilda: moves `swing`, `follow`, `windup`, `smash`, `threat`, `guard`, `avoid`; pairs `pin`, `finisher`.

### Driving it without the keyboard

`MW_EDITOR_RUN="40:Attacks/Wing strike;200:Hurt/Heavy;300:reset"` plays entries at game ticks (group / entry matched
by their start, any case; also `reset`, `camera:orbit`, `camera:game`). `MW_EDITOR_LOG=<n>` prints the module's line
(clip, frame, state, HP) every n ticks, `MW_EDITOR_ORBIT=yaw,pitch,dist` starts the orbit there (yaw from the actor's
facing), and `MW_PLAY_SHOTS="tick:file.png;..."` takes screenshots. A check of a new entry:

```
MW_STAGE=elise MW_EDITOR=em0630 MW_EDITOR_RUN="40:Power/Clash - player loses" MW_EDITOR_LOG=30 MW_PLAY_SHOTS="600:bite.png" mw-play
```

### Wiring a new actor in

1. Its definition here with an `editor` page (`stage` must be a stage whose module spawns it).
2. In its module's FixedUpdate step, take `ResMut<editor::Editor>` and, when `editor.on && editor.id == "<id>"`:
   - on the first frame (and on `Cmd::Reset`) stand it at `editor.spot` facing the player at `editor.player_spot`,
     full HP, any pair ended;
   - `if let Some(cmd) = editor.take() { let r = handle(&cmd); editor.done(&cmd, r); }`: `Cmd::Clip`, `Move`, `Hurt`,
     `Kill`, `Pair { name, win }`, `Scene { actor, player }`, `Reset`; return `Err("...")` for what it does not have
     (the editor shows it);
   - while a `Scene` plays, set `editor.scene = Some((frame, origin, yaw))` (its clip's frame, where it began) and
     draw the clip with its root travel; `None` once it is over;
   - set `editor.info` in words (`editor::clip_label(id, clip)`, `editor::phase_label(id, "state", ..)`: its clip,
     frame, state and phase, HP), `editor.anchor` (position, yaw: the orbit and camera clips ride
     it) and `editor.idle` (nothing playing: the player's HP refills);
   - while `!editor.ai`, hold it in its idle instead of its decisions (a state or clip the editor asked for still plays
     out, then back to idle).
3. Plays that need the player's button (a grab) call `editor::press(Press::Grab)`; a forced outcome reads the `win` it
   was given in place of the player's input.

play_elise.rs (`editor_reset` / `editor_cmd`), play_street.rs (`ed_reset` / `ed_cmd`), play_baron.rs and
play_editor.rs `rinrin` are the four worked examples.
