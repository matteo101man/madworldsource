"""Finger chain detection on Kreely's T-pose hands (palm down, fingers along +-x, thumb toward +z).

All inputs/outputs are in the same Y-up space as the mesh passed in. Thresholds are fractions of hand length
(wrist -> fingertip), so the same code works on the raw FBX scale and on the scaled build mesh.
"""
import numpy as np

FINGERS = ['index', 'middle', 'ring', 'pinky']


def plane_points(P, T, axis, value):
    """Points where the plane P[:, axis] == value cuts the triangle surface."""
    tri = P[T]; d = tri[:, :, axis] - value; out = []
    for a, b in ((0, 1), (1, 2), (2, 0)):
        m = d[:, a] * d[:, b] < 0; t = d[m, a] / (d[m, a] - d[m, b])
        out.append(tri[m, a] + (tri[m, b] - tri[m, a]) * t[:, None])
    return np.concatenate(out)


def split_clusters(z, n, min_gap):
    """Split sorted 1-D values at the n-1 largest gaps (each >= min_gap); None if not separable."""
    z = np.sort(z)
    if len(z) < 2 * n: return None
    gaps = np.diff(z); cut = np.sort(np.argsort(gaps)[-(n - 1):])
    if gaps[cut].min() < min_gap: return None
    return [g for g in np.split(z, cut + 1)]


def detect(P, T, side, wrist_x):
    """Return dict finger -> joints (4,3: base, mid, distal, tip) and 'palmDown'."""
    x = P[:, 0] * side
    hand = x > wrist_x
    tip = x[hand].max(); L = tip - wrist_x
    # first x (outward) where a slice shows the four fingers separated -> web line between the fingers
    web = None
    for f in np.linspace(0.30, 0.80, 101):
        q = plane_points(P, T, 0, side * (wrist_x + f * L))
        q = q[q[:, 2] < np.percentile(q[:, 2], 100)]  # all
        thumbless = q[q[:, 2] < q[:, 2].min() + 0.42 * L]  # fingers occupy the back ~0.4L of z at this depth
        cl = split_clusters(thumbless[:, 2], 4, 0.012 * L)
        if cl is not None and all(len(c) >= 4 for c in cl):
            web = wrist_x + f * L; break
    if web is None: raise RuntimeError('fingers not separable')
    # finger verts past the web line; the thumb is split off at the widest z gap, the rest 1-D k-means into 4
    W = P[hand & (x > web + 0.12 * L)]
    z = np.sort(W[:, 2]); g = np.diff(z); cut = z[np.argmax(g)] + g.max() / 2
    if (z > cut).sum() > (z < cut).sum(): raise RuntimeError('thumb split failed')
    V = W[W[:, 2] < cut]
    centers = np.quantile(V[:, 2], [0.875, 0.625, 0.375, 0.125])
    for _ in range(50):
        lab = np.argmin(np.abs(V[:, 2][:, None] - centers[None]), 1)
        centers = np.array([V[lab == k, 2].mean() for k in range(4)])
    order = np.argsort(-centers); lab = np.argsort(order)[lab]; centers = centers[order]   # +z first: index ... pinky
    spacing = np.mean(-np.diff(centers))
    out = {}
    for k, name in enumerate(FINGERS):
        F = V[lab == k]
        c = F.mean(0); u = np.linalg.svd(F - c)[2][0]; u = u * np.sign(u[0] * side)
        s = (F - c) @ u
        tipp = c + u * s.max()
        base = c + u * ((web - 0.13 * L - c[0] * side) / (u[0] * side))   # knuckle inside the palm, before the web
        Lf = np.linalg.norm(tipp - base)
        out[name] = np.array([base, base + u * 0.45 * Lf, base + u * 0.75 * Lf, tipp])
    # thumb: hand verts clearly in front of the index finger band (+z), outside the finger web
    Th = P[hand & (P[:, 2] > cut)]
    far = Th[np.argmax(Th[:, 2])]                                   # thumb tip: most forward point
    c = Th.mean(0); u = np.linalg.svd(Th - c)[2][0]
    u = u * np.sign((far - c) @ u)
    s = (Th - c) @ u
    tipp = c + u * s.max()
    base = c + u * (s.min() - 0.25 * (s.max() - s.min()))           # CMC joint sits inside the palm
    Lt = np.linalg.norm(tipp - base)
    out['thumb'] = np.array([base, base + u * 0.38 * Lt, base + u * 0.70 * Lt, tipp])
    # palm-down normal from the finger fan: finger direction x (index -> pinky), oriented to -y
    fdir = np.mean([out[f][3] - out[f][0] for f in FINGERS], 0); fdir /= np.linalg.norm(fdir)
    across = out['pinky'][0] - out['index'][0]
    n = np.cross(fdir, across); n /= np.linalg.norm(n); n = n * -np.sign(n[1])
    out['palmDown'] = n
    out['web'] = web; out['length'] = L
    return out
