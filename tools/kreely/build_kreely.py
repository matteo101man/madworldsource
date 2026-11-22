"""Build Kreese Kreely for the model explorer on Jack's pl0030 skeleton (v2: own proportions + fingers).

python -I tools/kreely/build_kreely.py [--blender PATH]

Inputs (local, on D:):
  assets/characters/kreely/source/kreely.fbx + kreely.fbm/kreely_basecolor.jpg   (user-provided model)
  owned disc pl/pl0030.dat#pl0030.mdb, read through the explorer's own MDB reader (exports/madworld/builders/native.py)
Outputs (local, gitignored):
  assets/characters/kreely/build/kreely.model.json    explorer /api/model format
  assets/characters/kreely/build/kreely_basecolor.png  source texture + painted eyes (primary texture)
  assets/characters/kreely/build/kreely_color.png      Color-version texture (per-texel regions x source)
  assets/characters/kreely/build/kreely_AB.png         synthesized shadow-ink texture for the native ink TEV path
  assets/characters/kreely/build/kreely.build.json     landmarks, joints, weight report, hashes
  exports/madworld/custom/characters.json              explorer catalog hook (paths + hashes, no asset data)

Skeleton: pl0030's 36 bones with the same ids, parents and zero bind rotations, but **Kreely's own joint positions**
(Jack's joints mapped into Kreely's proportions). pl0030 MOTs move body joints by rotation only, so they drive this
skeleton directly; root translation (bones -1/0) is scaled by the hip-height ratio (retarget.rootScale).
Plus 30 finger bones (ids 0x600+) parented to the hand bones, posed by named hand poses (default: fist, like Jack).
"""
import argparse, hashlib, json, subprocess, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageFilter
from scipy.spatial import cKDTree
from scipy import ndimage

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import kreely_fingers as kfing
import kreely_pose as kpose
import kreely_gun

PORT = HERE.parents[1]
EXPLORER = PORT.parents[1] / 'exports' / 'madworld'
SRC = PORT / 'assets/characters/kreely/source'
OUT = PORT / 'assets/characters/kreely/build'
JACK_ID = 'pl/pl0030.dat#pl0030.mdb@19552'
BLENDER = r'C:\Program Files\Blender Foundation\Blender 4.4\blender.exe'

# Jack bones Kreely is heat-skinned to. Bones 6/11 (Jack's deltoid helpers) and 24 (jaw) are added after heat with
# Jack-like profiles. Never weighted: 0 root, 25 cigar, 26 brow helper, 40/41 eyes (Kreely's eyes are painted, no
# eyeball geometry), 30/31 Jack's pouch, 50-52 collar/back helpers, 100/101 weapon attach.
DEFORM = [1, 2, 3, 4, 5, 10, 7, 8, 9, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23]
SIDES = {'L': (1, 14), 'R': (-1, 9)}            # +x is the character's left (faces +Z); hand bone ids
FINGERS = ['thumb', 'index', 'middle', 'ring', 'pinky']
FINGER_ID0 = 0x600                              # unused by every MOT in the catalog (ids there: <=0x81, 0x400-0x403, 0xf60+)
REGIONS = ['suit', 'skin', 'hair', 'shirt', 'belt', 'buckle', 'shoes', 'sclera', 'iris', 'gunmetal']
PALETTE = dict(skin='#d69c76', suit='#4a5368', hair='#3a2a20', shirt='#ece6d6', belt='#3b2a22', buckle='#d0a646',
               shoes='#2c2422', sclera='#f4f1ea', iris='#8a5a32', gunmetal='#7d838c')   # must match color.js `kreely`
# Painted eyes [inferred, hand-placed from ortho renders of the source texture]: the FBX face has only squint slits.
# Units: fractions of his height, front projection; x is mirrored per side (|x|). See research/KREELY.md.
EYES = dict(cx=0.0185, cy=0.9135, half_w=0.0105, half_h=0.0028, tilt=0.0012, iris_r=0.0034, pupil_r=0.0014,
            min_z=0.02, lid=0.25)
HAND_POSES = {  # degrees per segment (base, middle, distal); thumb: (swing to palm, cross toward fingers), base, distal
    'fist': dict(fingers=(80, 100, 65), pinky_extra=8, thumb=(55, 30, 30, 45)),
    'relaxed': dict(fingers=(22, 32, 18), pinky_extra=6, thumb=(15, 10, 10, 12)),
    'open': dict(fingers=(0, 0, 0), pinky_extra=0, thumb=(0, 0, 0, 0)),
    # gun grips: trigger finger half curled, others wrapped round the grip [inferred]
    'gun': dict(R=dict(per=kreely_gun.GRIP_POSES), L='fist'),
    'gun2': dict(R=dict(per=kreely_gun.GRIP_POSES), L=dict(per=kreely_gun.GRIP_POSES)),
}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def jack():
    sys.path.insert(0, str(EXPLORER / 'builders'))
    from native import mdb
    catalog = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    item = next(m for m in catalog['models'] if m['id'] == JACK_ID)
    bank = next(b for b in catalog['banks'] if b['id'] == item['bank'])
    with open(bank['path'], 'rb') as f:
        f.seek(item['offset']); data = f.read(item['size'])
    assert hashlib.sha256(data).hexdigest() == item['sha256'], 'pl0030.mdb changed'
    return item, mdb(data)


def slice_segments(P, T, y0):
    tri = P[T]; d = tri[:, :, 1] - y0; pts = {}
    for a, b in ((0, 1), (1, 2), (2, 0)):
        m = (d[:, a] * d[:, b]) < 0
        t = d[m, a] / (d[m, a] - d[m, b])
        for k, q in zip(np.nonzero(m)[0], tri[m, a] + (tri[m, b] - tri[m, a]) * t[:, None]):
            pts.setdefault(k, []).append(q)
    return [v for v in pts.values() if len(v) == 2]


def crotch(P, T, top):
    """Lowest height above the knees where a horizontal slice of the surface crosses x=0."""
    for y in np.linspace(0.25 * top, 0.7 * top, 400):
        if any(min(a[0], b[0]) <= 0 <= max(a[0], b[0]) for a, b in slice_segments(P, T, y)):
            return float(y)
    raise RuntimeError('no crotch')


def landmarks(P, T, side=1, wrist=None):
    """Measured on one arm side (+x for Jack's human hand; Kreely is symmetric). All values in Y-up units."""
    top = float(P[:, 1].max()); x = P[:, 0] * side
    tip = float(x.max())
    arm = P[(x > 0.55 * tip) & (x < 0.75 * tip)]
    arm_y, arm_z = float(np.median(arm[:, 1])), float(np.median(arm[:, 2]))
    arm_lo = float(np.percentile(arm[:, 1], 3))
    band = P[(P[:, 1] > arm_lo - 0.06 * top) & (P[:, 1] < arm_lo - 0.02 * top)]
    pit = float((band[:, 0] * side).max())
    if wrist is None:  # going outward, the first plane slice whose height extent drops well under the sleeve's
        xs = np.linspace(0.6 * tip, 0.98 * tip, 80)
        ext = np.array([np.ptp(q[:, 1]) if len(q) else 0 for q in (kfing.plane_points(P, T, 0, side * x0) for x0 in xs)])
        sleeve = float(np.median(ext[(xs > 0.70 * tip) & (xs < 0.78 * tip)]))
        wrist = float(xs[next(i for i in range(len(xs)) if xs[i] > 0.78 * tip and ext[i] < 0.62 * sleeve)])
    return dict(top=top, tip=tip, armY=arm_y, armZ=arm_z, armUnderside=arm_lo, armpitX=pit, wristX=float(wrist),
                crotchY=crotch(P, T, top))


def pw(v, xs, ys):
    xs, ys = np.asarray(xs, float), np.asarray(ys, float)
    out = np.interp(v, xs, ys); lo, hi = v < xs[0], v > xs[-1]
    out[lo] = ys[0] + (v[lo] - xs[0]) * (ys[1] - ys[0]) / (xs[1] - xs[0])
    out[hi] = ys[-1] + (v[hi] - xs[-1]) * (ys[-1] - ys[-2]) / (xs[-1] - xs[-2])
    return out


def smooth(a, b, v):
    t = np.clip((v - a) / (b - a), 0, 1); return t * t * (3 - 2 * t)


def warp(P, A, B):
    """Landmark warp from proportions A to proportions B (y: sole/crotch/sleeve underside/top; arms: armpit/wrist/tip
    along |x|, then arm-axis y/z offset). Used here only to carry Jack's joints into Kreely's proportions (A=Jack)."""
    Q = P.copy(); ys = lambda L: [0, L['crotchY'], L['armUnderside'], L['top']]
    Q[:, 1] = pw(P[:, 1], ys(A), ys(B))
    ax = np.abs(P[:, 0]); sgn = np.sign(P[:, 0])
    upper = smooth(A['crotchY'] + 0.1 * A['top'], A['armUnderside'] - 0.05 * A['top'], P[:, 1])
    xw = pw(ax, [0, A['armpitX'], A['wristX'], A['tip']], [0, B['armpitX'], B['wristX'], B['tip']])
    Q[:, 0] = sgn * (ax + upper * (xw - ax))
    armness = upper * smooth(A['armpitX'], A['armpitX'] + 0.06 * A['top'], ax)
    yA = pw(np.array([A['armY']]), ys(A), ys(B))[0]
    Q[:, 1] += armness * (B['armY'] - yA)
    Q[:, 2] += armness * (B['armZ'] - A['armZ'])
    return Q


def axis_angle(axis, deg):
    axis = np.asarray(axis, float); axis = axis / np.linalg.norm(axis); h = np.radians(deg) / 2
    return np.r_[axis * np.sin(h), np.cos(h)]


def qmul(a, b):
    ax, ay, az, aw = a; bx, by, bz, bw = b
    return np.array([aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
                     aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz])


def classify(P, lum, K, face):
    """Color regions for points P (Kreely space, chest-centered z), with texture luminance lum. [inferred]"""
    top = K['top']; y = P[:, 1] / top; ax = np.abs(P[:, 0]) / top; z = P[:, 2] / top
    r = np.zeros(len(P), np.int8)                                               # suit
    # head vs. collar: the jaw/beard forms a V from (0, 0.835) to (+-0.045, 0.875); outside it, below 0.875, is the
    # standing shirt collar (white in the source) or jacket collar. [inferred from ortho renders of the source texture]
    jaw_v = (y > 0.835) & (ax < (y - 0.835) * 1.125) & (z > -0.01)
    head = (y > 0.875) | jaw_v | ((y > face['neckY']) & (z < face['backZ']))
    r[head] = REGIONS.index('skin')
    hair = head & ((y > face['hairlineY']) | (z < face['backZ']))
    beard = head & (y < face['noseY'] - 0.004) & (y > face['chinY'] - 0.012) & (z > face['frontZ']) & (ax < 0.034) \
        & (lum < 0.6)                                       # only the inked beard; white skin under/around it stays skin
    r[hair | beard] = REGIONS.index('hair')
    r[np.abs(P[:, 0]) > K['wristX']] = REGIONS.index('skin')
    front = z > 0.03
    vee = (z > 0.0) & (y > 0.58) & ~head & (y < 0.875) & (ax < 0.045 + (y - 0.62) * 0.2)   # white shirt between the lapels   # front half: the shirt curves back at its sides
    r[vee & (lum > 0.55)] = REGIONS.index('shirt')              # the dark placket/tie keeps the suit tint (no red)
    collar = (y > 0.80) & ~head & (y < 0.875) & (ax < 0.065) & (lum > 0.55)   # standing white collar under the beard
    r[collar] = REGIONS.index('shirt')
    # open shirt: bare chest V from the apex (0, 0.745), edges measured at +-0.011 (0.76) / +-0.032 (0.80) / +-0.04
    # (0.835), then the neck between the collar edges (+-0.04 -> +-0.046 at 0.875). Inked strokes inside it are chest hair. [inferred from ortho renders]
    half = np.where(y < 0.835, 0.04 * (np.clip(y - 0.745, 0, None) / 0.09) ** 0.65, 0.04 + (y - 0.835) * 0.15)
    opening = (z > 0.0) & (y > 0.745) & (y < 0.875) & (ax < half) & ~head
    r[opening] = REGIONS.index('skin')
    r[opening & (lum < 0.45) & (y < 0.835)] = REGIONS.index('hair')
    belt = (y > 0.545) & (y < 0.585) & (ax < 0.06) & (z > 0.0)              # only the gap between the jacket fronts
    r[belt] = REGIONS.index('belt')
    r[belt & front & (ax < 0.03)] = REGIONS.index('buckle')
    r[y < 0.045] = REGIONS.index('shoes')
    return r


def face_landmarks(P, K):
    """Face heights (fractions of height) from the front profile of the head. [inferred]"""
    top = K['top']; y = P[:, 1] / top; z = P[:, 2] / top
    head = P[(y > 0.84) & (np.abs(P[:, 0]) / top < 0.06)]
    hy = head[:, 1] / top; hz = head[:, 2] / top
    bins = np.linspace(0.85, 1.0, 61); prof = np.array([hz[(hy >= a) & (hy < b)].max() if ((hy >= a) & (hy < b)).any() else -1
                                                          for a, b in zip(bins[:-1], bins[1:])])
    mid = (bins[:-1] + bins[1:]) / 2
    nose = mid[np.argmax(np.where((mid > 0.89) & (mid < 0.95), prof, -1))]
    # chin: front-most point below the mouth region; neck: the profile minimum under it
    chin = mid[np.argmax(np.where((mid > 0.85) & (mid < nose - 0.03), prof, -1))]
    neck = mid[np.argmin(np.where((mid > 0.84) & (mid < chin), prof, 9))]
    return dict(noseY=float(nose), chinY=float(chin), neckY=float(max(neck, 0.858)), mouthY=float(nose - 0.4 * (nose - chin)),
                hairlineY=float(nose + 0.06), frontZ=float(np.percentile(hz, 60)), backZ=float(np.percentile(hz, 25)))


def texel_positions(S, src, uv, Ts, size):
    """Rasterize every triangle in UV space; per texel, the interpolated 3-D position (NaN where no triangle)."""
    h = w = size; pos = np.full((h, w, 3), np.nan, np.float32)
    UV = uv * [w, h]; Pv = S[src]
    for t in Ts:
        a, b, c = UV[t]
        x0, y0 = np.floor(np.minimum(np.minimum(a, b), c)).astype(int); x1, y1 = np.ceil(np.maximum(np.maximum(a, b), c)).astype(int)
        x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, w - 1), min(y1, h - 1)
        if x1 < x0 or y1 < y0: continue
        gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
        v0, v1 = b - a, c - a; d = v0[0] * v1[1] - v1[0] * v0[1]
        if abs(d) < 1e-12: continue
        px, py = gx - a[0], gy - a[1]
        l1 = (px * v1[1] - v1[0] * py) / d; l2 = (v0[0] * py - px * v0[1]) / d; l0 = 1 - l1 - l2
        m = (l0 >= -1e-3) & (l1 >= -1e-3) & (l2 >= -1e-3)
        if not m.any(): continue
        yy, xx = gy[m].astype(int), gx[m].astype(int)
        pos[yy, xx] = l0[m, None] * Pv[t[0]] + l1[m, None] * Pv[t[1]] + l2[m, None] * Pv[t[2]]
    return pos


def eye_shade(x, y, z):
    """Painted-eye value at sample points (fractions of height). Returns (gray value, inside eye, inside iris)."""
    e = EYES
    ok = ~np.isnan(x) & (z > e['min_z']) & (np.abs(y - e['cy']) < 0.01)
    ax = np.where(ok, np.abs(x), 9)
    u = (ax - e['cx']) / e['half_w']
    yc = e['cy'] + e['tilt'] * np.clip(u, -1, 1)                    # outer corners slightly higher (scowl)
    edge = e['half_h'] * np.clip(1 - u ** 2, 0, 1) ** 0.75
    v = np.where(edge > 0, (y - yc) / np.maximum(edge, 1e-9), 9)
    eye = ok & (np.abs(u) < 1) & (np.abs(v) <= 1)
    r = np.hypot(ax - e['cx'], y - e['cy'])
    iris = eye & (r < e['iris_r'])
    val = np.full(x.shape, 238.0)
    val[iris] = 205; val[iris & (r > 0.78 * e['iris_r'])] = 70; val[eye & (r < e['pupil_r'])] = 12
    val[eye & (v > 1 - e['lid'])] = 15
    val[eye & (np.hypot(ax - (e['cx'] - 0.0010), y - (e['cy'] + 0.0009)) < 0.0006)] = 255
    return val, eye, iris


def paint_eyes(rgb_img, pos, K, ss=4):
    """Paint almond eyes (sclera, iris, pupil, upper-lid ink line, catch-light) into the texels whose 3-D position
    falls in each eye opening, supersampled ss x ss per texel (anti-aliased edges). Seam texels next to the eyes
    take the nearest covered texel's position first, so no unpainted specks show at UV island borders.
    Returns the painted image and per-texel masks (sclera, iris) for the color bake."""
    top = K['top']; out = np.asarray(rgb_img, np.float32).copy(); h, w = pos.shape[:2]
    cov = ~np.isnan(pos[..., 0])
    near = ndimage.distance_transform_edt(~cov, return_distances=False, return_indices=True)
    filled = pos[near[0], near[1]]                                         # every texel gets its nearest surface point
    y = filled[..., 1] / top; z = filled[..., 2] / top
    cand = (np.abs(y - EYES['cy']) < 0.012) & (z > EYES['min_z']) & (np.abs(np.abs(filled[..., 0] / top) - EYES['cx']) < 0.02)
    cand &= ndimage.binary_dilation(cov, iterations=3)                    # only real surface + a 3-texel seam margin
    sclera = np.zeros((h, w), bool); iris = np.zeros((h, w), bool)
    lab, _ = ndimage.label(ndimage.binary_dilation(cand, iterations=2))
    offs = (np.arange(ss) + 0.5) / ss - 0.5
    for sl in ndimage.find_objects(lab):
        sub = filled[sl] / top                                             # (H, W, 3)
        H, W = sub.shape[:2]
        # bilinear sub-texel positions from neighbouring texel centres
        zoom = [ndimage.zoom(sub[..., k], ss, order=1, mode='nearest', grid_mode=True) for k in range(3)]
        val, eye, ir = eye_shade(*zoom)
        cov_e = eye.reshape(H, ss, W, ss).mean((1, 3))
        cov_i = ir.reshape(H, ss, W, ss).mean((1, 3))
        v = np.where(eye, val, 0).reshape(H, ss, W, ss).sum((1, 3)) / np.maximum(eye.reshape(H, ss, W, ss).sum((1, 3)), 1)
        blk = out[sl]; a = cov_e[..., None]
        out[sl] = blk * (1 - a) + v[..., None] * a
        sclera[sl] |= (cov_e >= 0.5) & (cov_i < 0.5); iris[sl] |= cov_i >= 0.5
    return Image.fromarray(np.clip(out + 0.5, 0, 255).astype(np.uint8)), sclera, iris


def bake_color(pos, lum_img, rgb_img, K, face, eye_masks):
    """Per-texel regions from the texel position map (gutters take the nearest covered texel), times the palette."""
    h, w = pos.shape[:2]; reg = np.full((h, w), -1, np.int8)
    lum = np.asarray(lum_img.resize((w, h)), np.float32) / 255
    cov = ~np.isnan(pos[..., 0])
    reg[cov] = classify(pos[cov].astype(np.float64), lum[cov], K, face)
    sclera, iris = eye_masks
    reg[sclera] = REGIONS.index('sclera'); reg[iris] = REGIONS.index('iris')
    hole = reg < 0                                                         # gutters: nearest covered texel
    idx = ndimage.distance_transform_edt(hole, return_distances=False, return_indices=True)
    reg = reg[idx[0], idx[1]]
    tint = np.array([[int(PALETTE[n][i:i + 2], 16) / 255 for i in (1, 3, 5)] for n in REGIONS], np.float32)
    dec = lambda c: np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    enc = lambda c: np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(np.maximum(c, 0), 1 / 2.4) - 0.055)
    src_rgb = np.asarray(rgb_img.resize((w, h)), np.float32) / 255
    out = enc(dec(src_rgb) * dec(tint)[reg])                               # same math as map x vertex color in linear
    return Image.fromarray(np.clip(out * 255 + 0.5, 0, 255).astype(np.uint8)), reg


def synth_ab(lum_img):
    """Shadow-ink texture [inferred]: Jack's pl0030_AB keeps most whites and darkens midtones (L 96-224 -> ~0.8 L).
    Approximation: L^1.5 blended with a 3px min filter; intensity also stored in alpha like GX IA textures."""
    L = np.asarray(lum_img, np.float32) / 255
    E = np.asarray(lum_img.filter(ImageFilter.MinFilter(3)), np.float32) / 255
    A = np.clip(0.5 * L ** 1.5 + 0.5 * E ** 1.5, 0, 1)
    a8 = (A * 255 + 0.5).astype(np.uint8)
    return Image.fromarray(np.dstack([a8, a8, a8, a8]), 'RGBA')


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('--blender', default=BLENDER)
    ap.add_argument('--deltoid', type=float, default=0.8, help='share of clavicle+upper-arm weight moved to 6/11 at the shoulder')
    ap.add_argument('--clavicle-chest', type=float, default=0.0, help='share of inner clavicle weight moved to the chest (2)')
    ap.add_argument('--jack-clavicle-height', action='store_true', help='keep clavicle pivots at warped Jack height instead of the arm axis offset')
    ap.add_argument('--variant', help='write to build/variants/<name> only (no explorer hook)')
    a = ap.parse_args()
    global OUT
    if a.variant: OUT = OUT / 'variants' / a.variant
    OUT.mkdir(parents=True, exist_ok=True); tmp = PORT / 'assets/characters/kreely/build/work'; tmp.mkdir(exist_ok=True)
    fbx, jpg = SRC / 'kreely.fbx', SRC / 'kreely.fbm/kreely_basecolor.jpg'
    jitem, J0 = jack()
    subprocess.run([a.blender, '-b', '--factory-startup', '--python', str(HERE / 'dump_fbx_mesh.py'), '--',
                    str(fbx), str(tmp / 'raw.npz')], check=True, capture_output=True)
    raw = dict(np.load(tmp / 'raw.npz'))
    S0, src, uv, Ts = raw['source_positions'], raw['source_vertex'], raw['uvs'], raw['indices']
    Tw = src[Ts]

    # Jack reference geometry and joints (bind pose, Y-up, facing +Z)
    keep = [m for m in J0['meshes'] if not m['name'].startswith('danmen')]
    JP = np.concatenate([np.array(m['positions']) for m in keep]); JT, off = [], 0
    for m in keep:
        JT.append(np.array(m['indices']).reshape(-1, 3) + off); off += len(m['positions'])
    J = landmarks(JP, np.concatenate(JT), side=1, wrist=J0['bones'][14]['position'][0])  # Jack's fist: wrist joint

    # 1) uniform scale to Jack's height, feet on the floor, centered on x, chest depth centered on Jack's
    scale = J['top'] / S0[:, 1].max()
    S = S0 * scale; S[:, 1] -= S[:, 1].min(); S[:, 0] -= (S[:, 0].max() + S[:, 0].min()) / 2
    band = lambda P: P[(P[:, 1] > 0.6 * J['top']) & (P[:, 1] < 0.75 * J['top']) & (np.abs(P[:, 0]) < 0.15)]
    dz = float(np.ptp(band(JP)[:, 2]) / 2 + band(JP)[:, 2].min() - np.ptp(band(S)[:, 2]) / 2 - band(S)[:, 2].min())
    S[:, 2] += dz
    K = landmarks(S, Tw, side=1)
    Sc = S - [0, 0, dz]                                                     # chest-centered copy for region rules
    face = face_landmarks(Sc, K)

    # 2) skeleton: Jack's joints carried into Kreely's proportions; ids/parents/bind rotations unchanged
    jbones = J0['bones']; jpos = np.array([b['position'] for b in jbones])
    kpos = warp(jpos, J, K)
    kpos[[i for i, b in enumerate(jbones) if b['id'] in (0, 1, 2, 3, 4, 15, 24, 25, 26, 52)], 0] = 0  # keep the midline
    # arm chains (clavicle..hand, deltoid helpers, weapon attach): keep Jack's offset from his arm axis, so the bind
    # arm is as horizontal as Jack's and rotation-only playback has no constant arm-angle error
    for i, b in enumerate(jbones):
        if b['id'] in (6, 7, 8, 9, 11, 12, 13, 14, 100, 101) or (not a.jack_clavicle_height and b['id'] in (5, 10)):
            kpos[i, 1] = K['armY'] + (jpos[i, 1] - J['armY']); kpos[i, 2] = K['armZ'] + (jpos[i, 2] - J['armZ'])
    bones = []
    for i, b in enumerate(jbones):
        p = b['parent']; rel = kpos[i] - (kpos[p] if p >= 0 else 0)
        bones.append(dict(id=b['id'], parent=p, position=np.round(kpos[i], 6).tolist(), relative=np.round(rel, 6).tolist()))
    idx = {b['id']: i for i, b in enumerate(bones)}; pos = {b['id']: kpos[i] for i, b in enumerate(jbones)}

    # 3) fingers: chains detected on the mesh, parented to the hand bones
    fingers, names, chains = {}, {}, {}
    for side_name, (sgn, hand_id) in SIDES.items():
        det = kfing.detect(S, Tw, sgn, K['wristX']); fingers[side_name] = det
        for f, fname in enumerate(FINGERS):
            jts = det[fname]; parent = idx[hand_id]; ids = []
            for s in range(3):
                bid = FINGER_ID0 + (0 if side_name == 'L' else 0x20) + f * 4 + s
                ppos = kpos[parent] if s == 0 else jts[s - 1]
                bones.append(dict(id=bid, parent=parent, position=np.round(jts[s], 6).tolist(),
                                  relative=np.round(jts[s] - (ppos if s else np.array(bones[parent]['position'])), 6).tolist()))
                parent = len(bones) - 1; idx[bid] = parent; ids.append(bid)
                names[bid] = f'{side_name}_{fname}_{s + 1}'
            chains[(side_name, fname)] = (ids, jts)

    # 4) hand poses (local quaternions, xyzw; bind frames are identity so axes are bind-space vectors)
    poses = {}
    for pname, spec0 in HAND_POSES.items():
        q = {}
        for side_name in SIDES:
            spec = spec0.get(side_name, spec0) if isinstance(spec0, dict) else spec0
            spec = HAND_POSES[spec] if isinstance(spec, str) else spec
            if 'per' in spec:                                       # per-finger tuples -> same layout as the presets
                per = spec['per']; spec = dict(fingers=None, per=per, pinky_extra=0, thumb=per['thumb'])
            det = fingers[side_name]; down = det['palmDown']
            fdir = np.mean([det[f][3] - det[f][0] for f in FINGERS[1:]], 0); fdir /= np.linalg.norm(fdir)
            across = det['pinky'][0] - det['index'][0]; across /= np.linalg.norm(across)
            for fname in FINGERS[1:]:
                ids, jts = chains[(side_name, fname)]; d = jts[3] - jts[0]
                ax = np.cross(d, down)
                for s, deg in enumerate(spec['per'][fname] if spec.get('per') else spec['fingers']):
                    deg = deg + (spec['pinky_extra'] if fname == 'pinky' else 0)
                    q[str(ids[s])] = np.round(axis_angle(ax, deg), 6).tolist()
            ids, jts = chains[(side_name, 'thumb')]; d = jts[3] - jts[0]; d /= np.linalg.norm(d)
            swing, cross, bend1, bend2 = spec['thumb']
            q0 = qmul(axis_angle(np.cross(d, fdir), cross), axis_angle(np.cross(d, down), swing))
            q[str(ids[0])] = np.round(q0, 6).tolist()
            q[str(ids[1])] = np.round(axis_angle(np.cross(d, across), bend1), 6).tolist()
            q[str(ids[2])] = np.round(axis_angle(np.cross(d, across), bend2), 6).tolist()
        poses[pname] = q

    # 5) bone heat on the unwarped mesh with Kreely's joints
    mid_mcp = {s: fingers[s]['middle'][0] for s in SIDES}
    tail = {1: pos[2], 2: pos[3], 3: pos[4], 4: pos[4] + [0, 0.18, 0.02], 5: pos[7], 10: pos[12],
            7: pos[8], 8: pos[9], 9: mid_mcp['R'], 12: pos[13], 13: pos[14], 14: mid_mcp['L'],
            15: pos[15] - [0, 0.1, 0], 16: pos[17], 17: pos[18], 18: pos[19], 19: pos[19] + [0, 0, 0.14],
            20: pos[21], 21: pos[22], 22: pos[23], 23: pos[23] + [0, 0, 0.14]}
    heat_ids = list(DEFORM); heads = [pos[i] for i in DEFORM]; tails = [tail[i] for i in DEFORM]
    for (side_name, fname), (ids, jts) in chains.items():
        for s in range(3):
            heat_ids.append(ids[s]); heads.append(jts[s]); tails.append(jts[s + 1])
    heads, tails = np.array(heads), np.array(tails)
    np.savez(tmp / 'heat-in.npz', positions=S, triangles=Tw, bone_ids=np.array(heat_ids), heads=heads, tails=tails)
    run = subprocess.run([a.blender, '-b', '--factory-startup', '--python', str(HERE / 'blender_weights.py'), '--',
                          str(tmp / 'heat-in.npz'), str(tmp / 'heat-out.npz')], capture_output=True, text=True)
    if run.returncode: raise RuntimeError(run.stdout[-3000:] + run.stderr[-3000:])
    heat_log = [l for l in run.stdout.splitlines() if 'weighted' in l or 'failed' in l.lower()]
    with np.load(tmp / 'heat-out.npz') as z: Wh = z['weights'].copy()
    cols = heat_ids + [6, 11, 24]                                           # post-heat helper columns
    W = np.c_[Wh, np.zeros((len(S), 3), np.float32)]; col = {b: k for k, b in enumerate(cols)}

    # 6a) islands bone heat could not solve: copy the nearest solved vertex (buttons, buckle, small shells)
    empty = W.sum(1) <= 1e-6
    if empty.any():
        tree = cKDTree(S[~empty]); _, nn = tree.query(S[empty]); W[empty] = W[~empty][nn]
    # 6b) shoulders, Jack's pattern: deltoid helper 6/11 shares ~half of clavicle+upper-arm weight around the shoulder
    #     joint; clavicles share ~40% with the chest (bone 2) on the inner side
    for clav, helper, upper in ((5, 6, 7), (10, 11, 12)):
        p = pos[upper]; r = np.linalg.norm(S - p, axis=1)
        zone = 1 - smooth(0.055, 0.15, r)
        m = W[:, col[clav]] + W[:, col[upper]]
        give = a.deltoid * zone * m
        for b in (clav, upper):
            frac = np.divide(W[:, col[b]], m, out=np.zeros_like(m), where=m > 0)
            W[:, col[b]] -= give * frac
        W[:, col[helper]] += give
        inner = smooth(0.0, 0.08, (np.abs(p[0]) - np.abs(S[:, 0])))     # inside the shoulder joint, toward the neck
        move = a.clavicle_chest * inner * W[:, col[clav]]
        W[:, col[clav]] -= move; W[:, col[2]] += move
    # 6c) jaw (bone 24): chin/beard below the mouth, front of the face; Jack's MOTs open it up to ~1 rad
    y = Sc[:, 1] / K['top']; z = Sc[:, 2] / K['top']; ax = np.abs(Sc[:, 0]) / K['top']
    jaw = smooth(face['mouthY'] + 0.002, face['mouthY'] - 0.008, y) * smooth(face['chinY'] - 0.014, face['chinY'] - 0.002, y) \
        * smooth(face['frontZ'] - 0.03, face['frontZ'], z) * (1 - smooth(0.035, 0.05, ax))
    jw = 0.65 * jaw                                                         # no mouth interior: cap the opening
    W *= (1 - jw)[:, None]; W[:, col[24]] += jw

    order = np.argsort(-W, 1)[:, :4]; top4 = np.take_along_axis(W, order, 1)
    top4 = np.where(top4 > 0.01, top4, 0); top4 /= top4.sum(1, keepdims=True)
    joints = np.array([idx[c] for c in cols])[order]; joints[top4 == 0] = joints[:, :1].repeat(4, 1)[top4 == 0]

    fn = np.cross(S[Tw[:, 1]] - S[Tw[:, 0]], S[Tw[:, 2]] - S[Tw[:, 0]]); vn = np.zeros_like(S)
    for k in range(3): np.add.at(vn, Tw[:, k], fn)
    vn /= np.linalg.norm(vn, axis=1, keepdims=True) + 1e-12

    # 7) textures: per-texel color regions and a synthesized shadow-ink (AB) texture
    pos_map = texel_positions(Sc, src, uv, Ts, 2048)
    rgb, sclera_m, iris_m = paint_eyes(Image.open(jpg).convert('RGB'), pos_map, K)
    rgb.save(OUT / 'kreely_basecolor.png', optimize=True); lumi = rgb.convert('L')       # source + painted eyes
    color_img, reg_tex = bake_color(pos_map, lumi, rgb, K, face, (sclera_m, iris_m))
    color_img.save(OUT / 'kreely_color.png', optimize=True)
    synth_ab(lumi).save(OUT / 'kreely_AB.png', optimize=True)
    eye_texels = dict(sclera=int(sclera_m.sum()), iris=int(iris_m.sum()))
    vlum = np.asarray(lumi, np.float32)[(np.clip(uv[:, 1], 0, 1) * 2047).astype(int), (np.clip(uv[:, 0], 0, 1) * 2047).astype(int)] / 255
    vreg = classify(Sc[src], vlum, K, face)

    r6 = lambda v: np.round(v, 6).tolist()
    mesh = dict(name='kreely_body_SB', material=0, texture=0, flags=0, renderFlags=8192, inkThreshold=140, colors=0,
                positions=r6(S[src]), normals=r6(vn[src]), uvs=r6(uv), indices=Ts.reshape(-1).tolist(),
                joints=joints[src].tolist(), weights=np.round(top4[src], 5).tolist(), regions=vreg.tolist())
    root_scale = float(kpos[idx[0]][1] / jpos[0][1])
    model = dict(bones=bones, meshes=[mesh], flags=J0.get('flags', 0), regionNames=REGIONS,
                 boneNames={str(k): v for k, v in names.items()},
                 handPoses=poses, defaultHandPose='fist',
                 retarget=dict(skeleton=JACK_ID, rootScale=round(root_scale, 6), rotationOnly=True,
                               note='pl0030 body joints are rotation-only; non-root position tracks keep Kreely bind offsets'),
                 ignoreMotionBones=[1024, 1025],
                 provenance=dict(mesh='user-provided kreely.fbx (not disc data)',
                                 skeleton=JACK_ID + ' ids/parents/bind rotations; joint positions fitted to Kreely; +30 finger bones',
                                 weights='Blender bone heat on Kreely joints, island copy, Jack-pattern deltoid/clavicle helpers, jaw falloff; top-4',
                                 regions='inferred', abTexture='synthesized (inferred)'))
    knuckles = {sd: np.mean([fingers[sd][f][0] for f in FINGERS[1:]], 0) for sd in SIDES}   # MCP line centre per hand
    gun_files, gun_report = kreely_gun.add_handgun(model, K, K['tip'] - K['wristX'], OUT, idx, knuckles)
    model_path = OUT / 'kreely.model.json'
    model_path.write_text(json.dumps(model, separators=(',', ':')), encoding='utf-8')

    # 8) offline posing checks (same math as the explorer): finger-pose sanity and weights summary
    Wb = kpose.bind_worlds(model); fist = kpose.worlds(model, None, 0, 'fist')
    tips = {}
    for (side_name, fname), (ids, jts) in chains.items():
        b = idx[ids[2]]; tip_bind = np.r_[jts[3], 1]
        tips[f'{side_name}_{fname}'] = np.round((fist[b] @ np.linalg.inv(Wb[b]) @ tip_bind)[:3] - jts[3], 4).tolist()
    used = sorted({cols[c] for c in np.unique(order[top4 > 0])})
    # bind bone directions (parent -> child joint) vs Jack's; rotation-only playback is exact where these agree
    bind_dir = {}
    for i, b in enumerate(jbones):
        p = b['parent']
        if p < 0: continue
        dj, dk = jpos[i] - jpos[p], kpos[i] - kpos[p]
        if np.linalg.norm(dj) < 1e-4 or np.linalg.norm(dk) < 1e-4: continue
        bind_dir[f"{jbones[p]['id']}->{b['id']}"] = round(float(np.degrees(np.arccos(np.clip(dj @ dk / np.linalg.norm(dj) / np.linalg.norm(dk), -1, 1)))), 2)
    report = dict(source=dict(fbx=str(fbx), fbxSha256=sha(fbx), texture=str(jpg), textureSha256=sha(jpg)),
                  skeleton=dict(id=JACK_ID, sha256=jitem['sha256'], jackBones=len(jbones), fingerBones=len(bones) - len(jbones)),
                  scale=scale, zOffset=dz, rootScale=root_scale, jackLandmarks=J, kreelyLandmarks=K, face=face,
                  jointShift={str(b['id']): np.round(kpos[i] - jpos[i], 4).tolist() for i, b in enumerate(jbones)},
                  fingers={s: {f: np.round(fingers[s][f], 4).tolist() for f in FINGERS} for s in SIDES},
                  fistTipOffset=tips, bindDirectionDegFromJack=bind_dir, handgun=gun_report, vertices=dict(welded=len(S0), split=len(src), triangles=len(Ts)),
                  weightedBones=used, heatFallbackVertices=int(empty.sum()), heatLog=heat_log,
                  regionCountsVertices={n: int((vreg == i).sum()) for i, n in enumerate(REGIONS)},
                  regionCountsTexels={n: int((reg_tex == i).sum()) for i, n in enumerate(REGIONS)}, eyeTexels=eye_texels, eyes=EYES,
                  output=dict(model=str(model_path), modelSha256=sha(model_path)))
    (OUT / 'kreely.build.json').write_text(json.dumps(report, indent=1), encoding='utf-8')

    if a.variant: return
    hook = EXPLORER / 'custom'; hook.mkdir(exist_ok=True)
    files = {k: OUT / v for k, v in dict(model='kreely.model.json', texture='kreely_basecolor.png',
                                          colorTexture='kreely_color.png', abTexture='kreely_AB.png').items()}
    custom = {k: str(v) for k, v in files.items()}; custom.update({k + 'Sha256': sha(v) for k, v in files.items()})
    custom.update(skeleton=JACK_ID, motionFamily='pl0030', palette='kreely', notes=str(PORT / 'research/KREELY.md'))
    # extra texture pages (index 1 = wp0671 pistol, 2 = Jude's gun page) + their AB pages, and the muzzle FX mask
    for k, (prim, ab) in ((k, v) for k, v in gun_files.items() if k != 'fx'):
        custom.setdefault('textures', {})[k] = str(prim); custom.setdefault('texturesSha256', {})[k] = sha(prim)
        custom.setdefault('abTextures', {})[k] = str(ab); custom.setdefault('abTexturesSha256', {})[k] = sha(ab)
    custom.update(fxTexture=str(gun_files['fx']), fxTextureSha256=sha(gun_files['fx']),
                  extraClips=list(model['overlays']), secondary='handgun (wp0671 pistol) with Jude the Dude clips')
    entry = dict(id='custom/kreely#kreely.mdb', bank='custom/kreely', name='kreely.mdb', bones=len(bones),
                 flags=model['flags'], texture='kreely_basecolor.png', identity='Kreese Kreely', review='custom',
                 displayName='Kreese Kreely · user model on Jack pl0030 rig', variant='User model on Jack pl0030 rig',
                 identityAuthority='user-provided model and name; not a native disc resource', custom=custom)
    path = hook / 'characters.json'
    chars = json.loads(path.read_text(encoding='utf-8')) if path.exists() else dict(version=1, characters=[])
    chars['characters'] = [c for c in chars['characters'] if c['id'] != entry['id']] + [entry]
    path.write_text(json.dumps(chars, indent=1), encoding='utf-8')
    print(json.dumps({k: report[k] for k in ('scale', 'rootScale', 'face', 'weightedBones', 'heatFallbackVertices', 'heatLog',
                                              'regionCountsTexels', 'fistTipOffset')}, indent=1))


if __name__ == '__main__':
    main()
