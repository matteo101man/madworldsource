"""Kreely's handgun (explorer stage): native gun meshes, Jude the Dude's shooting clips and muzzle FX on Kreely's rig.

Sources (owned disc, read through the explorer's own readers):
  wp/wp0671.dat  wp0671.mdb/.tpl/_AB.tpl  semi-automatic pistol; the prop bound in ending events ev75/ev78
  em/em0400.dat  em0400.mdb (Jude the Dude; R_gun/L_gun revolvers on bones 40-42 / 43-45), em0400_*.mot gun clips,
                 em0400_*_00.seq shot cues (op 0x07), em0400.eff TEX sheet (muzzle blast streaks + ring)

Jude's skeleton shares pl0030's ids/hierarchy/bind directions for bones 0-24, so his clips drive Kreely with the same
rotation-only retarget (root scaled by hip height). His guns are not hand-skinned: they ride their own bones, animated
from the holsters into the hands. For Kreely we bake, per frame, the gun's transform relative to Kreely's hand:
  drawn (Jude's gun near his hand)      -> Jude's hand->gun transform (translation scaled by the hand-length ratio)
  holstered (near his thigh)             -> Jude's thigh->gun transform (scaled by the thigh-length ratio), at Kreely's hip
  in between                             -> blended by distance (draw / re-holster)
The result is stored as overlay tracks on prop bones 0x640 (right hand) / 0x641 (left hand); the explorer applies them
after the MOT tracks. Prop bones rest at scale 0 (hidden) in every clip without an overlay (e.g. Jack's pl0030 clips).
"""
import json, struct, sys
from pathlib import Path
import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
EXPLORER = HERE.parents[3] / 'exports' / 'madworld'
sys.path.insert(0, str(HERE)); sys.path.insert(0, str(EXPLORER / 'builders'))
import kreely_pose as kp
from native import mdb, motion, tpl

GUN_BONES = {'R': dict(id=0x640, hand=9, thigh=16, jude_gun=40), 'L': dict(id=0x641, hand=14, thigh=20, jude_gun=43)}
JUDE = 'em/em0400.dat#em0400.mdb'
PISTOL = 'wp/wp0671.dat#wp0671.mdb'
SHOT_OP = 0x07          # em0400 *_00.seq: shot cue (each is followed 1-3 frames later by a 0x1069:00xx gunshot sound in *_10.seq)
GRIP_POSES = {          # degrees: (base, mid, distal); fingers wrap a ~3 cm grip, trigger finger in the guard [inferred]
    'index': (22, 60, 30), 'middle': (72, 95, 55), 'ring': (75, 95, 55), 'pinky': (80, 95, 50), 'thumb': (35, 15, 10, 10)}
# canonical pistol-in-hand: barrel along the index finger (hand -x for the right hand), grip down across the knuckle
# line toward the pinky (hand -z), slide on the index/thumb side; grip centre under the proximal finger bones
R_HG = np.array([[0.0, 1, 0], [-1, 0, 0], [0, 0, 1]])                 # columns: gun x, y, z in the right-hand frame
S_X = np.diag([-1.0, 1, 1, 1])


def _cat():
    c = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    return c, {b['id']: b for b in c['banks']}


def _member(bank, entry_name):
    raw = Path(bank['path']).read_bytes(); e = next(x for x in bank['entries'] if x['name'] == entry_name)
    return raw[e['offset']:e['offset'] + e['size']]


def _model(c, banks, prefix):
    m = next(x for x in c['models'] if x['id'].startswith(prefix)); b = banks[m['bank']]
    raw = Path(b['path']).read_bytes()
    return m, b, mdb(raw[m['offset']:m['offset'] + m['size']])


def _seq(raw):
    n = struct.unpack_from('>H', raw, 6)[0]
    return [dict(op=raw[32 + 48 * i], frame=struct.unpack_from('>f', raw, 32 + 48 * i + 0x14)[0]) for i in range(n)]


def euler_zyx(R):
    """three.js Euler.setFromRotationMatrix(order 'ZYX')."""
    m31 = np.clip(R[2, 0], -1, 1); y = np.arcsin(-m31)
    if abs(m31) < 0.9999999: return np.array([np.arctan2(R[2, 1], R[2, 2]), y, np.arctan2(R[1, 0], R[0, 0])])
    return np.array([0.0, y, np.arctan2(-R[0, 1], R[1, 1])])


def _slerp_mat(A, B, t):
    """Blend two rigid transforms: positions linearly, rotations by quaternion slerp."""
    from scipy.spatial.transform import Rotation, Slerp
    r = Slerp([0, 1], Rotation.from_matrix([A[:3, :3], B[:3, :3]]))([t]).as_matrix()[0]
    out = np.eye(4); out[:3, :3] = r; out[:3, 3] = A[:3, 3] * (1 - t) + B[:3, 3] * t
    return out


def _grip_frame(P):
    """In the gun-bone frame (barrel along -y, grip toward -z): grip centre and muzzle point."""
    z0, z1 = P[:, 2].min(), P[:, 2].max()
    grip = P[P[:, 2] < z0 + 0.35 * (z1 - z0)].mean(0)
    y0 = P[:, 1].min(); tip = P[P[:, 1] < y0 + 0.012]
    return grip, np.array([tip[:, 0].mean(), y0, tip[:, 2].mean()])


def canonical_grip(model, idx, knuckles, g_rev):
    """4x4 gun-frame -> hand-frame transforms for the right hand and (mirrored) left hand."""
    out = {}
    hpos = np.array(model['bones'][idx[9]]['position']); kn = knuckles['R'] - hpos
    grip_h = np.array([kn[0] - 0.012, -0.032, kn[2]])                     # past the MCP line, palm side, knuckle centre
    G = np.eye(4); G[:3, :3] = R_HG; G[:3, 3] = grip_h - R_HG @ g_rev
    out['R'] = G; out['L'] = S_X @ G @ S_X
    return out


def _align(a, b):
    """Minimal rotation taking direction a to direction b."""
    a = a / np.linalg.norm(a); b = b / np.linalg.norm(b); v = np.cross(a, b); c = float(a @ b)
    if np.linalg.norm(v) < 1e-9: return np.eye(3) if c > 0 else -np.eye(3) + 2 * np.outer(np.cross(a, [0, 1, 0]), np.cross(a, [0, 1, 0])) / max(np.linalg.norm(np.cross(a, [0, 1, 0])) ** 2, 1e-9)
    vx = np.array([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])
    return np.eye(3) + vx + vx @ vx / (1 + c)


def two_bone_ik(W, idx, chain, target_pos, target_rot):
    """chain = (clavicle, upper, fore, hand) bone ids. Returns new local rotations {id: R} for upper/fore/hand that put
    the hand at target_pos with world rotation target_rot (elbow bends in the plane of its current pole)."""
    c, u, f, h = (idx[x] for x in chain)
    S, E0, H0 = W[u][:3, 3], W[f][:3, 3], W[h][:3, 3]
    a, b = np.linalg.norm(E0 - S), np.linalg.norm(H0 - E0)
    d = target_pos - S; dist = float(np.clip(np.linalg.norm(d), abs(a - b) + 1e-4, a + b - 1e-4)); un = d / np.linalg.norm(d)
    pole = (E0 - S) - ((E0 - S) @ un) * un
    if np.linalg.norm(pole) < 1e-5: pole = np.array([0, -1.0, 0]) - un[1] * un
    pole /= np.linalg.norm(pole)
    ca = (a * a + dist * dist - b * b) / (2 * a * dist); sa = np.sqrt(max(0.0, 1 - ca * ca))
    E = S + a * (ca * un + sa * pole); T = S + dist * un
    Ru = _align(E0 - S, E - S) @ W[u][:3, :3]
    fore_dir = _align(E0 - S, E - S) @ (H0 - E0)
    Rf = _align(fore_dir, T - E) @ _align(E0 - S, E - S) @ W[f][:3, :3]
    Rc = W[c][:3, :3]
    return {chain[1]: Rc.T @ Ru, chain[2]: Ru.T @ Rf, chain[3]: Rf.T @ target_rot}


def _rodrigues(axis, ang):
    axis = axis / np.linalg.norm(axis); K = np.array([[0, -axis[2], axis[1]], [axis[2], 0, -axis[0]], [-axis[1], axis[0], 0]])
    return np.eye(3) + np.sin(ang) * K + (1 - np.cos(ang)) * K @ K


def _blend_rot(A, B, w):
    from scipy.spatial.transform import Rotation, Slerp
    return Slerp([0, 1], Rotation.from_matrix([A, B]))([w]).as_matrix()[0]


def add_handgun(model, K, hand_len_k, out_dir, idx, knuckles=None):
    """Augment Kreely's model dict in place. Returns (extra texture files, report)."""
    c, banks = _cat()
    jm, jb, jude = _model(c, banks, JUDE)
    pm, pb, pistol = _model(c, banks, PISTOL)
    jids = [b['id'] for b in jude['bones']]; jpos = {b['id']: np.array(b['position']) for b in jude['bones']}

    # --- gun geometry in Jude's gun-bone frame (bind rotations are zero: frame axes = model axes) ---
    def mesh_by(name): return next(m for m in jude['meshes'] if m['name'] == name)
    rev = {s: mesh_by(f'{s}_gun') for s in 'RL'}
    rev_local = {s: np.array(rev[s]['positions']) - jpos[GUN_BONES[s]['jude_gun']] for s in 'RL'}
    # pistol (wp0671: barrel -x, grip -z) -> revolver frame (barrel -y, grip -z): (x, y, z) -> (-y, x, z)
    pm_mesh = pistol['meshes'][0]; Pp = np.array(pm_mesh['positions'])
    Pp = np.c_[-Pp[:, 1], Pp[:, 0], Pp[:, 2]]
    g_rev, mz_rev = _grip_frame(rev_local['R']); g_p, _ = _grip_frame(Pp)
    Pp = Pp - g_p + g_rev                                                 # grip onto Jude's grip position
    A_pistol = np.eye(4); A_pistol[:3, :3] = [[0, -1, 0], [1, 0, 0], [0, 0, 1]]; A_pistol[:3, 3] = g_rev - g_p   # wp0671 model -> gun frame
    _, mz_p = _grip_frame(Pp)
    jhand = np.array(mesh_by('R_hand_SB')['positions']); hand_len_j = float(np.abs(jhand[:, 0]).max() - abs(jpos[9][0]))
    s_hand = hand_len_k / hand_len_j
    thigh_k = np.linalg.norm(np.array(model['bones'][idx[17]]['position']) - np.array(model['bones'][idx[16]]['position']))
    s_leg = thigh_k / float(np.linalg.norm(jpos[17] - jpos[16]))
    # revolver scaled to Kreely's hand about its grip (Jude's hands are ~1.4x larger); pistol kept at native size
    rev_k = {s: (rev_local[s] - g_rev * [(-1 if s == 'L' else 1), 1, 1]) * s_hand + g_rev * [(-1 if s == 'L' else 1), 1, 1] for s in 'RL'}
    pistol_k = {'R': Pp, 'L': Pp * [-1, 1, 1]}                              # left = mirror across the gun's x
    muzzles = {}

    # --- prop bones (bind offset 0 from the hand; rest scale 0 = hidden) ---
    for s, g in GUN_BONES.items():
        hpos = np.array(model['bones'][idx[g['hand']]]['position'])
        model['bones'].append(dict(id=g['id'], parent=idx[g['hand']], position=np.round(hpos, 6).tolist(), relative=[0, 0, 0]))
        idx[g['id']] = len(model['bones']) - 1
        model['boneNames'][str(g['id'])] = f'{s}_handgun'
    model['propBones'] = [g['id'] for g in GUN_BONES.values()]

    # --- meshes ---
    def add_mesh(name, P_local, src_mesh, side, texture, hidden):
        g = GUN_BONES[side]; hpos = np.array(model['bones'][idx[g['hand']]]['position'])
        P = P_local + hpos; n = len(P)
        I = np.array(src_mesh['indices']).reshape(-1, 3)
        if side == 'L' and src_mesh is pm_mesh: I = I[:, ::-1]                 # mirrored: keep winding outward
        Pn = P; fn = np.cross(Pn[I[:, 1]] - Pn[I[:, 0]], Pn[I[:, 2]] - Pn[I[:, 0]]); vn = np.zeros_like(Pn)
        for k in range(3): np.add.at(vn, I[:, k], fn)
        vn /= np.linalg.norm(vn, axis=1, keepdims=True) + 1e-12
        model['meshes'].append(dict(name=name, material=0, texture=texture, flags=0, renderFlags=src_mesh['renderFlags'],
                                    inkThreshold=src_mesh['inkThreshold'], colors=0, positions=np.round(P, 6).tolist(),
                                    normals=np.round(vn, 6).tolist(), uvs=np.round(np.array(src_mesh['uvs']), 6).tolist(),
                                    indices=I.reshape(-1).tolist(), joints=[[idx[g['id']]] * 4] * n, weights=[[1, 0, 0, 0]] * n,
                                    regions=[model['regionNames'].index('gunmetal')] * n))
        if hidden: model.setdefault('hiddenMeshes', []).append(name)
    for s in 'RL':
        add_mesh(f'kreely_pistol_{s}_KB', pistol_k[s], pm_mesh, s, 1, hidden=(s == 'L'))
        add_mesh(f'kreely_revolver_{s}_SB', rev_k[s], rev[s], s, 2, hidden=True)
        muzzles[f'kreely_pistol_{s}_KB'] = (mz_p * ([-1, 1, 1] if s == 'L' else 1)).round(5).tolist()
        _, mz_r = _grip_frame(rev_k[s] * ([-1, 1, 1] if s == 'L' else 1)); mz_r = mz_r * ([-1, 1, 1] if s == 'L' else 1)
        muzzles[f'kreely_revolver_{s}_SB'] = mz_r.round(5).tolist()
    model['muzzles'] = dict(points=muzzles, barrel=[0, -1, 0])

    # --- textures (index 1 = pistol, 2 = Jude's gun atlas page) and the FX mask ---
    pt = tpl(_member(pb, 'wp0671.tpl')); pab = tpl(_member(pb, 'wp0671_AB.tpl'))
    jt = tpl(_member(jb, 'em0400.tpl')); jab = tpl(_member(jb, 'em0400_AB.tpl'))
    files = {}
    for key, (prim, ab, ti) in {'1': (pt, pab, pm_mesh['texture']), '2': (jt, jab, rev['R']['texture'])}.items():
        prim[ti][0].save(out_dir / f'kreely_tex{key}.png'); ab[ti][0].save(out_dir / f'kreely_tex{key}_AB.png')
        files[key] = (out_dir / f'kreely_tex{key}.png', out_dir / f'kreely_tex{key}_AB.png')
    eff = _member(jb, 'em0400.eff'); sheet = tpl(eff[0x60:])[0][0].convert('L')   # EFF 'TEX' section at 0x60
    L = np.asarray(sheet, np.uint8)
    Image.fromarray(np.dstack([np.full_like(L, 255)] * 3 + [255 - L]), 'RGBA').save(out_dir / 'kreely_fx.png')
    files['fx'] = out_dir / 'kreely_fx.png'

    # --- overlays: Jude's gun clips on Kreely (one-handed, right hand) ---
    jude_model = dict(bones=jude['bones'])
    root_k = np.array(model['bones'][idx[0]]['position'])[1]
    model['retarget'].setdefault('rootScaleByFamily', {})['em0400'] = round(float(root_k / jpos[0][1]), 6)
    model['ignoreMotionBones'] = sorted(set(model.get('ignoreMotionBones', [])) | {32, 33, 34, 35, 36, 37, 42, 43, 44, 45})
    raw = Path(jb['path']).read_bytes(); ents = {e['name']: e for e in jb['entries'] if e['size']}
    grip = canonical_grip(model, idx, knuckles, g_rev); support = support_grip(grip)
    kmodel = dict(model, overlays={})
    B = kp.bind_worlds(model)
    overlays, report = {}, {}
    g = GUN_BONES['R']
    for cl in [x for x in c['clips'] if x['bank'] == jb['id'] and x['name'].startswith('em0400_')]:
        mo = motion(raw[cl['offset']:cl['offset'] + cl['size']]); mo['family'] = 'em0400'
        moving = {t['bone'] for t in mo['tracks'] if t['bone'] in (40, 43) and np.ptp(t['values']) > 1e-3}
        if 40 not in moving: continue                                       # right gun never drawn: not a handgun clip
        stem = cl['name'][:-4]
        if stem in JUDE_DROP: report[stem] = dict(dropped=JUDE_DROP[stem]); continue
        two = stem in TWO_HAND_CLIPS
        shots_f = sorted({round(r['frame']) for r in _seq(raw[ents[stem + '_00.seq']['offset']:][:ents[stem + '_00.seq']['size']])
                          if r['op'] == SHOT_OP}) if stem + '_00.seq' in ents else []
        gvals = [[] for _ in range(7)]; arm = {b2: [[], [], []] for b2 in L_ARM}; near = []
        for t in range(mo['frames']):
            Wj = kp.worlds(jude_model, mo, t); Wk = kp.worlds(kmodel, mo, t)
            G = Wj[jids.index(g['jude_gun'])]
            Hj, Tj = Wj[jids.index(g['hand'])], Wj[jids.index(g['thigh'])]
            rel_t = np.linalg.inv(Tj) @ G; rel_t[:3, 3] *= s_leg
            Hk, Tk = Wk[idx[g['hand']]], Wk[idx[g['thigh']]]
            d_h = np.linalg.norm(G[:3, 3] - Hj[:3, 3]); d_t = np.linalg.norm(G[:3, 3] - Tj[:3, 3])
            w = float(np.clip((d_t - d_h) / max(d_t + d_h, 1e-6) * 2 + 0.5, 0, 1))   # 1 = in hand, 0 = at hip
            world = _slerp_mat(Tk @ rel_t, Hk @ grip['R'], w)               # in hand: canonical grip
            local = np.linalg.inv(Hk) @ world; e = euler_zyx(local[:3, :3])
            for k in range(3): gvals[k].append(local[k, 3]); gvals[3 + k].append(e[k])
            gvals[6].append(1.0); near.append(w > 0.5)
            # left arm: not dual wielding -> relaxed at his side, carried by the torso; 0051: support hand on the pistol
            tgt, rot = relaxed_left(Wk, idx)
            locs = two_bone_ik(Wk, idx, (10, 12, 13, 14), tgt, rot)
            if two and w > 1e-3:
                HL = world @ np.linalg.inv(support)
                sup = two_bone_ik(Wk, idx, (10, 12, 13, 14), HL[:3, 3], HL[:3, :3])
                locs = {b2: _blend_rot(locs[b2], sup[b2], w) for b2 in L_ARM}
            for b2 in L_ARM:
                e3 = euler_zyx(locs[b2])
                for k in range(3): arm[b2][k].append(e3[k])
        tracks = prop_tracks(g['id'], gvals) + arm_tracks(arm)
        shots = [dict(frame=f, gun='R') for f in shots_f if f < len(near) and near[f]]
        key = cl['id']
        ov = dict(clip=stem, family='em0400', tracks=tracks, shots=shots, handPose='gun2' if two else 'gun', style='gunslinger',
                  label=f"Gunslinger · {stem} · {len(shots)} shot{'s' if len(shots) != 1 else ''} · "
                        f"{'both hands on the pistol' if two else 'one-handed'} (Jude the Dude)")
        contact = jude_contact(jude, jude_model, mo)
        lift, lows = ground(model, B, key, ov, dict(mo, id=key), contact); ov['rootLift'] = np.round(lift, 4).tolist()
        ov['rootYaw'] = face_yaw(model, key, ov, dict(mo, id=key))
        overlays[key] = ov
        report[stem] = dict(frames=mo['frames'], twoHand=two, shotFrames=shots_f, shots=len(shots),
                            rootLiftRange=[round(float(lift.min()), 3), round(float(lift.max()), 3)],
                            lowestPointAfterLift=round(float((lows + lift).min()), 4), contactFrames=int(contact.sum()),
                            contactFloatMax=round(float((lows + lift)[contact].max()), 4))
    # Leonardo Fallmont's pistol cutscenes, trimmed into gameplay clips (same wp0671 pistol)
    leo_over, leo_report = _leonardo(model, idx, c, A_pistol, grip, support, Pp, B)
    overlays.update(leo_over); report.update(leo_report)
    model['overlays'] = overlays
    model['synthClips'] = synth_clips(overlays)
    model.setdefault('provenance', {})['handgun'] = ('wp0671 pistol (ending-event prop) + Jude the Dude (em0400) revolvers, '
                                                     'Jude gun clips/SEQ shot cues/em0400.eff sprite; per-frame gun transforms baked [inferred method]')
    return files, dict(sHand=s_hand, sLeg=s_leg, pistolGripAligned=g_rev.round(4).tolist(), muzzles=muzzles, clips=report)


JUDE_DROP = {'em0400_0102': '6-shot burst: dropped at user request', 'em0400_0032': '2 shots: dropped at user request',
             'em0400_0053': '2 shots: dropped at user request'}
TWO_HAND_CLIPS = {'em0400_0051'}      # kneeling crouch: both hands on the one pistol (user request)
L_ARM = (12, 13, 14)
RELAXED_L = np.array([[0.0, 1, 0], [-1, 0, 0], [0, 0, 1]])           # left hand hanging: fingers down, palm to the thigh


def support_grip(grip):
    """Left hand cupping the right-hand grip from below (gun frame -> left-hand frame)."""
    s = grip['L'].copy(); s[:3, 3] += s[:3, :3] @ np.array([0.0, 0.01, -0.035])
    return s


def relaxed_left(Wk, idx):
    """Left wrist target hanging beside the body in the chest's frame (moves with the torso) and its hand rotation."""
    R2 = Wk[idx[2]][:3, :3]; S = Wk[idx[12]][:3, 3]
    a = np.linalg.norm(Wk[idx[13]][:3, 3] - S); b = np.linalg.norm(Wk[idx[14]][:3, 3] - Wk[idx[13]][:3, 3])
    return S + R2 @ np.array([0.06, -(a + b) * 0.92, 0.07]), R2 @ RELAXED_L


def prop_tracks(bone, gvals):
    for k in range(3, 6): gvals[k] = np.unwrap(gvals[k]).tolist()
    out = []
    for ch, v in zip((0, 1, 2, 3, 4, 5, 7), gvals):
        out.append(dict(bone=bone, channel=ch, values=np.round(v, 5).tolist()))
        if ch == 7: out += [dict(bone=bone, channel=c2, values=np.round(v, 5).tolist()) for c2 in (8, 9)]
    return out


def arm_tracks(vals):
    return [dict(bone=b2, channel=3 + k, values=np.round(np.unwrap(vals[b2][k]), 5).tolist()) for b2 in vals for k in range(3)]


def ground(model, B, key, ov, clip, contact=None):
    """Root lift per frame from floor contact: on frames where the source character touches the floor (contact[t]),
    Kreely's lowest body point is put exactly on y=0; airborne frames interpolate the lift (jump arcs are kept).
    Fixes Leonardo sinking, Jude's skate-height float and proportion differences in falls / get-ups.
    Returns (lift per frame, lowest points before lift)."""
    from scipy.ndimage import gaussian_filter1d
    m = model['meshes'][0]; sel = slice(None, None, 3)
    sub = dict(positions=m['positions'][sel], joints=m['joints'][sel], weights=m['weights'][sel])
    temp = dict(model, overlays={key: dict(ov, rootLift=0)})
    lows = np.array([float(kp.skin(temp, sub, kp.worlds(temp, clip, t), B)[:, 1].min()) for t in range(clip['frames'])])
    contact = np.ones(len(lows), bool) if contact is None else np.asarray(contact, bool)
    if not contact.any(): contact[np.argmin(lows)] = True
    tt = np.arange(len(lows)); lift = np.interp(tt, tt[contact], -lows[contact])
    if len(lift) > 3: lift = gaussian_filter1d(lift, 1.0)
    lift = np.maximum(lift, -lows)                                          # never below the floor
    return lift, lows


def jude_contact(jude, jude_model, mo):
    """Frames where Jude's own skinned mesh (skates included) is within 3 cm of his lowest point in the clip."""
    P = np.concatenate([np.array(m['positions']) for m in jude['meshes']])[::4]
    J = np.concatenate([np.array(m['joints']) for m in jude['meshes']])[::4]; Wt = np.concatenate([np.array(m['weights']) for m in jude['meshes']])[::4]
    sub = dict(positions=P.tolist(), joints=J.tolist(), weights=Wt.tolist()); Bj = kp.bind_worlds(jude_model)
    lows = np.array([float(kp.skin(jude_model, sub, kp.worlds(jude_model, mo, t), Bj)[:, 1].min()) for t in range(mo['frames'])])
    return lows < max(lows.min(), 0.0) + 0.03


def face_yaw(model, key, ov, clip):
    """Root yaw that turns the hips to face +Z at the clip's first frame (all pistol clips share one facing, so
    blends between them don't spin him)."""
    W = kp.worlds(dict(model, overlays={key: dict(ov, rootYaw=0)}), clip, 0)
    f = W[[b['id'] for b in model['bones']].index(0)][:3, :3] @ np.array([0, 0, 1.0])
    return round(float(-np.arctan2(f[0], f[2])), 5)


def synth_clips(overlays):
    """Transition clips made by blending (eased full-pose crossfades, every bone incl. fingers and gun), plus a chained
    gameplay sequence. Segments play an overlay clip range; blends crossfade last frame -> next first frame."""
    by = {o['clip']: k for k, o in overlays.items()}
    idle, aim, fire = (by['ev93_cut18_idle'], by['ev93_cut8_aim'], by['ev93_cut8_fire'])
    last = lambda k: overlays[k]['range'][1] - overlays[k]['range'][0]
    full = lambda k: dict(key=k, **{'from': 0, 'to': last(k)})
    one = lambda k, f: dict(key=k, **{'from': f, 'to': f})
    return {
        'synth:raise': dict(label='Pistol · raise (idle -> aim), blended', segments=[one(idle, last(idle)), one(aim, 0)], blends=[20]),
        'synth:lower': dict(label='Pistol · lower (aim -> idle), blended', segments=[one(aim, last(aim)), one(idle, 0)], blends=[22]),
        'synth:aim-fire': dict(label='Pistol · aim -> fire, blended', segments=[one(aim, last(aim)), full(fire)], blends=[10]),
        'synth:sequence': dict(label='Pistol · sequence: idle, raise, aim, fire, aim, lower, idle',
                               segments=[full(idle), full(aim), full(fire), full(aim), full(idle)], blends=[20, 10, 10, 22]),
    }


def slice_clip(clip, a, b):
    return dict(clip, frames=b - a + 1, tracks=[dict(t, values=t['values'][a:b + 1]) for t in clip['tracks']])


def _pose_vec(clip, t):
    return np.array([tr['values'][t] for tr in clip['tracks'] if 0 <= tr['bone'] <= 23 and 3 <= tr['channel'] < 6]
                    + [tr['values'][t] * 3 for tr in clip['tracks'] if tr['bone'] == 0 and tr['channel'] == 1])


def best_loop(clip, w0, w1, l0, l1):
    """(a, b) inside [w0, w1] with l0 <= b-a <= l1 whose end pose best matches its start pose (seamless loop)."""
    V = np.array([_pose_vec(clip, t) for t in range(clip['frames'])]); best = None
    for a in range(w0, w1 - l0 + 1, 2):
        for b in range(a + l0, min(a + l1, w1) + 1):
            d = float(np.abs(V[a] - V[b]).sum()) - 0.002 * (b - a)
            if best is None or d < best[0]: best = (d, a, b)
    return best[1], best[2], round(best[0], 4)


LEO = 'em/em0940.dat#em0940.mdb'
LEO_CLIPS = [  # (event, cut, variant, label, how) - standing ending cuts only; standoff (ev78 c2) and turning (ev93 c19) dropped
    ('ev93', 8, 'aim', 'Pistol · aim hold (loop)', ('loop', 100, 235, 40, 90)),
    ('ev93', 8, 'fire', 'Pistol · aim & fire (1 shot)', ('shot', 14, 20, 60)),
    ('ev93', 18, 'idle', 'Pistol · idle, lowered (loop)', ('loop', 0, 85, 40, 85)),
]


def _leonardo(model, idx, c, A_pistol, grip, support, pistol_pts, B):
    """Gameplay clips cut from Leonardo's (em0940) event clips. Gun world = the wp0671 actor's own clip in the same cut,
    mapped into Kreely's gun frame. Left-handed cuts are mirrored so Kreely shoots right-handed. While Leonardo aims
    (barrel near level, gun held out in front above the hips) both arms are solved (two-bone IK): pistol at eye level,
    arms extended, rear sight ~3 cm under the eye line, barrel level along Leonardo's aim; right hand on the canonical
    grip, left hand cupping it (support grip). Shots = recoil spikes of the event gun (8-40 mm/frame, level, steady
    before) [inferred: event SEQs carry no shot cues]; each gets a short recoil kick. Ranges: loops are picked where the
    end pose best matches the start; the fire clip starts 14 frames before the shot."""
    ev = json.loads((EXPLORER / 'events.json').read_text(encoding='utf-8')); evc = {x['id']: x for x in ev['clips']}
    banks = {b['id']: b for b in c['banks']}
    lm = next(x for x in c['models'] if x['id'].startswith(LEO)); leo = mdb(Path(banks[lm['bank']]['path']).read_bytes()[lm['offset']:lm['offset'] + lm['size']])
    gm = next(x for x in c['models'] if x['id'].startswith(PISTOL)); gun = mdb(Path(banks[gm['bank']]['path']).read_bytes()[gm['offset']:gm['offset'] + gm['size']])
    lids = [b['id'] for b in leo['bones']]; lpos = {b['id']: np.array(b['position']) for b in leo['bones']}
    root_k = np.array(model['bones'][idx[0]]['position'])[1]
    model['retarget'].setdefault('rootScaleByFamily', {})['em0940'] = round(float(root_k / lpos[0][1]), 6)
    Bg = kp.worlds(dict(bones=gun['bones']), None, 0, bind=True)[0]
    load = lambda cid: motion(Path(evc[cid]['sourcePath']).read_bytes()[evc[cid]['offset']:evc[cid]['offset'] + evc[cid]['size']])
    sight = np.array([0.0, pistol_pts[:, 1].max(), pistol_pts[:, 2].max()])      # rear-top of the slide (gun frame)
    kmodel = dict(model, overlays={}); S = np.diag([-1.0, 1, 1, 1]); arm_ids = (7, 8, 9, 12, 13, 14)
    out, rep, cache = {}, {}, {}
    for eid, cut, variant, label, how in LEO_CLIPS:
        if (eid, cut) not in cache:
            e = next(x for x in ev['events'] if x['id'] == eid)
            la = next(a for a in e['actors'] if a['object'] == 'em0940'); ga = next(a for a in e['actors'] if a['object'] == 'wp0671')
            lc = next(x for x in la['clips'] if evc[x]['cut'] == cut); gc = next(x for x in ga['clips'] if evc[x]['cut'] == cut)
            native, gclip = load(lc), load(gc); n = min(native['frames'], gclip['frames'])
            def hand_d(h): return np.median([np.linalg.norm(kp.worlds(dict(bones=gun['bones']), gclip, t)[0][:3, 3]
                                                            - kp.worlds(dict(bones=leo['bones']), native, t)[lids.index(h)][:3, 3]) for t in range(0, n, 25)])
            mirror = bool(hand_d(14) < hand_d(9))
            lclip = dict(native, tracks=[dict(zip(('bone', 'channel', 'values'), kp.mirror_track(t['bone'], t['channel'], t['values'])))
                                         for t in native['tracks']]) if mirror else native
            props, pos, bar = [], [], []
            for t in range(n):
                Wl = kp.worlds(dict(bones=leo['bones']), lclip, t); Wg = kp.worlds(dict(bones=gun['bones']), gclip, t)
                prop = Wg[0] @ np.linalg.inv(Bg) @ np.linalg.inv(A_pistol)
                if mirror: prop = S @ prop @ S
                props.append((Wl, prop)); pos.append(prop[:3, 3]); bar.append(prop[:3, :3] @ [0, -1, 0])
            pos, bar = np.array(pos), np.array(bar)
            back = -(np.diff(pos, axis=0) * bar[1:]).sum(1) * 1000
            speed = np.linalg.norm(np.diff(pos, axis=0), axis=1) * 1000
            shots = [i + 1 for i in range(10, len(back) - 1) if 8 < back[i] < 40 and back[i] >= back[i - 1] and back[i] >= back[i + 1]
                     and abs(bar[i + 1][1]) < 0.57 and speed[i - 10:i].mean() < 3]
            def aiming(W, p, bv):
                hip = W[lids.index(0)][:3, 3]; fwd = np.linalg.norm((p[:3, 3] - hip)[[0, 2]])
                return float(abs(bv[1]) < 0.5 and fwd > 0.3 and p[1, 3] > hip[1])
            aim_w = np.array([aiming(W, p, bv) for (W, p), bv in zip(props, bar)])
            aim_w = np.convolve(np.r_[[aim_w[0]] * 6, aim_w, [aim_w[-1]] * 6], np.ones(13) / 13, 'valid')
            cache[(eid, cut)] = dict(lc=lc, native=native, lclip=lclip, mirror=mirror, bar=bar, shots=shots, aim_w=aim_w, n=n)
        d = cache[(eid, cut)]
        if how[0] == 'loop': a, b, score = best_loop(d['lclip'], how[1], how[2], how[3], how[4])
        else:
            s0 = d['shots'][0]; a = s0 - how[1]
            V = [_pose_vec(d['lclip'], t) for t in range(d['n'])]
            b = min(range(s0 + how[2], min(s0 + how[3], d['n'] - 1) + 1), key=lambda t: float(np.abs(V[a] - V[t]).sum()))
            score = round(float(np.abs(V[a] - V[b]).sum()), 4)
        lclip = slice_clip(d['lclip'], a, b); shots = [f - a for f in d['shots'] if a <= f <= b]
        aim_w = d['aim_w'][a:b + 1]; bar = d['bar'][a:b + 1]
        vals = {b2: [[], [], []] for b2 in arm_ids}; gvals = [[] for _ in range(7)]
        for t in range(b - a + 1):
            Wk = kp.worlds(kmodel, lclip, t)
            loc0 = {b2: Wk[model['bones'][idx[b2]]['parent']][:3, :3].T @ Wk[idx[b2]][:3, :3] for b2 in arm_ids}
            w = float(aim_w[t]); locs = dict(loc0)
            if w > 1e-3:
                dvec = bar[t].copy(); dvec[1] = 0; dvec /= max(np.linalg.norm(dvec), 1e-6)
                kick = max((1 - (t - f0) / 5 for f0 in shots if 0 <= t - f0 < 5), default=0.0)
                gy = -dvec; gz = np.array([0, 1.0, 0]); gx = np.cross(gy, gz); Rg = np.c_[gx, gy, gz]
                if kick: Rg = _rodrigues(gx, np.radians(9) * kick) @ Rg
                eye = Wk[idx[4]][:3, 3] + np.array([0, 0.055, 0]) + dvec * 0.06
                GW = np.eye(4); GW[:3, :3] = Rg; GW[:3, 3] = eye + dvec * (0.50 - 0.02 * kick) - Rg @ sight - np.array([0, 0.03, 0])
                HR = GW @ np.linalg.inv(grip['R']); HL = GW @ np.linalg.inv(support)
                ik = two_bone_ik(Wk, idx, (5, 7, 8, 9), HR[:3, 3], HR[:3, :3])
                ik.update(two_bone_ik(Wk, idx, (10, 12, 13, 14), HL[:3, 3], HL[:3, :3]))
                locs = {b2: _blend_rot(loc0[b2], ik[b2], w) for b2 in arm_ids}
            for b2 in arm_ids:
                e3 = euler_zyx(locs[b2])
                for k in range(3): vals[b2][k].append(e3[k])
            G = grip['R']; e3 = euler_zyx(G[:3, :3])
            for k in range(3): gvals[k].append(G[k, 3]); gvals[3 + k].append(e3[k])
            gvals[6].append(1.0)
        key = f"{d['lc']}|{variant}"; stem = f'{eid}_cut{cut}_{variant}'
        origin = [next((tr['values'][a] for tr in d['native']['tracks'] if tr['bone'] == -1 and tr['channel'] == ch), 0.0) for ch in range(3)]
        ov = dict(clip=stem, family='em0940', label=label, tracks=prop_tracks(GUN_BONES['R']['id'], gvals) + arm_tracks(vals),
                  shots=[dict(frame=f, gun='R') for f in shots], range=[a, b], rootOrigin=[round(float(v), 5) for v in origin],
                  style='realistic', mirror=d['mirror'], handPose='gun2' if float(aim_w.mean()) > 0.5 else 'gun')
        lift, lows = ground(model, B, key, ov, dict(slice_clip(d['native'], a, b), id=key, family='em0940')); ov['rootLift'] = np.round(lift, 4).tolist()
        ov['rootYaw'] = face_yaw(model, key, ov, dict(slice_clip(d['native'], a, b), id=key, family='em0940'))
        out[key] = ov
        rep[stem] = dict(range=[a, b], frames=b - a + 1, loopScore=score, mirrored=d['mirror'], aimFraction=round(float(aim_w.mean()), 2),
                         shots=shots, rootLiftRange=[round(float(lift.min()), 3), round(float(lift.max()), 3)],
                         lowestPointAfterLift=round(float((lows + lift).min()), 4))
    return out, rep
