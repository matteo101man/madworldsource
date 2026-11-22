"""Bake the explorer's added inspection palette into a Dolphin-format texture pack.

The explorer (exports/madworld/color.js) tints each mesh with per-vertex region
colors (mesh-name rules plus a bind-space height fallback) multiplied over the
source texture. The native GX renderer cannot see mesh names, so this script
rasterizes every mesh's tint into its texture's UV space and writes the product
as a replacement PNG named by the renderer's xxhash64 key (texture_pack.rs).
Colors are added inspection art direction, not recovered artwork. Strongly red
texels (blood, UI accents) keep their source value.

Stage scenery (SCR) has no explorer palette and only generic mesh names, so
--scenery colors each SCR texture by how the world uses it instead: area-weighted
ground/wall/roof from mesh normals and height, alpha cut-outs as accents, with a
shadow-to-highlight gradient. Hues avoid blood red so blood still reads.
It also tints other TPLs embedded in stage banks (signs, plaques) the same way,
but skips white silhouette masks: effect sheets, HUD glyphs and fonts get their
color per draw from the game, and blood takes its red that way.

HUD and effect sheets are white masks the game colors per draw (konst x texture),
which is also how blood and the red UI get their red. --fx-hud multiplies those
masks by hues whose red channel is exactly 1.0: a pure-red draw is then unchanged
bit for bit, while white and grey draws take the hue. HUD hues follow the sprite
groups in research/ui-live-core-layout.json. Textures that already carry color stay.

--legacy bakes the v1 look (explorer palette multiplied over the texture, ink kept,
cool scenery hues) into color-pack-legacy, for the frontend's legacy toggle.

usage: python tools/bake_native_color.py [--legacy] [--models] [--scenery] [--fx-hud] [explorer_root] [out_root]
       (no pass flag runs every pass)
"""
import hashlib, json, struct, sys
from pathlib import Path
import numpy as np, xxhash
from PIL import Image

ARGS = [a for a in sys.argv[1:] if not a.startswith('--')] if __name__ == '__main__' else []
FLAGS = {a for a in sys.argv[1:] if a.startswith('--')} if __name__ == '__main__' else set()
EXPLORER = Path(ARGS[0] if len(ARGS) > 0 else 'D:/Games/gamesfordecomp/exports/madworld')
LEGACY = '--legacy' in FLAGS
OUT = Path(ARGS[1] if len(ARGS) > 1 else Path(__file__).resolve().parents[1]
           / ('native-host-data/color-pack-legacy' if LEGACY else 'native-host-data/color-pack')) / 'RZZE8P'
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, mdb, tpl  # noqa: E402

PALETTES = {  # mirrors color.js
    'em0440': dict(skin='#eeb897', cloth='#c93248', accent='#e3ac38', hair='#263548'),
    'em0610': dict(skin='#dca780', cloth='#354d69', accent='#c287ae', hair='#d6b36a'),
    'em0630': dict(skin='#c6a6cd', cloth='#77344e', accent='#b99756', hair='#382943'),
    'em0920': dict(skin='#efc4ae', cloth='#b74765', accent='#d7b45d', hair='#e5c377'),
    'em0925': dict(skin='#e5bdac', cloth='#5b7ea5', accent='#d8b454', hair='#b98548'),
    'em0930': dict(skin='#b17859', cloth='#5d9b98', accent='#d9ac4e', hair='#2b2734'),
    'em0935': dict(skin='#b17859', cloth='#5d9b98', accent='#d9ac4e', hair='#2b2734'),
    'em0960': dict(skin='#f1c7b4', cloth='#9073b4', accent='#ce6688', hair='#283342'),
    'pl': dict(skin='#d9aa83', cloth='#718590', accent='#b19b62', hair='#554d42'),
}
import re, colorsys  # noqa: E402


def srgb(h):
    return np.array([int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)])


# v2 only: Asian Town street enemies (incl. the fire-breather) get strong outfits that
# stand apart from the warm streets, Lost Paradise-style.
PALETTES_V2 = {
    'em0100': dict(skin='#d9a27a', cloth='#2c7a86', accent='#e0b040', hair='#1e2230'),
    'em0101': dict(skin='#d39a72', cloth='#7e3a8a', accent='#e8a838', hair='#22182a'),
}


def palette(stem):
    key = 'pl' if stem.startswith('pl') else stem
    if not LEGACY and key in PALETTES_V2:
        return {k: srgb(v) for k, v in PALETTES_V2[key].items()}
    if key in PALETTES:
        return {k: srgb(v) for k, v in PALETTES[key].items()}
    h = 0
    for c in stem:
        h = (h * 31 + ord(c)) & 0xffffffff
    # THREE.Color.setHSL(h, .42, .62) in sRGB working space (legacy); v2 uses a deeper,
    # stronger outfit so unreviewed enemies don't read washed-out.
    # three.js setHSL works in linear space; convert to sRGB like the texture
    lin = np.array(colorsys.hls_to_rgb((h % 360) / 360, .62, .42) if LEGACY else colorsys.hls_to_rgb((h % 360) / 360, .45, .6))
    cloth = np.where(lin <= .0031308, lin * 12.92, 1.055 * lin ** (1 / 2.4) - .055)
    return dict(skin=srgb('#dab69a'), cloth=cloth, accent=srgb('#cdab61'), hair=srgb('#42445b'))


def vertex_colors(mesh, p, box):
    name = mesh['name'].lower()
    lo, hi = box
    height = max(.001, hi[1] - lo[1]); center = (lo[0] + hi[0]) / 2
    pos = np.asarray(mesh['positions'], float).reshape(-1, 3)
    y = (pos[:, 1] - lo[1]) / height; x = np.abs(pos[:, 0] - center) / height
    out = np.tile(p['cloth'], (len(pos), 1))
    fixed = None
    if re.search(r'hair|heir', name): fixed = p['hair']
    elif re.search(r'^(ha_|ha\b|sirome|medama)|tooth', name): fixed = srgb('#ece7dc')
    elif re.search(r'kuchi|danmen', name): fixed = srgb('#b66c71')
    elif re.search(r'skin|hada|hand|face|arm|mimi|mune|head_sb', name): fixed = p['skin']
    elif re.search(r'kazari|akuse|acce|ring|backle|buckle|kanagu|kushi|spike|tsuno|kougu', name): fixed = p['accent']
    elif re.search(r'goggles|sunglasses', name): fixed = srgb('#5a8c96')
    elif re.search(r'belt|leather|kutsu|kutu|geta|pants', name): fixed = p['hair']
    elif re.search(r'obi|ribon|himo|oogi', name): fixed = p['accent']
    elif re.search(r'^_m\d|^body_sb|^leg_sb', name):
        skin = (y > .84) | ((y > .46) & (x > .19))
        out[skin] = p['skin']; out[~skin & (y < .08)] = p['hair']
    if fixed is not None: out[:] = fixed
    if re.search(r'head_sb|face', name): out[y > .965] = p['hair']
    return out


def rasterize(acc, cnt, uvs, cols, idx):
    h, w = cnt.shape
    for t in range(0, len(idx) - 2, 3):
        a, b, c = idx[t:t + 3]
        P = np.array([uvs[a], uvs[b], uvs[c]], float) * (w, h)
        C = np.array([cols[a], cols[b], cols[c]])
        off = np.floor(P.min(0) / (w, h)) * (w, h); P -= off
        x0, y0 = np.floor(P.min(0)).astype(int); x1, y1 = np.ceil(P.max(0)).astype(int)
        if x1 - x0 > 4 * w or y1 - y0 > 4 * h: continue
        xs, ys = np.meshgrid(np.arange(x0, x1 + 1) + .5, np.arange(y0, y1 + 1) + .5)
        d = (P[1, 1] - P[2, 1]) * (P[0, 0] - P[2, 0]) + (P[2, 0] - P[1, 0]) * (P[0, 1] - P[2, 1])
        if abs(d) < 1e-9:
            xs = np.array([P[:, 0].mean()]); ys = np.array([P[:, 1].mean()]); l = np.array([[1 / 3, 1 / 3, 1 / 3]])
        else:
            l0 = ((P[1, 1] - P[2, 1]) * (xs - P[2, 0]) + (P[2, 0] - P[1, 0]) * (ys - P[2, 1])) / d
            l1 = ((P[2, 1] - P[0, 1]) * (xs - P[2, 0]) + (P[0, 0] - P[2, 0]) * (ys - P[2, 1])) / d
            l = np.stack([l0, l1, 1 - l0 - l1], -1)
            inside = (l >= -.02).all(-1)
            if not inside.any():  # sliver: take the centroid texel
                xs = np.array([P[:, 0].mean()]); ys = np.array([P[:, 1].mean()]); l = np.array([[1 / 3, 1 / 3, 1 / 3]])
            else:
                xs, ys, l = xs[inside], ys[inside], l[inside]
        px = np.floor(xs).astype(int) % w; py = np.floor(ys).astype(int) % h
        np.add.at(acc, (py, px), l.clip(0) @ C)
        np.add.at(cnt, (py, px), l.clip(0).sum(-1))


def tint_map(acc, cnt, fallback):
    covered = cnt > 0
    tint = np.where(covered[..., None], acc / np.maximum(cnt, 1e-9)[..., None], 0)
    for _ in range(24):  # grow coverage into UV gutters so bilinear/mip edges stay tinted
        if covered.all(): break
        s = np.zeros_like(tint); n = np.zeros(cnt.shape)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            s += np.roll(np.where(covered[..., None], tint, 0), (dy, dx), (0, 1))
            n += np.roll(covered, (dy, dx), (0, 1))
        grow = ~covered & (n > 0)
        tint[grow] = s[grow] / n[grow][:, None]; covered = covered | grow
    tint[~covered] = fallback
    return tint


BLOCK = {0: (8, 8, 32), 1: (8, 4, 32), 2: (8, 4, 32), 3: (4, 4, 32), 4: (4, 4, 32), 5: (4, 4, 32), 6: (4, 4, 64),
         8: (8, 8, 32), 9: (8, 4, 32), 10: (4, 4, 32), 14: (8, 8, 32)}


def names(raw, index):
    magic, count, table = struct.unpack_from('>3I', raw, 0)
    header, pal = struct.unpack_from('>II', raw, table + index * 8)
    height, width, fmt, p = struct.unpack_from('>HHII', raw, header)
    bw, bh, bb = BLOCK[fmt]
    data = raw[p:p + -(-width // bw) * -(-height // bh) * bb]
    digest = xxhash.xxh64_intdigest(data, 0)
    suffix = ''
    if fmt in (8, 9, 10):
        n, _, _, _, po = struct.unpack_from('>HBBII', raw, pal)
        if fmt == 8: used = [v for b in data for v in (b >> 4, b & 15)]
        elif fmt == 9: used = list(data)
        else: used = [v & 0x3fff for v, in struct.iter_unpack('>H', data)]
        lo, hi = min(used), max(used)
        suffix = '_%016x' % xxhash.xxh64_intdigest(raw[po + 2 * lo:po + 2 * hi + 2], 0)
    return [f'tex1_{width}x{height}{m}_{digest:016x}{suffix}_{fmt}' for m in ('', '_m')]


def main():
    catalog = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    banks = {b['id']: b for b in catalog['banks']}
    OUT.mkdir(parents=True, exist_ok=True)
    tints = {}  # (bank, tpl, index) -> [acc, cnt, image, raw, fallback, models]
    failures = []
    for item in catalog['models']:
        if not item.get('texture'): continue
        bank = banks[item['bank']]
        entry = next((e for e in bank['entries'] if e['name'] == item['texture'] and e['size']), None)
        if not entry: continue
        with open(item.get('sourcePath') or bank['path'], 'rb') as f:
            f.seek(item['offset']); data = f.read(item['size'])
            f.seek(entry['offset']); raw = f.read(entry['size'])
        try:
            model = mdb(data)
        except Exception as ex:
            failures.append((item['id'], str(ex))); continue
        stem = re.sub(r'_LOD\d$', '', item['name'].split('.')[0])
        p = palette(stem)
        pos = np.concatenate([np.asarray(m['positions'], float).reshape(-1, 3) for m in model['meshes'] if m['positions']] or [np.zeros((1, 3))])
        box = (pos.min(0), pos.max(0))
        for mesh in model['meshes']:
            if mesh['texture'] < 0 or not mesh['uvs'] or len(mesh['uvs']) != len(mesh['positions']): continue
            key = (item['bank'], item['texture'], mesh['texture'])
            if key not in tints:
                try:
                    image, meta = tpl(raw, mesh['texture'])[0]
                except Exception as ex:
                    failures.append((f'{key}', str(ex))); tints[key] = None; continue
                shape = (meta['height'], meta['width'])
                tints[key] = [np.zeros(shape + (3,)), np.zeros(shape), image, raw, p['cloth'], set(), np.zeros(shape + (3,)), np.zeros(shape)]
            t = tints[key]
            if t is None: continue
            # Skeleton (x-ray) parts share UV space with the visible body; they only
            # fill texels no visible mesh covers, so they don't wash out its regions.
            hidden = re.search(r'_hn_', mesh['name'].lower()) is not None
            acc, cnt = (t[6], t[7]) if hidden else (t[0], t[1])
            rasterize(acc, cnt, mesh['uvs'], vertex_colors(mesh, p, box), mesh['indices'])
            t[5].add(item['id'])
    written, index = 0, {}
    for (bank, tex, i), t in tints.items():
        if t is None: continue
        acc, cnt, image, raw, fallback, models, acc2, cnt2 = t
        free = cnt == 0
        acc[free] = acc2[free]; cnt[free] = cnt2[free]
        src = np.asarray(image.convert('RGBA'), float)
        tint = tint_map(acc, cnt, fallback)
        rgb = src[..., :3]
        red = (rgb[..., 0] - rgb[..., 1:].max(-1)) > 60
        lum = (rgb @ [.299, .587, .114] / 255)[..., None] ** .9
        out = np.where(red[..., None], rgb, rgb * tint if LEGACY else tint * 255 * (.22 + .78 * lum))  # v2: ink -> dark tint
        img = Image.fromarray(np.dstack([out, src[..., 3]]).round().clip(0, 255).astype(np.uint8), 'RGBA')
        for name in names(raw, i):
            img.save(OUT / f'{name}.png'); index[name] = dict(bank=bank, tpl=tex, index=i, models=sorted(models))
        written += 1
    (OUT.parent / 'index.json').write_text(json.dumps(dict(
        provenance='Added inspection palette v1 baked from exports/madworld/color.js; not recovered artwork',
        textures=written, names=index, failures=failures), indent=1), encoding='utf-8')
    print(json.dumps(dict(textures=written, pngs=len(index), failures=len(failures))))


# Scenery hues: muted, non-red (blood keeps its own red). (shadow, highlight) pairs.
# Asian Town palette v2, Lost Paradise-style: no pure black. Each (shadow, light) pair
# is a gradient map, so ink becomes the material's dark tone instead of black.
SCENERY = {
    'ground': ('#3a3029', '#c9b393'),   # worn stone flags
    'roof': ('#183329', '#4f9070'),     # jade glazed tile
    'roof2': ('#262a33', '#7d8796'),    # grey clay tile
    'accent': ('#4a1c0c', '#f08a3c'),   # lanterns, rails, trims (orange, kept apart from blood red)
}
# Weighted: plaster and teak dominate, lacquer accents stay rare so blood still pops.
WALLS = [('#3b3226', '#e6d4b0'), ('#3b3226', '#e6d4b0'), ('#2b1a10', '#a06a42'), ('#2b1a10', '#a06a42'),
         ('#173329', '#62a985'), ('#3a280c', '#deaa4a'), ('#3a120c', '#c24a2c'), ('#1d1e33', '#7076aa')]
# Signs, billboards, posters: neon.
NEON = [('#2a0c22', '#e85aaa'), ('#0c2a2c', '#48d2d0'), ('#2e2008', '#f2c24a'), ('#0e2a14', '#64d27c')]
if LEGACY:  # v1: cool varied hues, one roof tile, walls palette reused for embedded art
    SCENERY = {'ground': ('#5b5e66', '#cdbb9c'), 'roof': ('#24394a', '#6f9a93'), 'roof2': ('#24394a', '#6f9a93'),
               'accent': ('#6b4a1c', '#e8bd5a')}
    WALLS = [('#3b3550', '#d7b98a'), ('#30443f', '#9fc0a6'), ('#3d3a2e', '#d9c27e'),
             ('#2f3a4f', '#a3b4cf'), ('#43354a', '#c3a9c9'), ('#4a3a2c', '#c8a27a')]
    NEON = WALLS


def scenery_tint(src, cls):
    """Gradient map on luminance: ink -> the material's shadow tone, paper -> its light tone."""
    lo, hi = (srgb(c) for c in cls)
    t = np.clip((src[..., :3] @ [.299, .587, .114]) / 255, 0, 1)[..., None] ** .8
    if LEGACY:  # v1: multiply, ink stays black
        return src[..., :3] * (lo * (1 - t) + hi * t)
    return (lo * (1 - t) + hi * t) * 255


def scr_usage(scr, textures):
    """Area-weighted (ground, wall, roof) per SCR texture index; SCR transforms are identity."""
    count = struct.unpack_from('>I', scr, 4)[0]
    rows = [16 + i * 96 + struct.unpack_from('>I', scr, 16 + i * 96 + 48)[0] for i in range(count)]
    starts = sorted(set(rows) | {textures})
    meshes = []
    for start in rows:
        try:
            model = mdb(scr[start:starts[starts.index(start) + 1]])
        except Exception:
            continue
        for mesh in model['meshes']:
            if mesh['texture'] < 0 or not mesh['indices']: continue
            pos = np.asarray(mesh['positions'], float).reshape(-1, 3); idx = np.asarray(mesh['indices']).reshape(-1, 3)
            a, b, c = pos[idx[:, 0]], pos[idx[:, 1]], pos[idx[:, 2]]
            n = np.cross(b - a, c - a); length = np.linalg.norm(n, axis=1)
            meshes.append((mesh['texture'], length / 2, np.abs(n[:, 1]) / np.maximum(length, 1e-9), (a[:, 1] + b[:, 1] + c[:, 1]) / 3))
    usage = {}
    if not meshes: return usage
    ground_top = np.percentile(np.concatenate([m[3] for m in meshes]), 20) + 1.0
    for t, area, ny, y in meshes:
        up = ny > .7
        usage.setdefault(t, np.zeros(3))
        usage[t] += [area[up & (y <= ground_top)].sum(), area[~up].sum(), area[up & (y > ground_top)].sum()]
    return usage


TPL_MAGIC = bytes.fromhex('0020af30')


def scenery():
    files = EXPLORER / 'extracted/disc/DATA/files'
    index, written = {}, 0
    models = OUT.parent / 'index.json'  # explorer-palette textures win over scenery tints
    owned = set(json.loads(models.read_text(encoding='utf-8'))['names']) if models.exists() else set()
    for dat in sorted(files.glob('case*/*.dat')):
        raw = dat.read_bytes()
        try:
            entries = dat_entries(raw)
        except Exception:
            continue
        for e in entries:
            scr = raw[e['offset']:e['offset'] + e['size']]
            if not e['name'].endswith('.scr') or scr[:4] != b'SCR\0': continue
            textures = struct.unpack_from('>I', scr, 8)[0]
            block = scr[textures:]
            try:
                images = tpl(block)
            except Exception as ex:
                print('skip', e['name'], ex); continue
            usage = scr_usage(scr, textures)
            for i, (image, meta) in enumerate(images):
                ground, wall, roof = usage.get(i, np.zeros(3))
                if meta['format'] in (2, 3):
                    cls = SCENERY['accent']
                elif ground + wall + roof == 0 or wall > max(ground, roof):
                    cls = WALLS[xxhash.xxh32_intdigest(f"{e['name']}{i}".encode()) % len(WALLS)]
                elif roof >= ground:
                    cls = SCENERY['roof' if xxhash.xxh32_intdigest(f"{e['name']}{i}".encode()) % 2 else 'roof2']
                else:
                    cls = SCENERY['ground']
                src = np.asarray(image.convert('RGBA'), float)
                rgb = src[..., :3]
                red = (rgb[..., 0] - rgb[..., 1:].max(-1)) > 60
                out = np.where(red[..., None], rgb, scenery_tint(src, cls))
                img = Image.fromarray(np.dstack([out, src[..., 3]]).round().clip(0, 255).astype(np.uint8), 'RGBA')
                for name in names(block, i):
                    if name in index or name in owned: continue  # shared sheet, or a model texture
                    img.save(OUT / f'{name}.png')
                    index[name] = dict(scr=f"{dat.relative_to(files).as_posix()}:{e['name']}", index=i,
                                       use=[round(float(v), 1) for v in (ground, wall, roof)])
                written += 1
    # Other TPLs embedded in stage-bank members (signs, plaques) that no pass above covered.
    # Only ink-style art (clear black and white areas) is tinted; soft particles, white
    # silhouette masks, _AB ink thresholds and stage UI pages (.idd) are left as they are.
    for dat in sorted(files.glob('case*/*.dat')):
        raw = dat.read_bytes()
        try:
            entries = dat_entries(raw)
        except Exception:
            continue
        for e in entries:
            if '_AB' in e['name'] or not e['name'].endswith(('.tpl', '.eff', '.imt', '.mtd')): continue
            member = raw[e['offset']:e['offset'] + e['size']]; pos = 0
            while (pos := member.find(TPL_MAGIC, pos)) >= 0:
                block, at = member[pos:], pos; pos += 32
                try:
                    count, table = struct.unpack_from('>II', block, 4)
                    if not 0 < count < 512 or table != 12: continue
                    images = tpl(block)
                except Exception:
                    continue
                for i, (image, meta) in enumerate(images):
                    todo = [n for n in names(block, i) if n not in index and n not in owned]
                    if not todo: continue
                    src = np.asarray(image.convert('RGBA'), float)
                    lum = (src[..., :3] @ [.299, .587, .114])[src[..., 3] > 128]
                    if lum.size == 0 or (lum < 40).mean() < .15 or (lum > 200).mean() < .15: continue
                    cls = NEON[xxhash.xxh32_intdigest(e['name'].encode()) % len(NEON)]  # one hue per member: animation frames stay steady
                    rgb = src[..., :3]
                    red = (rgb[..., 0] - rgb[..., 1:].max(-1)) > 60
                    out = np.where(red[..., None], rgb, scenery_tint(src, cls))
                    img = Image.fromarray(np.dstack([out, src[..., 3]]).round().clip(0, 255).astype(np.uint8), 'RGBA')
                    for name in todo:
                        img.save(OUT / f'{name}.png')
                        index[name] = dict(scr=f"{dat.relative_to(files).as_posix()}:{e['name']}@{at}", index=i, use='embedded')
                    written += 1
    (OUT.parent / 'scenery-index.json').write_text(json.dumps(dict(
        provenance='Added scenery palette v1 by SCR geometry usage class; not recovered artwork',
        textures=written, names=index), indent=1), encoding='utf-8')
    print(json.dumps(dict(scenery_textures=written, pngs=len(index))))


# Red channel is 1.0 in every hue, so red konst draws (blood, red UI) are unchanged.
HUD_HUES = ['#ffd36b', '#ffae5c', '#fff0c8', '#ffc98f', '#ffc2cf']
FX_HUES = ['#ffd36b', '#ffae5c', '#ffc98f', '#ffe6b0']


def red_neutral(src, hue):
    """Multiply by a red-neutral hue map (H x W x 3, red == 1); texels with color already pass through."""
    rgb = src[..., :3]
    colored = (rgb.max(-1) - rgb.min(-1)) > 40
    return np.where(colored[..., None], rgb, rgb * hue)


def fx_hud():
    files = EXPLORER / 'extracted/disc/DATA/files'
    taken = set()
    for name in ('index.json', 'scenery-index.json'):
        f = OUT.parent / name
        if f.exists(): taken |= set(json.loads(f.read_text(encoding='utf-8'))['names'])
    index, written = {}, 0

    def emit(block, i, image, hue, label):
        nonlocal written
        todo = [n for n in names(block, i) if n not in taken and n not in index]
        if not todo: return
        src = np.asarray(image.convert('RGBA'), float)
        if ((src[..., :3].max(-1) - src[..., :3].min(-1))[src[..., 3] > 32] > 40).mean() > .25: return  # already color art
        out = red_neutral(src, hue)
        img = Image.fromarray(np.dstack([out, src[..., 3]]).round().clip(0, 255).astype(np.uint8), 'RGBA')
        for n in todo:
            img.save(OUT / f'{n}.png'); index[n] = dict(source=label, index=i)
        written += 1

    # HUD: core_us.idd pages, one hue per sprite group, cream elsewhere.
    layout = json.loads((Path(__file__).resolve().parents[1] / 'research/ui-live-core-layout.json').read_text(encoding='utf-8'))
    # The live game reads the PC-staged copy (staged/pc-ui, tools/stage_pc_ui.py); same layout.
    staged = Path(__file__).resolve().parents[1] / 'staged/pc-ui'
    for idd in [files / 'etc/core_us.idd', staged / 'core_us.idd']:
        if not idd.exists(): continue
        block = idd.read_bytes()[4096:]
        for i, (image, meta) in enumerate(tpl(block)):
            hue = np.tile(srgb('#fff0c8'), (meta['height'], meta['width'], 1))
            for sp in layout['sprites']:
                if sp['texturePage'] != i: continue
                x0, y0, x1, y1 = sp['rect']
                hue[max(y0, 0):y1, max(x0, 0):x1] = srgb(HUD_HUES[sp['group'] % len(HUD_HUES)])
            emit(block, i, image, hue, f'{idd.parent.name}/{idd.name}#{i}')
    # Effects: TPLs in the core banks and stage effect/UI members; one hue per source.
    # Shadow/noise sheets are lighting inputs, not art: tinting them would warm every shadow.
    sources = []
    for dat in [files / 'etc' / n for n in ('core2.dat', 'core2_us.dat', 'core.dat', 'core_us.dat')] + [staged / 'core2_us.dat'] + sorted(files.glob('case*/*.dat')):
        if not dat.exists(): continue
        raw = dat.read_bytes()
        try:
            entries = dat_entries(raw)
        except Exception:
            continue
        stage = dat.parent.name not in ('etc', 'pc-ui')
        where = dat.relative_to(files).as_posix() if dat.is_relative_to(files) else f'staged/{dat.parent.name}/{dat.name}'
        sources += [(f"{where}:{e['name']}", raw[e['offset']:e['offset'] + e['size']]) for e in entries
                    if '_AB' not in e['name'] and not any(k in e['name'].lower() for k in ('shadow', 'noise'))
                    and (e['name'].endswith(('.eff', '.idd')) if stage else e['name'].endswith(('.tpl', '.eff', '.idd')))]
    # Stage UI pages (e.g. the challenge prompt) live in per-stage idpac containers.
    sources += [(f.relative_to(files).as_posix(), f.read_bytes()) for f in sorted(files.glob('case*/idpac_*_us.dat'))]
    for label, data in sources:
        pos = 0
        while (pos := data.find(TPL_MAGIC, pos)) >= 0:
            block, at = data[pos:], pos; pos += 32
            try:
                count, table = struct.unpack_from('>II', block, 4)
                if not 0 < count < 512 or table != 12: continue
                images = tpl(block)
            except Exception:
                continue
            for i, (image, meta) in enumerate(images):
                hue = np.tile(srgb(FX_HUES[xxhash.xxh32_intdigest(f'{label}{at}{i}'.encode()) % len(FX_HUES)]), (meta['height'], meta['width'], 1))
                emit(block, i, image, hue, f'{label}@{at}#{i}')
    (OUT.parent / 'fxhud-index.json').write_text(json.dumps(dict(
        provenance='Added red-neutral HUD/FX hues v1 (red channel 1.0); not recovered artwork',
        textures=written, names=index), indent=1), encoding='utf-8')
    print(json.dumps(dict(fx_hud_textures=written, pngs=len(index))))


if __name__ == '__main__':
    OUT.mkdir(parents=True, exist_ok=True)
    every = not FLAGS & {'--models', '--scenery', '--fx-hud'}
    if '--models' in FLAGS or every:
        main()
    if '--scenery' in FLAGS or every:
        scenery()
    if '--fx-hud' in FLAGS or every:
        fx_hud()
