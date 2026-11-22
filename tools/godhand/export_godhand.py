"""Export God Hand explorer solved clips to plain keyframe JSON for retargeting (assets/native/godhand/*.json + index.json).
Every clip is sampled through the explorer's own solver (serve_main.Server.raw_sample, the same path as the UI's
/api/clip?space=inspection-local, cached in exports/godhand/cache/solved-clips-v2) ON THE RIG IT WAS PICKED ON: a
motion shared by several banks looks different on another skeleton. See tools/godhand/README.md.
Usage: python tools/godhand/export_godhand.py     (edit `singles` / the pairs below to change what is exported)
"""
import sys, json, hashlib, math
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
from godhand_server import server, R
from godhand_seq import seqinfo
OUT = Path(r'D:/Games/gamesfordecomp/ports/madworld-rinrin/assets/native/godhand')
OUT.mkdir(parents=True, exist_ok=True)

S = server()
LIB = json.loads((R / 'interactions.json').read_text())
MOT = S.motions

# ---------------------------------------------------------------- semantics
BASE = {
    0: ('hips', 'hips / root of hierarchy (pelvis height); parent of spine (1) and lower-body pivot (17)'),
    1: ('spine', 'lower spine / waist'),
    2: ('chest', 'upper spine / chest; parent of both clavicles and neck'),
    3: ('neck', 'neck'),
    4: ('head', 'head'),
    5: ('clavicle_r', 'right clavicle / shoulder girdle'),
    6: ('shoulder_helper_r', 'right shoulder helper (leaf sibling of upper arm at the same pivot; deform/pad helper)'),
    7: ('upper_arm_r', 'right upper arm (shoulder joint)'),
    8: ('forearm_r', 'right forearm (elbow joint)'),
    9: ('forearm_twist_r', 'right forearm twist / mid-forearm (between elbow and wrist)'),
    10: ('hand_r', 'right hand (wrist joint)'),
    11: ('clavicle_l', 'left clavicle / shoulder girdle'),
    12: ('shoulder_helper_l', 'left shoulder helper (leaf sibling of upper arm)'),
    13: ('upper_arm_l', 'left upper arm (shoulder joint)'),
    14: ('forearm_l', 'left forearm (elbow joint)'),
    15: ('forearm_twist_l', 'left forearm twist / mid-forearm'),
    16: ('hand_l', 'left hand (wrist joint)'),
    17: ('pelvis_lower', 'lower-body pivot (child of hips at same position; parent of both thighs)'),
    18: ('thigh_r', 'right thigh (hip joint); IK chain root'),
    19: ('shin_r', 'right shin (knee joint); IK chain middle'),
    20: ('foot_r', 'right foot (ankle joint); IK chain end'),
    21: ('toe_r', 'right toe / ball of foot'),
    22: ('thigh_l', 'left thigh (hip joint); IK chain root'),
    23: ('shin_l', 'left shin (knee joint); IK chain middle'),
    24: ('foot_l', 'left foot (ankle joint); IK chain end'),
    25: ('toe_l', 'left toe / ball of foot'),
    26: ('head_extra', 'head child (face/jaw/hat helper), not animated by most clips'),
    27: ('locator_a', 'parentless locator at actor origin (prop/effect slot); not body'),
    28: ('locator_b', 'parentless locator at actor origin (prop/effect slot); not body'),
}


def semantic(model, i):
    parts = model['parts']; p = parts[i]['parent']
    if i in BASE: return BASE[i]
    pn = BASE.get(p, (f'bone_{p}',))[0] if p >= 0 else 'actor'
    return (f'extra_{i}_{pn}', f'rig-specific extra bone, child of {pn} ({p}); accessory/cloth/helper')


def skeleton(pid):
    prof = S.profiles[pid]; model = json.loads((R / prof['model']).read_text())
    bones = []
    for i, part in enumerate(model['parts']):
        name, desc = semantic(model, i)
        world = [-v for v in model['source_bind_offsets'][i]]
        bones.append(dict(index=i, name=name, semantic=desc, parent=part['parent'],
                          mirror=part['link'] if part['link'] >= 0 else None,
                          rest_local=dict(translation=[round(v, 6) for v in part['pos']], rotation=[0, 0, 0, 1], scale=[1, 1, 1]),
                          rest_model_position=[round(v, 6) for v in world]))
    return model, dict(profile=pid, character=prof.get('display_name') or prof.get('name'), model_file=prof['model'],
                       joint_count=len(bones), bones=bones,
                       rest_pose_note='Bind pose is translation-only (identity rotations, unit scale) in a T-pose. parent=-1 bones are relative to the actor/ground root (see root_motion). rest_model_position is the model-space bind position (= -source_bind_offsets).')

# ---------------------------------------------------------------- math

def mat_to_trs(m):
    t = m[:3, 3].copy(); b = m[:3, :3].copy(); s = np.linalg.norm(b, axis=0)
    if np.linalg.det(b) < 0: s[0] = -s[0]
    r = np.eye(3) if np.any(np.abs(s) < 1e-9) else b / s
    return t, quat(r), s


def quat(r):
    tr = r[0, 0] + r[1, 1] + r[2, 2]
    if tr > 0:
        k = math.sqrt(tr + 1.0) * 2; w = 0.25 * k; x = (r[2, 1] - r[1, 2]) / k; y = (r[0, 2] - r[2, 0]) / k; z = (r[1, 0] - r[0, 1]) / k
    elif r[0, 0] > r[1, 1] and r[0, 0] > r[2, 2]:
        k = math.sqrt(1.0 + r[0, 0] - r[1, 1] - r[2, 2]) * 2; w = (r[2, 1] - r[1, 2]) / k; x = 0.25 * k; y = (r[0, 1] + r[1, 0]) / k; z = (r[0, 2] + r[2, 0]) / k
    elif r[1, 1] > r[2, 2]:
        k = math.sqrt(1.0 + r[1, 1] - r[0, 0] - r[2, 2]) * 2; w = (r[0, 2] - r[2, 0]) / k; x = (r[0, 1] + r[1, 0]) / k; y = 0.25 * k; z = (r[1, 2] + r[2, 1]) / k
    else:
        k = math.sqrt(1.0 + r[2, 2] - r[0, 0] - r[1, 1]) * 2; w = (r[1, 0] - r[0, 1]) / k; x = (r[0, 2] + r[2, 0]) / k; y = (r[1, 2] + r[2, 1]) / k; z = 0.25 * k
    q = np.array([x, y, z, w]); return q / np.linalg.norm(q)


def mats(frame, J):
    return [np.array(frame[i * 16:i * 16 + 16], dtype=float).reshape(4, 4).T for i in range(J + 1)]


def roty(deg_or_rad, rad=False):
    a = deg_or_rad if rad else math.radians(deg_or_rad); c, s = math.cos(a), math.sin(a)
    m = np.eye(4); m[0, 0] = c; m[0, 2] = s; m[2, 0] = -s; m[2, 2] = c; return m


def r6(v): return [round(float(x), 6) for x in v]


def model_world(L, parts, J):
    W = {}
    def w(i):
        if i in W: return W[i]
        p = parts[i]['parent']; W[i] = (L[J] if p < 0 else w(p)) @ L[i]; return W[i]
    return [w(i) for i in range(J)]


def encode_frames(clip_frames, parts, J):
    """Return frames list + root motion, keeping quaternion hemisphere continuity."""
    prev = [None] * J; prev_root = None; out = []; root = []; nonunit = 0
    for fr in clip_frames:
        L = mats(fr, J); local = []
        for i in range(J):
            t, q, s = mat_to_trs(L[i])
            if prev[i] is not None and np.dot(prev[i], q) < 0: q = -q
            prev[i] = q
            if np.max(np.abs(np.abs(s) - 1)) > 1e-4: nonunit += 1
            local.append(r6(t) + r6(q) + r6(s))
        t, q, s = mat_to_trs(L[J])
        if prev_root is not None and np.dot(prev_root, q) < 0: q = -q
        prev_root = q
        W = model_world(L, parts, J)
        root.append(dict(actor_translation=r6(t), actor_rotation=r6(q), hips_world_translation=r6(W[0][:3, 3])))
        out.append(local)
    return out, root, nonunit, None


def world_positions(clip_frames, parts, J, bones):
    res = []
    for fr in clip_frames:
        L = mats(fr, J); W = model_world(L, parts, J); res.append(np.array([W[b][:3, 3] for b in bones]))
    return np.array(res)


STRIKE_CANDIDATES = {'hand_r': 10, 'hand_l': 16, 'foot_r': 20, 'toe_r': 21, 'foot_l': 24, 'toe_l': 25, 'knee_r(shin_r)': 19, 'knee_l(shin_l)': 23, 'head': 4}


def striking(clip_frames, parts, J, windows):
    names = list(STRIKE_CANDIDATES); idx = [STRIKE_CANDIDATES[n] for n in names]
    P = world_positions(clip_frames, parts, J, idx); n = len(P); res = []
    for w in windows:
        a = max(1, w['start'] - 1); b = min(n - 1, w['end'])
        if b < a: res.append(None); continue
        sp = np.array([np.linalg.norm(P[f] - P[f - 1], axis=1) for f in range(a, b + 1)]).mean(axis=0) * 30
        order = np.argsort(-sp)
        res.append(dict(window=[w['start'], w['end']], flag=w['flag'], bone=names[order[0]], bone_index=idx[order[0]],
                        speed_m_per_s=round(float(sp[order[0]]), 3),
                        runner_up=dict(bone=names[order[1]], speed_m_per_s=round(float(sp[order[1]]), 3)),
                        method='HEURISTIC: fastest candidate end-effector (world, incl. root motion) over the active window at 30 fps; not native collision data'))
    return res

# ---------------------------------------------------------------- common header
UNITS = dict(source='God Hand model units', meters_per_unit=1.0,
             note='Units are treated as metres (Gene hips 1.14, knee 0.575, ankle 0.135, head pivot 1.65). No authoritative metre conversion recovered; scale 1.0 is an inference from human proportions.')
AXES = dict(up='+Y', forward='+Z', character_right='-X', character_left='+X', handedness='right-handed (as rendered by the explorer/three.js; no mirroring applied)',
            note='Gene meshes named RH_*/GHA_* (right hand / God Hand arm) sit on -X, so bones 5-10 are the right arm. Attack root motion travels toward +Z. Bevy is also Y-up right-handed but its glTF convention faces -Z/+Z: rotate 180 deg about Y if your Kreely rig faces -Z.')
LAYOUT = ('frames[f][bone] = [tx,ty,tz, qx,qy,qz,qw, sx,sy,sz]: LOCAL transform relative to the parent bone (parent=-1 => relative to the actor/ground root). '
          'root_motion[f]: actor_translation/actor_rotation = ground root (XZ travel + yaw, applied above bone 0); hips_world_translation = bone 0 in actor-container space (actor * hips local). '
          'Full model pose = actor * hierarchy.')
SOLVER = ('Explorer server path /api/clip?space=inspection-local (serve_main.Server.raw_sample -> clip_solver.solve, cache/solved-clips-v2): decoded mtb3 curves + original-ELF leg IK (two-bone chains 18-19-20 and 22-23-24) on a flat y=0 inspection floor. '
          'Body only; native cloth/secondary tracks not included. native_verified=false per explorer.')


def seq_windows(mid):
    info = seqinfo(mid); primary = min(info) if info else None
    if primary is None: return None, [], info
    flags = info[primary]['flags']
    win = [dict(start=a, end=b, flag=f) for a, b, f, _ in flags if f in (1, 2)]
    return primary, win, info


def clip_doc(key, pid, mid, label, ui_label, description, frames=None, timeline=None, extra=None):
    model, sk = skeleton(pid); parts = model['parts']
    clip = S.raw_sample(pid, mid); J = clip['joints']
    src = clip['frames']
    if timeline is not None:
        src = [clip['frames'][min(len(clip['frames']) - 1, int(math.floor(t)))] for t in timeline]
    fr, root, nonunit, _ = encode_frames(src, parts, J)
    m = MOT[mid]
    primary, win, info = seq_windows(mid)
    doc = dict(key=key, label=label, explorer_ui_label=ui_label, user_description=description,
               source=dict(motion_id=mid, bank=m['bank'], slot=m['slot'], sha256=m['sha256'], explorer_name=m['name'],
                           identical_resources=[x['id'] for x in S.motions.values() if x['sha256'] == m['sha256']][:24],
                           source_frames=m['frames'], motion_joint_tracks=m['joints'], rig_profile=pid),
               fps=30, fps_note='Explorer playback rate (descriptor fps=30; one native clock tick per source frame at dt=1). SEQ speed multipliers are NOT baked in (see seq.speed).',
               frame_count=len(fr), units=UNITS, axes=AXES, layout=LAYOUT, solver=SOLVER,
               skeleton=sk, seq=dict(slot=primary, records={str(k): dict(flags=[list(x) for x in v['flags']], speed=[list(x) for x in v['speed']]) for k, v in info.items()},
                                     note='flags rows are [start,end,bits,unused] in source frames (end inclusive). bits 1 and 2 = the two damage-setter configurations (hit-active window). Other bits (16/32/64/1024/...) are unidentified (likely cancel/chain/state windows). Which adjacent SEQ the game pairs with a candidate MOT is unproven except where native data says so; lowest slot shown as primary.'),
               hit_windows=win, root_motion=root, frames=fr)
    if timeline is not None:
        doc['timeline_source_frames'] = [float(t) for t in timeline]
    doc['nonunit_scale_samples'] = nonunit
    if win:
        doc['striking_bone'] = striking(src, parts, J, win if timeline is None else [])
    if m.get('native_attack_cases'):
        nac = m['native_attack_cases']
        doc['native_attack'] = nac
        doc['suggested_damage'] = dict(value=None, packet_field_44=nac['windows'][0].get('packet_field_44'), damage_kind=nac['windows'][0].get('damage_kind'),
                                       note='Native fixture writes packet field +0x44 = 10 and damage kind; the explorer explicitly does NOT treat +0x44 as a proven damage amount. Use 10 as a relative placeholder if needed.')
    else:
        doc['suggested_damage'] = dict(value=None, note='No native damage data for this candidate (SEQ flags only). Gameplay damage must be chosen by you.')
    if extra: doc.update(extra)
    path = OUT / f'{key}.json'
    path.write_text(json.dumps(doc, separators=(',', ':')), encoding='utf-8')
    return doc, model, clip


INDEX = []


def add(doc, kind='clip', **kw):
    INDEX.append(dict(file=f"{doc['key']}.json", kind=kind, label=doc['label'], explorer_ui_label=doc['explorer_ui_label'],
                      user_description=doc['user_description'], motion=doc['source']['motion_id'], sha256=doc['source']['sha256'],
                      rig_profile=doc['source']['rig_profile'], fps=doc['fps'], frames=doc['frame_count'], hit_windows=doc['hit_windows'],
                      striking_bone=[s and s['bone'] for s in doc.get('striking_bone', [])], **kw))

# ---------------------------------------------------------------- clips
G = 'pl00-1'
singles = [
    ('gene_063_pistol_whip', G, 'pl00:236', 'Gene attack / action candidate 063', 'Attack / action candidate 063', 'pistol whip'),
    ('gene_069_pistol_whip_alt', G, 'pl00:247', 'Gene attack / action candidate 069', 'Attack / action candidate 069', 'pistol whip alternate'),
    ('gene_105_uppercut_combo', G, 'pl00:344', 'Gene attack / action candidate 105', 'Attack / action candidate 105', 'left and right uppercut heavy combo'),
    ('gene_119_knee', G, 'pl00:447', 'Gene attack / action candidate 119', 'Attack / action candidate 119', 'knee attack'),
    ('gene_150_cross_punch', G, 'pl00:59', 'Gene attack / action candidate 150', 'Attack / action candidate 150', 'basic cross punch'),
    ('gene_154_powered_punch', G, 'pl00:434', 'Gene attack / action candidate 154', 'Attack / action candidate 154', 'powered-up punch'),
    ('gene_166_sweep_kick', G, 'pl00:336', 'Gene attack / action candidate 166 (now "Sweep kick")', 'Sweep kick (previous label: Attack / action candidate 166)', 'sweep kick'),
    # labels above 166 moved down by one when "candidate 166" became "Sweep kick": the user's 256/351/373/535 are these
    ('gene_255_punch_combo', G, 'pl00:402', 'Gene attack / action candidate 255 (was 256)', 'Attack / action candidate 255', 'full punch combo'),
    ('gene_350_spin_kick', G, 'pl00:267', 'Gene attack / action candidate 350 (was 351)', 'Attack / action candidate 350', 'spin kick knockdown'),
    ('gene_372_step_jab', G, 'pl00:55', 'Gene attack / action candidate 372 (was 373)', 'Attack / action candidate 372', 'forward step and jab'),
    ('gene_534_turn_jab', G, 'pl00:32', 'Gene attack / action candidate 534 (was 535)', 'Attack / action candidate 534', 'standing turn and jab'),
]
for key, pid, mid, label, ui, desc in singles:
    doc, _, _ = clip_doc(key, pid, mid, label, ui, desc); add(doc)
    print(key, doc['frame_count'], doc['hit_windows'], [s and (s['bone'], s['speed_m_per_s']) for s in doc.get('striking_bone', [])], 'nonunit', doc['nonunit_scale_samples'])

# full receiver reaction and get-up candidate
doc, _, _ = clip_doc('smallfry_375_sweep_knockdown_reaction', 'em00-1', 'em00:375', 'Sweep receiver reaction (em00:375)', 'Hit / captured reaction 032',
                     'knocked-down sweep reaction: native-selected receiver response to sweep kick (damage kind 23) for healthy common class 0x200 (em00-1)',
                     extra=dict(native_selection=json.loads((R / 'evidence/combat-native/sweep-pair.json').read_text()).get('native_receiver_fixture'),
                                selection_authority='HANDOFF.md 2026-11-10: native receiver router 0x20a178 -> handler 0x253de8 selects em00:375 / SEQ376 (backward flip, supine landing). Contact/root physics/recovery unverified.'))
add(doc, kind='reaction')
GETUP = 'em00:1505'
doc, _, _ = clip_doc('smallfry_getup_candidate', 'em00-1', GETUP, 'Get-up candidate after em00:375', MOT[GETUP]['name'],
                     'HEURISTIC get-up: no native recovery link recovered. Chosen because its first frame best matches the last frame of em00:375 (mean joint error ~0.10 m over hips/head/hands/feet/knees) and it ends standing.',
                     extra=dict(selection_authority='HEURISTIC pose match over all em00-1 cached clips; alternatives with equal score: em00:3766; next: em00:217/em00:389/em00:488 (~0.21 m). Not native.'))
add(doc, kind='reaction_heuristic')

# ---------------------------------------------------------------- paired helpers

def pair(key, desc_id, subject, roles_out, label, ui_label, description):
    d = S.interaction(subject, desc_id)
    placements = []
    actors = []
    for i, role in enumerate(d['actors']):
        pid = role['profile']; model, sk = skeleton(pid); parts = model['parts']
        tl = role.get('timeline')
        clips = {mid: S.raw_sample(pid, mid) for mid in (role.get('motions') or [role['motion']])}
        if tl is None:
            c = clips[role['motion']]; n = d['frames']
            tl = [[role['motion'], (t % len(c['frames'])) if role['role'] == 'opponent' else min(t, len(c['frames']) - 1)] for t in range(n)]
        J = sk['joint_count']
        src = [clips[m]['frames'][max(0, min(len(clips[m]['frames']) - 1, int(math.floor(t))))] for m, t in tl]
        fr, root, nonunit, _ = encode_frames(src, parts, J)
        actors.append(dict(role=role['role'], profile=pid, parts=parts, J=J, src=src, tl=tl, fr=fr, root=root, sk=sk, clips=clips))
    # per-tick containers
    keys = d.get('alignment_keys')
    rel = []
    for t in range(d['frames']):
        C = []
        for i in range(len(actors)):
            if keys:
                k = [x for x in keys if x['tick'] <= t][-1]['actors'][i]
                c = np.eye(4); c[:3, 3] = k['position']; c = c @ roty(k['yaw'], rad=True)
            elif i == 1:
                c = np.eye(4); c[:3, 3] = d['placement']['position']; c = c @ roty(d['placement']['yaw'])
            else:
                c = np.eye(4)
            C.append(c)
        A = [C[i] @ mats(actors[i]['src'][t], actors[i]['J'])[actors[i]['J']] for i in range(2)]
        H = [A[i] @ mats(actors[i]['src'][t], actors[i]['J'])[0] for i in range(2)]
        ra = np.linalg.inv(A[0]) @ A[1]; rh = np.linalg.inv(H[0]) @ H[1]
        ta, qa, _ = mat_to_trs(ra); th, qh, _ = mat_to_trs(rh)
        rel.append(dict(tick=t, gene_container=dict(translation=r6(C[0][:3, 3]), rotation=r6(quat(C[0][:3, :3]))),
                        receiver_container=dict(translation=r6(C[1][:3, 3]), rotation=r6(quat(C[1][:3, :3]))),
                        receiver_root_in_gene_root=dict(translation=r6(ta), rotation=r6(qa)),
                        receiver_hips_in_gene_hips=dict(translation=r6(th), rotation=r6(qh)),
                        gene_hips_world=r6(H[0][:3, 3]), receiver_hips_world=r6(H[1][:3, 3])))
    files = []
    for a, (suffix, lab) in zip(actors, roles_out):
        mids = sorted({m for m, _ in a['tl']}, key=lambda m: [x for x, _ in a['tl']].index(m))
        main = a['tl'][0][0] if len(mids) == 1 else [m for m in mids if m != 'em00:258'][0]
        m = MOT[main]
        flagsets = {mid: seq_windows(mid) for mid in mids}
        doc = dict(key=f'{key}_{suffix}', label=f'{label} - {lab}', explorer_ui_label=ui_label, user_description=description,
                   source=dict(interaction_id=desc_id, role=a['role'], motion_id=main, motions=mids, bank=m['bank'], slot=m['slot'], sha256=m['sha256'], explorer_name=m['name'],
                               motion_shas={mid: MOT[mid]['sha256'] for mid in mids}, rig_profile=a['profile']),
                   fps=d['fps'], fps_note='One entry per explorer timeline tick (30 fps); timeline gives the source MOT frame shown at each tick.',
                   frame_count=len(a['fr']), units=UNITS, axes=AXES, layout=LAYOUT + ' Root motion here is the clip\'s own actor root; the pair placement container is in the _pair file.', solver=SOLVER,
                   skeleton=a['sk'], timeline=[[mm, float(tt)] for mm, tt in a['tl']],
                   seq={mid: dict(slot=v[0], hit_windows_source_frames=v[1], records={str(k): dict(flags=[list(x) for x in r['flags']], speed=[list(x) for x in r['speed']]) for k, r in v[2].items()}) for mid, v in flagsets.items()},
                   root_motion=a['root'], frames=a['fr'])
        (OUT / f"{doc['key']}.json").write_text(json.dumps(doc, separators=(',', ':')), encoding='utf-8')
        files.append(doc['key'] + '.json')
        INDEX.append(dict(file=doc['key'] + '.json', kind='paired_actor', label=doc['label'], explorer_ui_label=ui_label, user_description=description,
                          motion=main, motions=mids, sha256=m['sha256'], rig_profile=a['profile'], fps=doc['fps'], frames=doc['frame_count']))
    pdoc = dict(key=f'{key}_pair', label=label, explorer_ui_label=ui_label, interaction_id=desc_id, actor_files=files, fps=d['fps'], frame_count=d['frames'],
                units=UNITS, axes=AXES,
                placement=d.get('placement'), alignment_keys=keys, source=d.get('source'), authority=d.get('authority'), native_verified=False,
                composition='World of actor i at tick t = container_i(t) * actor_root_i(t) * bone hierarchy. container = T(position) * RotY(yaw) from alignment_keys (latest key with tick<=t) or placement (receiver only, yaw in degrees) when no keys. '
                            'receiver_root_in_gene_root = inverse(container_g*actor_g) * (container_r*actor_r); receiver_hips_in_gene_hips uses bone 0 world matrices.',
                per_tick=rel)
    (OUT / f'{key}_pair.json').write_text(json.dumps(pdoc, separators=(',', ':')), encoding='utf-8')
    INDEX.append(dict(file=f'{key}_pair.json', kind='pair_placement', label=label, explorer_ui_label=ui_label, interaction_id=desc_id, actor_files=files, fps=d['fps'], frames=d['frames']))
    return pdoc, actors


pdoc, actors = pair('grab_0011c348_suplex', 'sequence:96b7c17f0a1b600b', G, [('gene', 'Gene (player)'), ('receiver', 'receiver (Henchman thin, tattooed em00-3)')],
                    'Grab sequence 0011c348 (suplex)', 'Grab sequence 0011c348 / Henchman · thin, tattooed / no escape input / inspection timing', 'suplex paired grab')
print('grab', pdoc['frame_count'], pdoc['per_tick'][0]['receiver_root_in_gene_root'], pdoc['per_tick'][48]['receiver_hips_in_gene_hips'])
pdoc, actors = pair('sweep_kick_pair', 'attack:pl00-1:4c4551692d52bf40', G, [('gene', 'Gene sweep kick (from source frame 3)'), ('receiver', 'receiver em00-1 idle then em00:375 knockdown')],
                    'Sweep kick native hit pair', 'Sweep kick / native hit 23', 'sweep kick with native-selected receiver knockdown; contact assumed at source frame 13 = tick 10')
print('sweep', pdoc['frame_count'], pdoc['per_tick'][0]['receiver_root_in_gene_root'])

# ---------------------------------------------------------------- Gene skeleton
model, sk = skeleton(G)
sk['ik_chains'] = S.raw_sample(G, 'pl00:236')['ik_chains']
sk['mirror_note'] = 'mirror = source "link" field (left/right partner index).'
sk['madworld_mapping_hint'] = {b['name']: b['index'] for b in sk['bones'] if b['index'] <= 25}
sk['units'] = UNITS; sk['axes'] = AXES
(OUT / 'gene_skeleton.json').write_text(json.dumps(sk, indent=1), encoding='utf-8')

index = dict(generated_from=str(R), solver=SOLVER, solver_hash=S.solver_hash, units=UNITS, axes=AXES, layout=LAYOUT, fps=30,
             skeleton_file='gene_skeleton.json', clips=INDEX,
             label_resolution=dict(
                 note='Labels resolved against the current explorer motion list (app.js plainRows emulation over explorer.json) and interactions.json.',
                 gene_166='No Gene motion is currently labelled "candidate 166". pl00:336 carries identification.previous_label "Attack / action candidate 166" and is now named "Sweep kick" (HANDOFF line 3). em00:2940 is a different motion (sha 4d481d7b...) still labelled 166 but only in enemy banks and not in Gene\'s list.',
                 enemy_candidates='256/351/535 are em00-bank henchman motions (Small Fry em02-1 et al.), 373 is Elvis (em03-1/em03-3/em60-1) only. 351 also appears in Gene-sleeveless (em7e-1) list; exported on both rigs.',
                 grab='92 Gene descriptors are labelled "Grab sequence 0011c348 / <receiver> / <scenario>"; exported the first listed (Henchman thin tattooed em00-3, no escape input). Gene motion bank varies with receiver (em00:327, em02:327, ...).'))
(OUT / 'index.json').write_text(json.dumps(index, indent=1), encoding='utf-8')
S.server_close()
print('done', len(INDEX))
