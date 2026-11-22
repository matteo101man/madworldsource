"""Compare a live recomp Elise clash recording (MW_TEST_ELISE_CLASH, evidence/elise-clash/recomp-*.jsonl) with the port's
MW_ELISE_ROWS output, clip by clip and frame by frame (her clips 0206 / 0200 / 0201 / 0204).

python -I tools/compare_elise_clash.py <recomp rows> <port rows>
"""
import json, math, sys
native = [json.loads(l) for l in open(sys.argv[1])]
port = [json.loads(l) for l in open(sys.argv[2])]
# the original's objects stand 0.1 above the floor (feet drawn on it); the port keeps actors on the floor itself
for r in port:
    for who in ('elise', 'player'): r[who]['pos'][1] += 0.1
# native motion resources in order of first appearance -> clip names (the clash order: pull-out, start, struggle, bite)
HER = ['em0630_0206.mot', 'em0630_0200.mot', 'em0630_0201.mot', 'em0630_0204.mot']
HIS = ['pl0030_0466.mot', 'pl0030_0460.mot', 'pl0030_0461.mot', 'pl0030_0464.mot']
seen_e, seen_p = [], []
rows = []
for r in native[1:]:
    e, p = r['elise'], r['player']
    if e['state'] != 16: continue
    if e['motion'] not in seen_e: seen_e.append(e['motion'])
    if p['motion'] not in seen_p: seen_p.append(p['motion'])
    rows.append((HER[seen_e.index(e['motion'])], e['frame'], e['pos'], e['rot'][1], HIS[seen_p.index(p['motion'])] if len(seen_p) <= 4 else '?', p['frame'], p['pos'], p['facing']))
pk = {}
for r in port:
    e, p = r['elise'], r['player']
    pk.setdefault((e['clip'], round(e['frame'])), r)
worst = {}
print(f"{'clip':16} {'f':>5} | native her / his | port her / his | d_her d_his")
for clip, f, epos, eyaw, pclip, pf, ppos, pfac in rows:
    r = pk.get((clip, round(f)))
    if not r: continue
    a, b = r['elise']['pos'], r['player']['pos']
    dh = math.hypot(epos[0] - a[0], epos[2] - a[2]); dp = math.hypot(ppos[0] - b[0], ppos[2] - b[2])
    w = worst.setdefault(clip, [0, 0, 0, 0, 0, 0]); w[0] = max(w[0], dh); w[1] = max(w[1], dp); w[2] += 1
    w[4] = max(w[4], abs(epos[1] - a[1])); w[5] = max(w[5], abs(ppos[1] - b[1]))
    w[3] = max(w[3], abs(((eyaw - r['elise']['yaw'] + math.pi) % (2 * math.pi)) - math.pi))
    if round(f) % 20 == 1:
        print(f"{clip:16} {f:5.0f} | ({epos[0]:6.2f},{epos[2]:6.2f}) ({ppos[0]:6.2f},{ppos[2]:6.2f}) f{pfac:5.2f} | ({a[0]:6.2f},{a[2]:6.2f}) ({b[0]:6.2f},{b[2]:6.2f}) f{r['player']['facing']:5.2f} | {dh:.3f} {dp:.3f}")
print()
for clip, (dh, dp, n, dy, hy, py) in worst.items():
    print(f"{clip}: {n} frames, max her {dh:.3f} m, max his {dp:.3f} m, max her yaw {dy:.3f} rad, height her {hy:.3f} his {py:.3f}")

# best integer lag (native position at frame f == port position at frame f - k) per clip and actor
print()
for actor in ('her', 'his'):
    for clip in HER:
        best = None
        for k in range(0, 5):
            errs = []
            for c, f, epos, eyaw, pclip, pf, ppos, pfac in rows:
                if c != clip: continue
                r = pk.get((clip, round(f) - k))
                if not r: continue
                a = r['elise']['pos'] if actor == 'her' else r['player']['pos']
                n = epos if actor == 'her' else ppos
                errs.append(math.hypot(n[0] - a[0], n[2] - a[2]))
            if errs:
                m = max(errs)
                if best is None or m < best[1]: best = (k, m)
        print(actor, clip, 'best lag', best[0], 'frames, max error', round(best[1], 3))

# strict: the whole clash update by update (both from her 0206 frame 2), positions with the port's feet-on-floor offset
ns = [r for r in native[1:] if r['elise']['state'] == 16]
start = next(i for i, r in enumerate(port) if r['elise']['clip'] == 'em0630_0206.mot' and round(r['elise']['frame']) == 2)
ps = port[start:]
bad, worst = [], [0., 0.]
for k, (a, b) in enumerate(zip(ns, ps)):
    for j, who in enumerate(('elise', 'player')):
        x, y = a[who]['pos'], b[who]['pos']
        d = math.dist(x, y)
        worst[j] = max(worst[j], d)
        if d > 0.01: bad.append((k, who[0], round(d, 3)))
print(f"strict: {min(len(ns), len(ps))} updates (native {len(ns)}, port {len(ps)}); max her {worst[0]:.3f} m, his {worst[1]:.3f} m (3D); "
      f"{len(bad)} over 1 cm: {bad[:10]}")
