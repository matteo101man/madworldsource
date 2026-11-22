"""HTTP regression for Kreese Kreely in the model explorer.

python -I tools/kreely/regress_kreely.py [--base http://127.0.0.1:8795] [--out evidence/kreely/kreely-http-regression.json]
Checks: Kreely is in the catalog; his skeleton keeps pl0030's 36 bone ids/parents (own joint positions, rotation-only
retarget) plus finger chains under the hand bones; skin data is sane; his animation menu (motion family pl0030,
"Character source clips") contains every clip in Jack's menu plus all 248 pl/pl0030.dat clips; every pl/pl0030.dat clip
decodes; the textures (source, color, AB) are served; hand poses exist; the Color version checkbox is still default-off.
"""
import argparse, json, re, sys, urllib.parse, urllib.request
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kreely_pose as kp

KREELY, JACK = 'custom/kreely#kreely.mdb', 'pl/pl0030.dat#pl0030.mdb@19552'


def get(base, path):
    with urllib.request.urlopen(base + path, timeout=120) as r:
        body = r.read()
        return json.loads(body) if r.headers.get_content_type() == 'application/json' else body


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('--base', default='http://127.0.0.1:8795')
    ap.add_argument('--out', default=str(Path(__file__).resolve().parents[2] / 'evidence/kreely/kreely-http-regression.json'))
    a = ap.parse_args(); q = lambda s: urllib.parse.quote(s, safe='')
    res, checks = {}, {}
    html = get(a.base, '/').decode('utf-8')
    checks['colorCheckboxPresent'] = 'id="colorize"' in html
    checks['colorCheckboxDefaultOff'] = bool(re.search(r'<input id="colorize" type="checkbox">', html))
    color = get(a.base, '/color.js').decode('utf-8')
    checks['kreelyPalettePresent'] = 'kreely:{' in color and 'native.regionNames' in color
    cat = get(a.base, '/api/catalog')
    item = next((m for m in cat['models'] if m['id'] == KREELY), None)
    checks['kreelyInCatalog'] = item is not None
    k, j = get(a.base, '/api/model?id=' + q(KREELY)), get(a.base, '/api/model?id=' + q(JACK))
    n0 = len(j['bones'])
    checks['pl0030BoneIdsAndParents'] = [(b['id'], b['parent']) for b in k['bones'][:n0]] == [(b['id'], b['parent']) for b in j['bones']]
    fingers = [b for b in k['bones'][n0:] if b['id'] not in (k.get('propBones') or [])]; hand = {9: n0 and [b['id'] for b in j['bones']].index(9), 14: [b['id'] for b in j['bones']].index(14)}
    checks['fingerChains'] = len(fingers) == 30 and all(b['parent'] in (hand[9], hand[14]) or b['parent'] >= n0 for b in fingers)         and len({b['id'] for b in fingers} & {x for c in cat['clips'] for x in c['bones']}) == 0
    checks['handPoses'] = set(k.get('handPoses', {})) >= {'fist', 'relaxed', 'open'} and k.get('defaultHandPose') == 'fist'
    checks['retargetRotationOnly'] = bool(k.get('retarget', {}).get('rotationOnly')) and 0.8 < k['retarget']['rootScale'] < 1.2
    for bank in ('primary', 'ab', 'color'):
        checks['texture_' + bank] = len(get(a.base, '/api/texture?id=' + q(KREELY) + '&index=0&bank=' + bank)) > 1000
    m = k['meshes'][0]; n = len(m['positions'])
    checks['skinArraysAligned'] = all(len(m[f]) == n for f in ('normals', 'uvs', 'joints', 'weights', 'regions'))
    checks['weightsNormalized'] = max(abs(sum(w) - 1) for w in m['weights']) < 1e-3
    checks['jointIndicesValid'] = all(0 <= i < len(k['bones']) for js in m['joints'] for i in js)
    used = sorted({k['bones'][i]['id'] for js, ws in zip(m['joints'], m['weights']) for i, w in zip(js, ws) if w > 0})
    ids = {b['id'] for b in k['bones']}
    ignored = set(k.get('ignoreMotionBones') or [])
    compatible = lambda c: all(b == -1 or b >= 0xf60 or b in ids or b in ignored for b in c['bones'])
    jids = {b['id'] for b in j['bones']}
    jack_compatible = lambda c: all(b == -1 or b >= 0xf60 or b in jids for b in c['bones'])
    family = item['custom']['motionFamily']
    kreely_menu = sorted(c['id'] for c in cat['clips'] if c['name'].startswith(family + '_') and compatible(c))
    jack_menu = sorted(c['id'] for c in cat['clips'] if c['name'].startswith('pl0030_') and jack_compatible(c))
    checks['motionMenuContainsJacks'] = set(jack_menu) <= set(kreely_menu)
    own = [c for c in cat['clips'] if c['bank'] == 'pl/pl0030.dat']
    failures = []
    for c in own:
        try:
            r = get(a.base, '/api/clip?id=' + q(c['id']))
            if r['frames'] != c['frames']: failures.append((c['id'], 'frame count'))
        except Exception as e:
            failures.append((c['id'], str(e)))
    checks['allPl0030DatClipsDecode'] = not failures
    # motion fidelity: every pl0030 bone's world rotation on Kreely equals Jack's, 5 frames per clip, all pl0030.dat clips
    worst = 0.0; sampled = 0
    for c in own:
        if not jack_compatible(c): continue
        clip = get(a.base, '/api/clip?id=' + q(c['id']))
        for t in np.linspace(0, clip['frames'] - 1, 5):
            Wk, Wj = kp.worlds(k, clip, t), kp.worlds(j, clip, t)
            worst = max(worst, float(np.abs(Wk[:n0, :3, :3] - Wj[:n0, :3, :3]).max())); sampled += 1
    checks['boneRotationsMatchJack'] = worst < 1e-9
    res['rotationFidelity'] = dict(framesSampled=sampled, maxAbsDiff=worst)
    checks['allPl0030DatClipsInKreelyMenu'] = all(c['id'] in set(kreely_menu) for c in own)
    # some pl0030.dat clips reference bones pl0030.mdb lacks; they are hidden for Jack too (same skeleton -> same set)
    unmapped = {c['name']: sorted(set(c['bones']) - jids - {-1}) for c in own if not jack_compatible(c)}
    # --- handgun (secondary weapon): overlays, textures, visibility, shots, aim height, grip, one-handed arm, floor ---
    ov = k.get('overlays') or {}
    allclips = {c2['id']: c2 for c2 in cat['clips'] + (cat.get('events') or {}).get('clips', [])}
    names = {o['clip'] for o in ov.values()}
    checks['handgunClipSet'] = names == {'ev93_cut8_aim', 'ev93_cut8_fire', 'ev93_cut18_idle', 'em0400_0002', 'em0400_0031', 'em0400_0033',
                                         'em0400_0050', 'em0400_0051', 'em0400_0052', 'em0400_0057', 'em0400_0058', 'em0400_0105', 'em0400_0201'}
    checks['handgunClipsExistAndCompatible'] = all(key.split('|')[0] in allclips and compatible(allclips[key.split('|')[0]]) for key in ov)
    for idx_, bank in (('1', 'primary'), ('1', 'ab'), ('2', 'primary'), ('2', 'ab'), ('0', 'fx')):
        checks[f'texture_{bank}_{idx_}'] = len(get(a.base, f'/api/texture?id={q(KREELY)}&index={idx_}&bank={bank}')) > 500
    ids2 = [b['id'] for b in k['bones']]; gi = ids2.index(0x640); I = {b: ids2.index(b) for b in (4, 9, 12, 14)}
    jclip = get(a.base, '/api/clip?id=' + q(sorted(c2['id'] for c2 in own)[0]))
    checks['gunHiddenInJackClips'] = float(np.abs(kp.worlds(k, jclip, 0)[gi][:3, :3]).max()) < 1e-9
    body = k['meshes'][0]; sub = dict(positions=body['positions'][::3], joints=body['joints'][::3], weights=body['weights'][::3])
    pm = next(m for m in k['meshes'] if m['name'] == 'kreely_pistol_R_KB'); P = np.array(pm['positions'])
    grip_pt = np.r_[P[P[:, 2] < P[:, 2].min() + 0.35 * np.ptp(P[:, 2])].mean(0), 1]
    B = kp.bind_worlds(k); hg = {}; floor_ok = one_hand_ok = True
    for key, o in ov.items():
        clip = kp.prepare(get(a.base, '/api/clip?id=' + q(key.split('|')[0])), key, o)
        lows, lhand_gun, eye_err, grips = [], [], [], []
        for t in range(0, clip['frames'], 3):
            W = kp.worlds(k, clip, t)
            lows.append(float(kp.skin(k, sub, W, B)[:, 1].min()))
            g3 = (W[gi] @ np.linalg.inv(B[gi]) @ grip_pt)[:3]
            lhand_gun.append(float(np.linalg.norm(W[I[14]][:3, 3] - g3)))
            if o['clip'] in ('ev93_cut8_aim', 'ev93_cut8_fire'):
                eye_err.append(float(W[gi][1, 3] - (W[I[4]][1, 3] + 0.055))); grips.append(float(np.linalg.norm(g3 - W[I[9]][:3, 3])))
        hg[o['clip']] = dict(frames=clip['frames'], range=o.get('range'), shots=[x['frame'] for x in o['shots']], style=o.get('style'),
                             lowestBodyPoint=round(min(lows), 4), leftHandToGripMin=round(min(lhand_gun), 3))
        floor_ok &= min(lows) > -0.005
        if o['clip'].startswith('em0400_') and o['clip'] != 'em0400_0051': one_hand_ok &= min(lhand_gun) > 0.12
        if o['clip'] == 'em0400_0051': checks['crouchBothHandsOnPistol'] = sorted(lhand_gun)[len(lhand_gun) // 2] < 0.12
        if eye_err:
            hg[o['clip']].update(gunMinusEyeHeight=[round(e, 3) for e in eye_err[:6]], gripToWrist=[round(x, 3) for x in grips[:6]])
            checks[f"aimAtEyeHeight_{o['clip']}"] = all(-0.22 < e < 0.05 for e in eye_err)
            checks[f"gripInRightHand_{o['clip']}"] = all(x < 0.12 for x in grips)
    checks['noClipBelowFloor'] = bool(floor_ok); checks['judeClipsOneHanded'] = bool(one_hand_ok)
    fire = next(o for o in ov.values() if o['clip'] == 'ev93_cut8_fire'); checks['fireClipHasShot'] = len(fire['shots']) == 1
    # synthetic (blended) transitions: defined, grounded, no pops
    loadk = lambda key: kp.prepare(get(a.base, '/api/clip?id=' + q(key.split('|')[0])), key, ov[key])
    sy = k.get('synthClips') or {}; synth_ok = set(sy) >= {'synth:raise', 'synth:lower', 'synth:aim-fire', 'synth:sequence'}
    for key in sy:
        clip = kp.make_synth(k, key, loadk); lows, prev, jump = [], None, 0.0
        for t in range(clip['frames']):
            W = kp.worlds(k, clip, t); J = W[:24, :3, 3]
            if t % 3 == 0: lows.append(float(kp.skin(k, sub, W, B)[:, 1].min()))
            if prev is not None: jump = max(jump, float(np.linalg.norm(J - prev, axis=1).max()))
            prev = J
        hg[key] = dict(frames=clip['frames'], lowestBodyPoint=round(min(lows), 4), maxJointJumpPerFrame=round(jump, 4))
        synth_ok &= min(lows) > -0.01 and jump < 0.12
    checks['blendedTransitions'] = bool(synth_ok)
    res['handgun'] = hg
    res.update(base=a.base, model=item, checks=checks, ok=all(checks.values()), weightedBoneIds=used,
               menuClips=dict(kreely=len(kreely_menu), jack=len(jack_menu), uniqueNames=len({c.split('#')[1].split('@')[0] for c in kreely_menu})),
               pl0030DatClips=len(own), pl0030DatClipsHiddenForJackOnly=unmapped, clipFailures=failures)
    Path(a.out).write_text(json.dumps(res, indent=1), encoding='utf-8')
    print(json.dumps(dict(ok=res['ok'], checks=checks, menuClips=res['menuClips'], pl0030DatClips=len(own), hidden=len(unmapped)), indent=1))


if __name__ == '__main__':
    main()
