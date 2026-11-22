"""Stage the town's civilians [added] (MW_STAGE=town, play_town.rs).

The ending crowd (em0915 / em0925 / em0935 / em0955, ev79-ev84) is drawn natively as blank white figures: each bank's
TPL is a 320-byte placeholder and most parts' UVs collapse to one texel. Each is a trimmed copy of a textured sibling
(em0910 / em0920 / em0930 / em0950), so the parts get the sibling's look:
- a part with the same name and vertex count as a sibling part takes its UVs and texture as they are;
- any other part takes, per triangle, the texture the sibling's nearest surface has there and, per corner, the UV of the
  nearest sibling vertex on that texture (position + normal), de-indexed so no triangle straddles two textures.
The sibling's textures are packed side by side into one atlas. Both looks are written (source: the ink texture; color:
the same over the explorer's vertex palette), plus a rig file for play_street.rs: the bones' bind offsets, the root
scale against em0101 (hip height), the head parts (for a head blown off) and the skin parts (kept when clothes are
re-tinted per civilian).

Run: python -I tools/town/stage_civilians.py   (needs the explorer at D:/Games/gamesfordecomp/exports/madworld)
"""
from pathlib import Path
import io, json, sys
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from native import dat_entries, mdb, tpl  # noqa: E402
from glbio import Glb  # noqa: E402

FILES = EXPLORER / 'extracted/disc/DATA/files/em'
EV84 = ROOT / 'assets/native/events/ev84'
OUT = ROOT / 'assets/native/civ'
CIVS = [('em0915', 'em0910'), ('em0925', 'em0920'), ('em0935', 'em0930'), ('em0955', 'em0950')]
SKIN = ('hand', 'head', 'face', 'kuchi', 'kuti', 'kutinaka', 'ha_', 'ha', 'skin', 'leg', 'teaf', 'matuge')


def entry(stem, name):
    raw = (FILES / f'{stem}.dat').read_bytes()
    e = next(x for x in dat_entries(raw) if x['name'] == name)
    return raw[e['offset']:e['offset'] + e['size']]


def key(name):
    return name.rsplit('_', 1)[0]


def is_skin(name):
    k = key(name).lower()
    return any(w in k for w in ('hand', 'head', 'face', 'skin', 'leg', 'matuge')) or k in ('kuchi', 'kuti', 'kutinaka', 'ha', 'ha_d', 'ha_u', 'teaf')


def nearest(q, qn, p, pn, chunk=512):
    """index into p of the nearest point to each q (distance, plus a penalty for facing the other way)"""
    out = np.empty(len(q), dtype=np.int64)
    for a in range(0, len(q), chunk):
        d = ((q[a:a + chunk, None, :] - p[None, :, :]) ** 2).sum(-1) ** 0.5
        d += 0.03 * (1. - (qn[a:a + chunk, None, :] * pn[None, :, :]).sum(-1))
        out[a:a + chunk] = d.argmin(1)
    return out


def kind(name):
    """what a part is, by its native name: hair, hand, face, teeth, mouth (inside) or other"""
    k = key(name).lower()
    if 'sunglass' in k or 'glass' in k: return 'glasses'
    if k.startswith('ha_') or k == 'ha': return 'teeth'
    if k in ('kuchi', 'kuti', 'kutinaka'): return 'mouth'
    if 'hair' in k or 'kami' in k: return 'hair'
    if 'hand' in k: return 'hand'
    if 'head' in k or 'face' in k: return 'face'
    return 'other'


def set_colour(g, prim, colours):
    """the primitive's COLOR_0 replaced (float RGBA)"""
    c = np.ascontiguousarray(np.clip(colours, 0, 1), dtype=np.float32)
    g.j['accessors'].append({'bufferView': g.add_view(c.tobytes(), 34962), 'byteOffset': 0, 'componentType': 5126, 'count': len(c), 'type': 'VEC4'})
    prim['attributes']['COLOR_0'] = len(g.j['accessors']) - 1


def stage(n, civ, donor):
    cm, dm = mdb(entry(civ, civ + '.mdb')), mdb(entry(donor, donor + '.mdb'))
    images = [im[0].convert('RGBA') for im in tpl(entry(donor, donor + '.tpl'))]
    count = len(images)
    atlas = Image.new('RGBA', (512 * count, 512))
    for k, im in enumerate(images):
        atlas.paste(im.resize((512, 512), Image.LANCZOS), (512 * k, 0))
    buf = io.BytesIO(); atlas.convert('RGB').save(buf, 'PNG'); png = buf.getvalue()
    byname = {}
    for m in dm['meshes']: byname.setdefault(key(m['name']), m)
    # the sibling's colour look (the explorer's vertex palette, per vertex: skin, cloth, hair), when it was exported
    dcol = {}
    for folder in ('ev84', 'ev93'):
        cg = ROOT / f'assets/native/events/{folder}/{donor}-color.glb'
        if cg.is_file() and not dcol:
            g = Glb(cg)
            for i, mi in g.mesh_prims().items():
                a_ = g.j['meshes'][mi]['primitives'][0]['attributes'].get('COLOR_0')
                if a_ is not None:
                    c = g.acc(a_).astype(np.float64)
                    if c.max() > 1.5: c = c / (65535. if c.max() > 256 else 255.)
                    dcol[i] = c[:, :4] if c.shape[1] == 4 else np.concatenate([c, np.ones((len(c), 1))], 1)

    def to_atlas(uv, tex):
        uv = np.clip(np.asarray(uv, dtype=np.float64), 0.002, 0.998)
        return np.stack([(uv[:, 0] + tex) / count, uv[:, 1]], 1)

    def fit(a, ref):
        """a's points scaled per axis into ref's box"""
        lo, hi, rlo, rhi = a.min(0), a.max(0), ref.min(0), ref.max(0)
        return rlo + (a - lo) / np.maximum(hi - lo, 1e-6) * (rhi - rlo)

    # the sibling's triangles (boxed onto this body): corners, face normal, corner UVs, texture, corner colours
    cp_all = np.array([p for m in cm['meshes'] for p in m['positions']], dtype=np.float64)
    dp_all = np.array([p for m in dm['meshes'] if m['uvs'] and m['texture'] >= 0 for p in m['positions']], dtype=np.float64)
    lo, hi, rlo, rhi = dp_all.min(0), dp_all.max(0), cp_all.min(0), cp_all.max(0)
    box = lambda a: rlo + (a - lo) / np.maximum(hi - lo, 1e-6) * (rhi - rlo)
    T, TU, TT, TC, TK, TM = [], [], [], [], [], []
    for mi, m in enumerate(dm['meshes']):
        if not m['uvs'] or m['texture'] < 0: continue
        P = box(np.array(m['positions'], dtype=np.float64)); U = np.array(m['uvs'], dtype=np.float64) % 1.
        C = dcol.get(mi, np.ones((len(P), 4)))
        I = np.array(m['indices']).reshape(-1, 3)
        T.append(P[I]); TU.append(U[I]); TT.append(np.full(len(I), m['texture'])); TC.append(C[I]); TK.append(np.full(len(I), kind(m['name']))); TM.append(np.full(len(I), mi))
    T, TU, TT, TC, TK, TM = np.concatenate(T), np.concatenate(TU), np.concatenate(TT), np.concatenate(TC), np.concatenate(TK), np.concatenate(TM)
    # flat texels for the inside of the mouth: the atlas's whitest and darkest spots (teeth / mouth)
    grey = np.asarray(atlas.convert('L').resize((64 * count, 64)), dtype=np.float64)
    def texel(pick):
        y, x = np.unravel_index(pick(grey), grey.shape)
        return np.array([(x + 0.5) / grey.shape[1], (y + 0.5) / grey.shape[0]])
    WHITE, DARK = texel(np.argmax), texel(np.argmin)
    TN = np.cross(T[:, 1] - T[:, 0], T[:, 2] - T[:, 0]); TN /= np.maximum(np.linalg.norm(TN, axis=1, keepdims=True), 1e-12)
    TCEN = T.mean(1)

    def onto(points, tri):
        """barycentrics (u, v) of points on the plane of each matching triangle (affine: may lie outside)"""
        a, e1, e2 = tri[:, 0], tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0]
        d = points - a
        d00, d01, d11 = (e1 * e1).sum(1), (e1 * e2).sum(1), (e2 * e2).sum(1)
        d20, d21 = (d * e1).sum(1), (d * e2).sum(1)
        den = np.where(np.abs(d00 * d11 - d01 * d01) < 1e-14, 1e-14, d00 * d11 - d01 * d01)
        return (d11 * d20 - d01 * d21) / den, (d00 * d21 - d01 * d20) / den

    plans = []  # per civ mesh: ('copy', uv, None, colours) or ('split', uv per corner, corner order, colours per corner)
    for mi, m in enumerate(cm['meshes']):
        P = np.array(m['positions'], dtype=np.float64)
        o = next((x for x in dm['meshes'] if x['name'] == m['name']), None) or byname.get(key(m['name']))
        def same_topology(o):
            # equal counts are not enough: the triangles' edge lengths must agree too (pose changes them only a little)
            if len(o['positions']) != len(P) or len(o['indices']) != len(m['indices']): return False
            I = np.array(m['indices']).reshape(-1, 3); Q = np.array(o['positions'], dtype=np.float64)
            e = lambda X: np.linalg.norm(X[I[:, [1, 2, 0]]] - X[I], axis=2)
            ea, eb = e(P), e(Q)
            return float(np.median(np.abs(ea - eb) / np.maximum(ea, 1e-4))) < 0.06 and float(np.mean(np.abs(ea - eb) / np.maximum(ea, 1e-4) > 0.3)) < 0.05
        own = np.array(m['uvs'], dtype=np.float64) % 1. if m['uvs'] else None
        def shared_layout(o):
            # the part kept its own UVs, laid out as the sibling's part (only its texture is missing)
            if own is None or not o['uvs'] or np.ptp(own, 0).min() < 0.02: return False
            ou = np.array(o['uvs'], dtype=np.float64) % 1.
            lo_, hi_ = np.maximum(own.min(0), ou.min(0)), np.minimum(own.max(0), ou.max(0))
            inter = np.prod(np.maximum(hi_ - lo_, 0)); union = np.prod(np.ptp(own, 0)) + np.prod(np.ptp(ou, 0)) - inter
            return inter / max(union, 1e-9) > 0.8
        if o is not None and o['texture'] >= 0 and shared_layout(o) and not same_topology(o):
            # its own UVs on the sibling's texture; colours from the sibling part's nearest triangles
            oi = dm['meshes'].index(o)
            pool = np.where(TM == oi)[0]
            best = np.empty(len(P), dtype=np.int64)
            for k in range(0, len(P), 256):
                best[k:k + 256] = pool[np.linalg.norm(P[k:k + 256, None] - TCEN[pool][None], axis=2).argmin(1)]
            u, v = onto(P, T[best]); u, v = np.clip(u, 0, 1), np.clip(v, 0, 1); s_ = u + v; over = s_ > 1; u[over] /= s_[over]; v[over] /= s_[over]
            cw = TC[best]; col = cw[:, 0] + u[:, None] * (cw[:, 1] - cw[:, 0]) + v[:, None] * (cw[:, 2] - cw[:, 0])
            plans.append(('copy', to_atlas(own, o['texture']), None, col if dcol else None)); continue
        if o is not None and o['texture'] >= 0 and same_topology(o):
            # the same part in another pose: same topology, the same UVs and colours
            oi = dm['meshes'].index(o)
            plans.append(('copy', to_atlas([[u % 1., v % 1.] for u, v in o['uvs']], o['texture']), None, dcol.get(oi))); continue
        idx = np.array(m['indices']).reshape(-1, 3)
        k_ = kind(m['name'])
        if k_ in ('teeth', 'mouth', 'glasses'):
            # flat parts: the inside of the mouth (teeth white, the mouth dark), sunglasses black
            uv = np.tile(WHITE if k_ == 'teeth' else DARK, (len(idx) * 3, 1))
            col = np.tile({'teeth': [0.95, 0.93, 0.9, 1.], 'mouth': [0.25, 0.08, 0.08, 1.], 'glasses': [0.06, 0.06, 0.08, 1.]}[k_], (len(idx) * 3, 1))
            plans.append(('split', uv, idx.reshape(-1), col)); continue
        tri = P[idx]
        # same kind of part only (hair from hair, a hand from a hand, a face from a face), when the sibling has one
        pool = np.where(TK == k_)[0] if (TK == k_).sum() >= 8 and k_ != 'other' else np.arange(len(T))
        # (a part the sibling has under the same name maps onto that part only)
        if o is not None and o['texture'] >= 0 and abs(len(o['positions']) - len(P)) <= 0.1 * len(P) and (TM == dm['meshes'].index(o)).sum() >= 4: pool = np.where(TM == dm['meshes'].index(o))[0]
        cen = tri.mean(1)
        fn = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0]); fn /= np.maximum(np.linalg.norm(fn, axis=1, keepdims=True), 1e-12)
        # each triangle takes the nearest sibling triangle facing its way, and that triangle's mapping, whole
        best = np.empty(len(idx), dtype=np.int64)
        for k in range(0, len(idx), 256):
            d = np.linalg.norm(cen[k:k + 256, None] - TCEN[pool][None], axis=2) + 0.04 * (1. - np.abs((fn[k:k + 256, None] * TN[pool][None]).sum(2)))
            best[k:k + 256] = pool[d.argmin(1)]
        corners = tri.reshape(-1, 3)
        src = np.repeat(best, 3)
        u, v = onto(corners, T[src])
        # (a little extrapolation keeps the mapping affine across the triangle; far outside it is clamped)
        u, v = np.clip(u, -0.6, 1.6), np.clip(v, -0.6, 1.6)
        uvw = TU[src]
        uv = uvw[:, 0] + u[:, None] * (uvw[:, 1] - uvw[:, 0]) + v[:, None] * (uvw[:, 2] - uvw[:, 0])
        uv = to_atlas(uv, TT[src])
        # its colours: interpolated inside the sibling triangle (clamped to it)
        cu, cv = np.clip(u, 0, 1), np.clip(v, 0, 1); s_ = cu + cv; over = s_ > 1; cu[over] /= s_[over]; cv[over] /= s_[over]
        cw = TC[src]
        col = cw[:, 0] + cu[:, None] * (cw[:, 1] - cw[:, 0]) + cv[:, None] * (cw[:, 2] - cw[:, 0])
        plans.append(('split', uv, idx.reshape(-1), col if dcol else None))

    rig = None
    for look, src in (('source', EV84 / f'{civ}.glb'), ('color', EV84 / f'{civ}-color.glb')):
        g = Glb(src)
        g.j['images'] = [{'bufferView': g.add_view(png), 'mimeType': 'image/png'}]
        g.j['samplers'] = [{'magFilter': 9729, 'minFilter': 9987, 'wrapS': 33071, 'wrapT': 33071}]
        g.j['textures'] = [{'sampler': 0, 'source': 0}]
        for mat in g.j['materials']:
            mat.setdefault('pbrMetallicRoughness', {})['baseColorTexture'] = {'index': 0, 'texCoord': 0}
        mp = g.mesh_prims()
        for i, plan in enumerate(plans):
            prim = g.j['meshes'][mp[i]]['primitives'][0]
            colours = plan[3] if look == 'color' and 'COLOR_0' in prim['attributes'] else None
            if look == 'color' and colours is None and 'COLOR_0' in prim['attributes'] and kind(cm['meshes'][i]['name']) in ('face', 'hand'):
                # (no sibling palette: the face and hands take a skin tone instead of the part's flat colour)
                colours = np.tile([0.88, 0.68, 0.55, 1.], (len(plan[1]), 1))
            if plan[0] == 'copy':
                g.set_uv(prim['attributes']['TEXCOORD_0'], plan[1])
                if colours is not None: set_colour(g, prim, colours)
                continue
            _, uv, order, _ = plan
            # de-index: every attribute re-read in triangle-corner order, the UVs per corner
            for name, a in list(prim['attributes'].items()):
                if name == 'TEXCOORD_0': continue
                acc = g.j['accessors'][a]; data = g.acc(a)[order]
                view = g.add_view(np.ascontiguousarray(data).tobytes(), 34962)
                new = {k: v for k, v in acc.items() if k not in ('bufferView', 'byteOffset', 'count', 'min', 'max')}
                new.update(bufferView=view, byteOffset=0, count=len(order))
                if name == 'POSITION': new.update(min=data.min(0).tolist(), max=data.max(0).tolist())
                g.j['accessors'].append(new); prim['attributes'][name] = len(g.j['accessors']) - 1
            tacc = dict(g.j['accessors'][prim['attributes']['TEXCOORD_0']]); tacc.pop('min', None); tacc.pop('max', None)
            tacc['count'] = len(order); g.j['accessors'].append(tacc)
            prim['attributes']['TEXCOORD_0'] = len(g.j['accessors']) - 1
            g.set_uv(prim['attributes']['TEXCOORD_0'], uv)
            ind = np.arange(len(order), dtype=np.uint32)
            g.j['accessors'].append({'bufferView': g.add_view(ind.tobytes(), 34963), 'byteOffset': 0, 'componentType': 5125,
                                     'count': len(ind), 'type': 'SCALAR'})
            prim['indices'] = len(g.j['accessors']) - 1
            if colours is not None: set_colour(g, prim, colours)
        OUT.mkdir(parents=True, exist_ok=True)
        g.save(OUT / f'civ{n}-{look}.glb')
    bones = cm['bones']
    hip = next(b for b in bones if b['id'] == 0)['position'][1]
    grunt = json.loads((ROOT / 'assets/native/kungfu-motion.json').read_text())['bones']
    ghip = next(b for b in grunt if b['id'] == 0)['position'][1]
    # the head (bone 4) and every bone under it (parents are bone indices)
    head_ids = set()
    for i, b in enumerate(bones):
        k = i
        while k >= 0:
            if bones[k]['id'] == 4: head_ids.add(b['id']); break
            k = bones[k]['parent']
    head, skin = [], []
    for i, m in enumerate(cm['meshes']):
        w = np.zeros(4096)
        for js, ws in zip(m['joints'], m['weights']):
            for j, x in zip(js, ws):
                if 0 <= j < len(bones): w[bones[j]['id']] += x
        if w.sum() and w[[k for k in range(4096) if k in head_ids]].sum() / w.sum() > 0.6: head.append(i)
        if is_skin(m['name']): skin.append(i)
    rig = dict(object=civ, sibling=donor, rootScale=hip / ghip, hipHeight=hip,
               height=max(p[1] for m in cm['meshes'] for p in m['positions']),
               bones=[dict(id=b['id'], parent=b['parent'], relative=b['relative']) for b in bones],
               meshes=[m['name'] for m in cm['meshes']], head=head, skin=skin,
               plans=[p[0] for p in plans])
    (OUT / f'civ{n}-rig.json').write_text(json.dumps(rig, indent=1))
    print(civ, '->', f'civ{n}', dict(atlas=atlas.size, plans=rig['plans'], head=head, skin=skin, rootScale=round(rig['rootScale'], 3),
                                     height=round(rig['height'], 2)))


if __name__ == '__main__':
    for n, (civ, donor) in enumerate(CIVS, 1):
        stage(n, civ, donor)
