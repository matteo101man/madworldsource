"""Stage native effect banks for mw-play (crates/mw-game/src/play_fx.rs): etc/core2.dat core2.eff (the bank the game
loads into EffCore 8060412C; core2_us.dat holds a smaller 286-record variant without 1005/532) and RinRin's em0440.eff.

EFF: 'EFF\\0', u32 n, then n (u32 id, u32 offset) rows at +8: 0 TEX, 1 EST, 2 MOD, 3 TUV, 4 MOT.
TEX: 'TEX\\0', u32 n, (u32 id, u32 offset) rows at +8; TPL blobs. 199 is the sprite atlas (4 pages).
TUV: 'TUV\\0', ..., u32 groups at +0x20, then (u16 id, u16 first, u16 count, u16 data) rows at +0x24; frames at
     +0x24+data: (u16 frame, u16 atlas page, f32 u0, v0, u1, v1).
EST: 'EST\\0', u32 n, (u32 id, u32 offset) rows at +8 (lookup 801FD7C0). Record: u32 emitters, u32 offsets[].
Emitter fields [native readers]:
  8008F1EC: u16 0x02 life (frames), u16 0x12 life random; flags 0x08 & 0x40 -> TUV group u16 0x204, start frame
            byte 0x1EA, frame count byte 0x1EB (else TEX u16 0x1D0); type 0x11 + TEX 101/104 swaps to the blood
            variant when tinted with getBloodColor.
  8008D2C8: byte 0x1A0 particle count, byte 0x1A1 emit interval, u16 0x1A6 emitter frames.
  801FAC58 (MOVE block at 0x40; every random term is base + range * U[-1,1], 800BB888): 0x44 offset, 0x50 offset
            random, 0x5C velocity, 0x68 velocity random (or, when 0x194 != 0, a random direction of length 0x194),
            0x74 drag, 0x78 acceleration, 0x84 rotation deg, 0x90 rotation random, 0x9C spin deg, 0xA8 spin random,
            0xC4 width, 0xCC width random, 0xC0 height, 0xC8 height random, 0x100 RGBA, 0x18C color random,
            0x190 gravity.
  801FBAF8: pos += vel*dt; pos.y += g_acc*dt; g_acc += gravity*dt; vel += acc*dt; vel, acc *= drag^dt; unless byte
            0x0D == 0xFC, y stops at the floor.
Pixels are written unchanged (white masks; the game tints per draw).
"""
import json, struct, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, tpl  # noqa: E402

# Banks mw-play loads: core (EffCore 8060412C) and RinRin's own em0440.eff (her motion op-16 cues with bank 2 and the
# setEffect calls of her state code use it). Every record is staged; which ones play, and when, is in play_fx.rs
# (observed with MW_TRACE_EFFECTS in evidence/native-effects-trace*.log, including a manual play with hits on RinRin).
BANKS = [('core', 'etc/core2.dat', 'core2.eff'), ('em0440', 'em/em0440.dat', 'em0440.eff'),
         ('em0400', 'em/em0400.dat', 'em0400.eff'), ('em0101', 'em/em0101.dat', 'em0101.eff'),
         ('bl0063', 'bl/bl0063.dat', 'bl0063.eff'),
         # the bosses' own banks (their motion op-16 cues with bank 2): Elise, the Baron, Mathilda (em0615.dat holds
         # em0610.eff)
         ('em0630', 'em/em0630.dat', 'em0630.eff'), ('em0623', 'em/em0623.dat', 'em0623.eff'),
         ('em0610', 'em/em0615.dat', 'em0610.eff'),
         # c303's breakable ScrObjs: the pillars (EST 1 at the blow, 800 as a piece lands) and the pews
         ('bl005c', 'bl/bl005c.dat', 'bl005c.eff'), ('bl005d', 'bl/bl005d.dat', 'bl005d.eff')]   # (em0960.eff, the oirans', is an empty member)   # bl0063: RinRin's bomb (EST 0 blast, EST 1 fuse spark)   # em0101: the kung-fu grunts (broadcast, street stage)   # em0400: Jude's gun flashes, for Kreely's pistol (added use)
BLOOD = [0x80 / 255, 0, 0, 1]   # getBloodColor(0) [observed]
OUT = ROOT / 'assets/native/effects'


def stage(name, dat, member):
    raw = (EXPLORER / 'extracted/disc/DATA/files' / dat).read_bytes()
    e = next(x for x in dat_entries(raw) if x['name'] == member)
    eff = raw[e['offset']:e['offset'] + e['size']]
    assert eff[:4] == b'EFF\0'
    sec = dict(struct.unpack_from('>II', eff, 8 + 8 * i) for i in range(struct.unpack_from('>I', eff, 4)[0]))
    u16 = lambda o: struct.unpack_from('>H', eff, o)[0]
    u32 = lambda o: struct.unpack_from('>I', eff, o)[0]
    f3 = lambda o: list(struct.unpack_from('>3f', eff, o))
    f1 = lambda o: struct.unpack_from('>f', eff, o)[0]

    tex = sec[0]
    assert eff[tex:tex + 4] == b'TEX\0'
    trows = dict(struct.unpack_from('>II', eff, tex + 8 + 8 * i) for i in range(u32(tex + 4)))
    offs = sorted(trows.values()) + [sec[1] - tex]
    images = lambda t: tpl(eff[tex + trows[t]:tex + offs[offs.index(trows[t]) + 1]])

    # TUV: +4 atlas count (1), +8 atlas TEX id, groups at +0x20
    tuv = sec.get(3)
    groups, pages = {}, []
    if tuv and eff[tuv:tuv + 4] == b'TUV\0' and u32(tuv + 4):
        for i in range(u32(tuv + 0x20)):
            gid, first, count, data = struct.unpack_from('>4H', eff, tuv + 0x24 + 8 * i)
            groups[gid] = [dict(frame=f, page=p, uv=[round(v, 6) for v in (a, b, c, d)])
                           for f, p, a, b, c, d in (struct.unpack_from('>2H4f', eff, tuv + 0x24 + data + 0x14 * k) for k in range(count))]
        for j, (im, _) in enumerate(images(u32(tuv + 8))):
            im.convert('RGBA').save(OUT / f'{name}-atlas-{j}.png'); pages.append(f'native/effects/{name}-atlas-{j}.png')

    est = sec[1]
    assert eff[est:est + 4] == b'EST\0'
    rows = dict(struct.unpack_from('>II', eff, est + 8 + 8 * i) for i in range(u32(est + 4)))

    def decal(o):
        # type 24 (cEsp24, ctor 80097A54): a decal projected onto the owner's meshes (801F89E8 / 801D5C5C); extension
        # u16 +0x182 = X, shorts at +0x40+X: depth x1000 (0 -> 0.5, 80560E04), mesh A, mesh B, region remap, cull
        if eff[o + 1] != 24: return None
        x = u16(o + 0x182)
        d, a, b, remap, cull = struct.unpack_from('>5h', eff, o + 0x40 + x)
        return dict(depth=(d / 1000) if d else 0.5, meshes=[a, b], remap=remap, cull=cull)

    def emitter(o):
        m = o + 0x40
        return dict(kind=eff[o + 1], decal=decal(o),
            life=u16(o + 2), life_random=u16(o + 0x12), flags=[u32(o + 4), u32(o + 8)], floor=eff[o + 0xD] != 0xFC,
            count=eff[o + 0x1A0], interval=eff[o + 0x1A1], frames=u16(o + 0x1A6),
            tuv=u16(o + 0x204) if u32(o + 8) & 0x40 else None, tex=u16(o + 0x1D0), frame_start=eff[o + 0x1EA], frame_count=eff[o + 0x1EB],
            offset=f3(m + 4), offset_random=f3(m + 0x10), velocity=f3(m + 0x1C), velocity_random=f3(m + 0x28), speed_random=f1(o + 0x194),
            drag=f1(m + 0x34), accel=f3(m + 0x38), rotation=f3(m + 0x44), rotation_random=f3(m + 0x50), spin=f3(m + 0x5C),
            spin_random=f3(m + 0x68), size=[f1(o + 0xC4), f1(o + 0xC0)], size_random=[f1(o + 0xCC), f1(o + 0xC8)],
            rgba=list(struct.unpack_from('>4f', eff, o + 0x100)), color_random=f1(o + 0x18C), gravity=f1(o + 0x190),
            pivot=[f1(o + 0x1E0), f1(o + 0x1E4)])  # [inferred: quad anchor in sprite units]

    effects = {}
    for i, off in rows.items():
        if not off or i in effects: continue
        base = est + off
        n = u32(base)
        if not 0 < n < 64: continue
        effects[str(i)] = [emitter(base + x) for x in struct.unpack_from('>%dI' % n, eff, base + 4)]
    return dict(source=f'{dat}:{member}', pages=pages, sprites={str(g): v for g, v in groups.items()}, effects=effects, images=images, trows=trows)


OUT.mkdir(parents=True, exist_ok=True)
# (only this tool's own images: tools/stage_wound_decals.py writes wound-*.png / claw-marks.png here too)
for old in OUT.glob('*.png'):
    if not old.name.startswith(('wound-', 'claw-')): old.unlink()
banks = {name: stage(name, dat, member) for name, dat, member in BANKS}
# Sprites and sheets resolve through the core bank for every bank (em0440.eff has empty TEX/TUV sections; the TUV group
# u16 +0x204 goes to the shared sprite manager 8069550C, TEX ids to texture manager 80604344).
core = banks['core']; sheets = {}; sheet_frames = {}; core_pages, core_sprites = core['pages'], core['sprites']
# A bank with its own TUV groups / TEX records registers them in those shared managers while its room is loaded, so its
# emitters draw its own sprites (the bosses: em0623's group 72 is his punch trail, core's 72 an unrelated sheet). They
# are staged under bank-unique ids: group / tex + 10000 x (bank index), pages appended after core's.
for bi, (name, b) in enumerate(banks.items()):
    if name not in ('em0630', 'em0623', 'em0610'): continue   # (the older banks keep their traced core resolution)
    own_groups = {int(g): v for g, v in b['sprites'].items() if v}
    own_tex = set(b['trows'])
    if not own_groups and not own_tex: continue
    base = len(core_pages); core_pages.extend(b['pages'])
    for g, frames in own_groups.items():
        core_sprites[str(10000 * bi + g)] = [dict(f, page=f['page'] + base) for f in frames]
    for v in b['effects'].values():
        for em in v:
            if em['tuv'] is not None and em['tuv'] in own_groups: em['tuv'] = 10000 * bi + em['tuv']
            elif em['tuv'] is None and em['tex'] in own_tex:
                t = 10000 * bi + em['tex']; em['tex'] = t
                frames = b['images'](t - 10000 * bi)
                frames[0][0].convert('RGBA').save(OUT / f'tex-{t}.png'); sheets[str(t)] = f'native/effects/tex-{t}.png'
                if len(frames) > 1:
                    for k, (im, _) in enumerate(frames): im.convert('RGBA').save(OUT / f'tex-{t}-{k}.png')
                    sheet_frames[str(t)] = [f'native/effects/tex-{t}-{k}.png' for k in range(len(frames))]
    print(name, 'own sprite groups', len(own_groups), 'own textures', len(own_tex))
for t in sorted({em['tex'] for b in banks.values() for v in b['effects'].values() for em in v if em['tuv'] is None}):
    if t not in core['trows']: continue
    frames = core['images'](t)
    frames[0][0].convert('RGBA').save(OUT / f'tex-{t}.png'); sheets[str(t)] = f'native/effects/tex-{t}.png'
    # every image of the TPL (the decal splats pick theirs by the start frame byte 0x1EA)
    if len(frames) > 1:
        for k, (im, _) in enumerate(frames): im.convert('RGBA').save(OUT / f'tex-{t}-{k}.png')
        sheet_frames[str(t)] = [f'native/effects/tex-{t}-{k}.png' for k in range(len(frames))]
for b in banks.values(): del b['images'], b['trows'], b['pages'], b['sprites']
(OUT / 'effects.json').write_text(json.dumps(dict(
    schema='mw-native-effects/2', blood=BLOOD, pages=core_pages, sprites=core_sprites, sheets=sheets, sheet_frames=sheet_frames, banks=banks,
    authority='EST/TUV fields from the native readers named in tools/stage_effects.py; triggers observed with MW_TRACE_EFFECTS; '
              'size/alpha-over-life curves and the draw path are not decoded'), separators=(',', ':')))
print({k: len(v['effects']) for k, v in banks.items()}, len(core_sprites), 'sprite groups', len(sheets), 'sheets')
