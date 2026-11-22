"""Numpy re-implementation of the explorer's sampleActor + linear blend skinning (app.js), for offline checks.

Bones: explorer model JSON bones (id, parent = array index or -1, relative). Bone -1 (origin) is appended last, as in
app.js. Euler order ZYX (three.js: R = Rz @ Ry @ Rx). Bind rotations are zero, so inverse bind = -bind translation.
"""
import numpy as np


def euler_zyx(x, y, z):
    cx, sx, cy, sy, cz, sz = np.cos(x), np.sin(x), np.cos(y), np.sin(y), np.cos(z), np.sin(z)
    Rx = np.array([[1, 0, 0], [0, cx, -sx], [0, sx, cx]])
    Ry = np.array([[cy, 0, sy], [0, 1, 0], [-sy, 0, cy]])
    Rz = np.array([[cz, -sz, 0], [sz, cz, 0], [0, 0, 1]])
    return Rz @ Ry @ Rx


def quat_matrix(q):
    x, y, z, w = q
    return np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                     [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                     [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])


MIRROR_PAIRS = [(5, 10), (6, 11), (7, 12), (8, 13), (9, 14), (16, 20), (17, 21), (18, 22), (19, 23), (40, 41), (50, 51), (100, 101)]
MIRROR = {**{a: b for a, b in MIRROR_PAIRS}, **{b: a for a, b in MIRROR_PAIRS}}


def mirror_track(bone, ch, values):
    """Left<->right mirror across x (bind frames are identity): swap paired bones; x positions and y/z Euler angles
    (ZYX) change sign, since S Rz(c) Ry(b) Rx(a) S = Rz(-c) Ry(-b) Rx(a)."""
    sign = -1 if ch in (0, 4, 5) else 1
    return MIRROR.get(bone, bone), ch, (values if sign == 1 else [-v for v in values])


def local_pose(model, clip=None, t=0, hand_pose=None, bind=False):
    """Local transforms (pos (n,3), R (n,3,3), scale (n,3)) exactly as app.js sampleActor sets them; n = bones + origin."""
    if clip is not None and clip.get('synth'): return synth_pose(model, clip, t)
    bones = model['bones']; n = len(bones) + 1
    pos = np.array([b['relative'] for b in bones] + [[0, 0, 0]], float)
    rot = np.zeros((n, 3)); scl = np.ones((n, 3)); quat = {}
    rt = model.get('retarget') or {}
    ovh = ((model.get('overlays') or {}).get(clip.get('id')) or {}) if clip is not None else {}
    if hand_pose is None: hand_pose = ovh.get('handPose') or model.get('defaultHandPose')
    if hand_pose and model.get('handPoses'):
        for bid, q in model['handPoses'][hand_pose].items():
            quat[next(i for i, b in enumerate(bones) if b['id'] == int(bid))] = np.array(q)
    if clip is not None:
        a = min(clip['frames'] - 1, int(np.floor(t))); b = min(clip['frames'] - 1, a + 1); f = t - a
        idx = {x['id']: i for i, x in enumerate(bones)}; idx[-1] = n - 1
        ovm = ((model.get('overlays') or {}).get(clip.get('id')) or {}).get('mirror')
        for tr in clip['tracks']:
            bone, ch, values = mirror_track(tr['bone'], tr['channel'], tr['values']) if ovm else (tr['bone'], tr['channel'], tr['values'])
            i = idx.get(bone)
            if i is None: continue
            v = values[a] * (1 - f) + values[b] * f
            if ch < 3:
                if rt.get('rotationOnly') and bone not in (-1, 0): continue
                rs = (rt.get('rootScaleByFamily') or {}).get(clip.get('family'), rt.get('rootScale', 1))
                ov0 = (model.get('overlays') or {}).get(clip.get('id')) or {}
                if bone == -1 and ov0.get('rootOrigin'): v = v - ov0['rootOrigin'][ch] * (-1 if ovm and ch == 0 else 1)   # event clips: world -> local
                pos[i, ch] = v * (rs if bone in (-1, 0) else 1)
            elif ch < 6: rot[i, ch - 3] = v; quat.pop(i, None)
            elif 7 <= ch <= 9: scl[i, ch - 7] = v
    idx0 = {x['id']: i for i, x in enumerate(bones)}
    for pb in ([] if bind else model.get('propBones') or []): scl[idx0[pb]] = 0   # props hidden unless an overlay shows them
    ov = (model.get('overlays') or {}).get(clip.get('id')) if clip is not None else None
    if ov:                                                                  # baked prop tracks (positions not retargeted)
        a = min(clip['frames'] - 1, int(np.floor(t))); b = min(clip['frames'] - 1, a + 1); f = t - a
        for tr in ov['tracks']:
            i = idx0[tr['bone']]; v = tr['values'][a] * (1 - f) + tr['values'][b] * f; ch = tr['channel']
            if ch < 3: pos[i, ch] = v
            elif ch < 6: rot[i, ch - 3] = v
            elif 7 <= ch <= 9: scl[i, ch - 7] = v
    if ov and ov.get('rootLift'):                                           # floor contact: number or per-frame list
        L = ov['rootLift']
        if isinstance(L, list): a2 = min(len(L) - 1, int(np.floor(t))); b2 = min(len(L) - 1, a2 + 1); L = L[a2] * (1 - (t - a2)) + L[b2] * (t - a2)
        pos[n - 1, 1] += L
    if ov and ov.get('rootYaw'): rot[n - 1, 1] += ov['rootYaw']             # face +Z at the clip start
    R = np.array([quat_matrix(quat[i]) if i in quat else euler_zyx(*rot[i]) for i in range(n)])
    return pos, R, scl


def fk(model, pos, R, scl):
    bones = model['bones']; n = len(bones) + 1
    W = np.zeros((n, 4, 4)); done = np.zeros(n, bool)
    parent = [b['parent'] for b in bones] + [None]

    def solve(i):
        if done[i]: return W[i]
        L = np.eye(4)
        L[:3, :3] = R[i] * scl[i][None, :]; L[:3, 3] = pos[i]
        p = parent[i]
        P = np.eye(4) if p is None else solve(n - 1) if p == -1 else solve(p) if p >= 0 else np.eye(4)
        W[i] = P @ L; done[i] = True
        return W[i]
    for i in range(n): solve(i)
    return W


def worlds(model, clip=None, t=0, hand_pose=None, bind=False):
    return fk(model, *local_pose(model, clip, t, hand_pose, bind))


def _slerp_R(A, B, w):
    from scipy.spatial.transform import Rotation, Slerp
    return Slerp([0, 1], Rotation.from_matrix(np.stack([A, B])))([w]).as_matrix()[0]


def synth_timeline(model, key):
    """Parts of a synthetic clip: segments play their range, blends crossfade (eased) from the previous segment's last
    frame to the next segment's first frame. Same layout as app.js loadSynth."""
    sc = model['synthClips'][key]; parts, T = [], 0
    for i, sg in enumerate(sc['segments']):
        a, b = sg['from'], sg['to']; parts.append(('seg', i, T, b - a + 1)); T += b - a + 1
        if i < len(sc['segments']) - 1: parts.append(('blend', i, T, sc['blends'][i])); T += sc['blends'][i]
    return parts, T


def synth_pose(model, clip, t):
    sc = clip['synth']; parts = clip['parts']
    k = next((p for p in parts if p[2] <= t < p[2] + p[3]), parts[-1])
    seg = lambda i, f: local_pose(model, sc['clips'][i], f)
    if k[0] == 'seg': return seg(k[1], sc['segments'][k[1]]['from'] + min(t - k[2], k[3] - 1))
    i = k[1]; w = (t - k[2] + 1) / (k[3] + 1); w = w * w * (3 - 2 * w)
    pa, Ra, sa = seg(i, sc['segments'][i]['to']); pb, Rb, sb = seg(i + 1, sc['segments'][i + 1]['from'])
    return pa * (1 - w) + pb * w, np.array([_slerp_R(x, y, w) for x, y in zip(Ra, Rb)]), sa * (1 - w) + sb * w


def make_synth(model, key, load_clip):
    """Build a synthetic clip dict; load_clip(overlay_key) -> prepared overlay clip."""
    sc = model['synthClips'][key]; parts, T = synth_timeline(model, key)
    clips = [load_clip(sg['key']) for sg in sc['segments']]
    return dict(id=key, frames=T, tracks=[], synth=dict(sc, clips=clips), parts=parts)


def prepare(clip, key, ov):
    """Overlay clip as the explorer plays it: trimmed to ov['range'], id = overlay key, family from the overlay."""
    if ov.get('range'):
        a, b = ov['range']; clip = dict(clip, frames=b - a + 1, tracks=[dict(t, values=t['values'][a:b + 1]) for t in clip['tracks']])
    return dict(clip, id=key, family=ov['family'])


def bind_worlds(model):
    return worlds(model, None, 0, hand_pose='open', bind=True)


def skin(model, mesh, W, B=None):
    if B is None: B = bind_worlds(model)
    M = W @ np.linalg.inv(B)                                          # (n,4,4)
    P = np.c_[np.array(mesh['positions']), np.ones(len(mesh['positions']))]
    J = np.array(mesh['joints']); Wt = np.array(mesh['weights'])
    out = np.zeros((len(P), 3))
    for k in range(4):
        out += Wt[:, k:k + 1] * np.einsum('nij,nj->ni', M[J[:, k]], P)[:, :3]
    return out
