"""Bake the town's walk grid [added] (MW_STAGE=town, play_town.rs): where the civilians can walk on c201's streets.

From assets/native/collision/world.json (the shapes' world-space triangles; convex bodies carry their hull):
- floor: per cell, the highest col.hkx surface facing up (ny > 0.55) under 2.2 m, kept at street level (-1 .. 1.6 m);
- walls: col.hkx / col_pl.hkx / col_obj.hkx / col_ef.hkx (the building blocks) surfaces facing sideways, sampled every 0.12 m; a cell is shut when such a point lies within
  the body radius (+ a margin) of its centre between 0.3 and 1.7 m over its floor (stalls, crates, fences, building fronts);
- the walkable cells joined to the street start (8-neighbours, a step of at most 0.4 m) are kept.
Writes assets/native/world/town-nav.json: origin (x, z of cell 0), cell size, width, height and each cell's floor (null
where nobody walks). Run: python -I tools/town/stage_nav.py
"""
from pathlib import Path
import json
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
CELL = 0.5
X0, X1, Z0, Z1 = -48., 48., -48., 58.
RADIUS = 0.42
START = (-2.0, 34.0)


def triangles(world, sources):
    shapes = {s['id']: s for s in world['shapes']}
    out = []
    for b in world['bodies']:
        if b['source'] not in sources: continue
        for sid in b['shapes']:
            s = shapes.get(sid)
            if not s or not s.get('triangles'): continue
            v = np.array(s['vertices'], dtype=np.float64)
            for t in s['triangles']:
                out.append(v[list(t)])
    return np.array(out)


def main():
    world = json.loads((ROOT / 'assets/native/collision/world.json').read_text())
    w, h = int((X1 - X0) / CELL), int((Z1 - Z0) / CELL)
    cx = X0 + (np.arange(w) + 0.5) * CELL
    cz = Z0 + (np.arange(h) + 0.5) * CELL
    floor = np.full((h, w), -np.inf)
    tris = triangles(world, {'col.hkx'})
    n = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)
    for t, nn in zip(tris, n):
        if abs(nn[1]) < 0.55: continue
        lo, hi = t.min(0), t.max(0)
        if lo[1] > 2.2: continue
        i0, i1 = np.searchsorted(cx, lo[0]), np.searchsorted(cx, hi[0])
        j0, j1 = np.searchsorted(cz, lo[2]), np.searchsorted(cz, hi[2])
        if i1 <= i0 or j1 <= j0: continue
        X, Z = np.meshgrid(cx[i0:i1], cz[j0:j1])
        a, b, c = t
        v0, v1 = b - a, c - a
        d00, d01, d11 = v0[0] * v0[0] + v0[2] * v0[2], v0[0] * v1[0] + v0[2] * v1[2], v1[0] * v1[0] + v1[2] * v1[2]
        den = d00 * d11 - d01 * d01
        if abs(den) < 1e-12: continue
        px, pz = X - a[0], Z - a[2]
        d20, d21 = px * v0[0] + pz * v0[2], px * v1[0] + pz * v1[2]
        v = (d11 * d20 - d01 * d21) / den
        u = (d00 * d21 - d01 * d20) / den
        inside = (u >= -1e-6) & (v >= -1e-6) & (u + v <= 1 + 1e-6)
        y = a[1] + v * v0[1] + u * v1[1]
        y = np.where(inside & (y <= 2.2), y, -np.inf)
        sub = floor[j0:j1, i0:i1]
        floor[j0:j1, i0:i1] = np.maximum(sub, y)
    walk = (floor > -1.0) & (floor < 1.6)
    # walls: sideways surfaces, sampled
    wt = triangles(world, {'col.hkx', 'col_pl.hkx', 'col_obj.hkx', 'col_ef.hkx'})
    wn = np.cross(wt[:, 1] - wt[:, 0], wt[:, 2] - wt[:, 0])
    wn /= np.maximum(np.linalg.norm(wn, axis=1, keepdims=True), 1e-12)
    pts = []
    for t, nn in zip(wt, wn):
        if abs(nn[1]) > 0.7: continue
        if t[:, 1].max() < -1. or t[:, 1].min() > 3.4: continue
        e = max(np.linalg.norm(t[1] - t[0]), np.linalg.norm(t[2] - t[0]), np.linalg.norm(t[2] - t[1]))
        k = max(2, int(e / 0.12) + 1)
        s = np.linspace(0, 1, k)
        U, V = np.meshgrid(s, s)
        m = (U + V) <= 1
        U, V = U[m], V[m]
        pts.append(t[0] + U[:, None] * (t[1] - t[0]) + V[:, None] * (t[2] - t[0]))
    pts = np.concatenate(pts)
    blocked = np.zeros_like(walk)
    r = int(np.ceil(RADIUS / CELL)) + 1
    pi = np.floor((pts[:, 0] - X0) / CELL).astype(int)
    pj = np.floor((pts[:, 2] - Z0) / CELL).astype(int)
    for di in range(-r, r + 1):
        for dj in range(-r, r + 1):
            i, j = pi + di, pj + dj
            ok = (i >= 0) & (i < w) & (j >= 0) & (j < h)
            i, j, p = i[ok], j[ok], pts[ok]
            d = np.hypot(cx[i] - p[:, 0], cz[j] - p[:, 2])
            f = floor[j, i]
            hit = (d < RADIUS) & np.isfinite(f) & (p[:, 1] > f + 0.3) & (p[:, 1] < f + 1.7)
            blocked[j[hit], i[hit]] = True
    walk &= ~blocked
    # the part joined to the start
    si, sj = int((START[0] - X0) / CELL), int((START[1] - Z0) / CELL)
    if not walk[sj, si]:
        ys, xs = np.nonzero(walk)
        k = np.argmin((xs - si) ** 2 + (ys - sj) ** 2); si, sj = xs[k], ys[k]
    seen = np.zeros_like(walk); stack = [(sj, si)]; seen[sj, si] = True
    while stack:
        j, i = stack.pop()
        for dj in (-1, 0, 1):
            for di in (-1, 0, 1):
                a, b = j + dj, i + di
                if (dj or di) and 0 <= a < h and 0 <= b < w and walk[a, b] and not seen[a, b] and abs(floor[a, b] - floor[j, i]) < 0.4:
                    seen[a, b] = True; stack.append((a, b))
    cells = np.where(seen, np.round(floor, 3), np.nan)
    out = dict(schema='mw-town-nav/1', origin=[X0, Z0], cell=CELL, width=w, height=h, radius=RADIUS,
               floor=[None if np.isnan(v) else float(v) for v in cells.reshape(-1)],
               authority='added: baked from the staged c201 collision for the town civilians (not native navigation)')
    (ROOT / 'assets/native/world/town-nav.json').write_text(json.dumps(out, separators=(',', ':')))
    print(dict(cells=int(seen.sum()), area_m2=float(seen.sum() * CELL * CELL), grid=(w, h), walls=len(pts)))
    try:
        from PIL import Image
        img = np.zeros((h, w, 3), np.uint8)
        img[walk & ~seen] = (90, 60, 60); img[seen] = (200, 220, 200); img[blocked] = (40, 40, 160)
        Image.fromarray(img[::-1]).resize((w * 3, h * 3), Image.NEAREST).save(ROOT / 'research/town-nav.png')
    except Exception as e:
        print('no preview', e)


if __name__ == '__main__':
    main()
