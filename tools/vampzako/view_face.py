"""Textured close-ups of a built vampire zako's head (bind pose): front and three-quarter, with the concept sheet's face
beside them. Z-buffered per-pixel rasteriser, bilinear texture lookup.

python -I tools/vampzako/view_face.py N [--texture source|color] [--out PATH]
"""
import argparse
from pathlib import Path
import numpy as np
from PIL import Image

PORT = Path(__file__).resolve().parents[2]
CONCEPT = {1: 13, 2: 25, 3: 26}


def raster(P, UV, T, tex, size, yaw):
    c, s = np.cos(yaw), np.sin(yaw)
    R = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]]); Q = P @ R.T
    used = np.unique(T); lo, hi = Q[used, :2].min(0), Q[used, :2].max(0); sc = (size - 20) / (hi - lo).max()
    X = (Q[:, 0] - (lo[0] + hi[0]) / 2) * sc + size / 2; Y = size / 2 - (Q[:, 1] - (lo[1] + hi[1]) / 2) * sc; Z = Q[:, 2]
    img = np.full((size, size, 3), 30, np.float32); zb = np.full((size, size), -1e9)
    th, tw = tex.shape[:2]
    for t in T:
        x, y, z = X[t], Y[t], Z[t]
        x0, x1 = int(max(x.min(), 0)), int(min(x.max() + 1, size - 1)); y0, y1 = int(max(y.min(), 0)), int(min(y.max() + 1, size - 1))
        if x1 < x0 or y1 < y0: continue
        gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
        d = (y[1] - y[2]) * (x[0] - x[2]) + (x[2] - x[1]) * (y[0] - y[2])
        if abs(d) < 1e-9: continue
        l0 = ((y[1] - y[2]) * (gx - x[2]) + (x[2] - x[1]) * (gy - y[2])) / d
        l1 = ((y[2] - y[0]) * (gx - x[2]) + (x[0] - x[2]) * (gy - y[2])) / d; l2 = 1 - l0 - l1
        m = (l0 >= 0) & (l1 >= 0) & (l2 >= 0)
        if not m.any(): continue
        zz = l0 * z[0] + l1 * z[1] + l2 * z[2]
        yy, xx = (gy[m] - 0.5).astype(int), (gx[m] - 0.5).astype(int); zz = zz[m]
        w = zz > zb[yy, xx]
        if not w.any(): continue
        u = (l0[m] * UV[t[0], 0] + l1[m] * UV[t[1], 0] + l2[m] * UV[t[2], 0])[w]
        v = (l0[m] * UV[t[0], 1] + l1[m] * UV[t[1], 1] + l2[m] * UV[t[2], 1])[w]
        px = np.clip(u * tw - 0.5, 0, tw - 1.001); py = np.clip(v * th - 0.5, 0, th - 1.001)
        ix, iy = px.astype(int), py.astype(int); fx, fy = (px - ix)[:, None], (py - iy)[:, None]
        col = tex[iy, ix] * (1 - fx) * (1 - fy) + tex[iy, ix + 1] * fx * (1 - fy) + tex[iy + 1, ix] * (1 - fx) * fy + tex[iy + 1, ix + 1] * fx * fy
        yy, xx = yy[w], xx[w]; zb[yy, xx] = zz[w]; img[yy, xx] = col
    return Image.fromarray(img.clip(0, 255).astype(np.uint8))


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('n', type=int); ap.add_argument('--texture', default='color'); ap.add_argument('--out')
    a = ap.parse_args()
    w = PORT / f'assets/characters/vampzako/build/work{a.n}'
    z = np.load(w / 'skin.npz'); raw = np.load(w / 'raw.npz')
    S = z['S']; src, uv, Ts = raw['source_vertex'], raw['uvs'], raw['indices']
    P = S[src]; top = P[:, 1].max()
    head = (P[:, 1] > top - 0.27)
    T = Ts[head[Ts].all(1)]
    tex = np.asarray(Image.open(PORT / f'assets/characters/vampzako/build/vampzako{a.n}_{a.texture}.png').convert('RGB'), np.float32)
    views = [raster(P, uv, T, tex, 520, yaw) for yaw in (0., 0.6)]
    c = Image.open(f'C:/Users/Matteo/Desktop/newmadworldchars/vampire_purple_000{CONCEPT[a.n]:02d}_.png').convert('RGB')
    cw = c.size[0]; face = c.crop((cw // 2 - 70, 0, cw // 2 + 70, 140)).resize((520, 520), Image.LANCZOS)
    out = Image.new('RGB', (1560, 520)); [out.paste(im, (520 * i, 0)) for i, im in enumerate(views + [face])]
    path = a.out or str(PORT / f'assets/characters/vampzako/build/face{a.n}.png'); out.save(path); print(path)


if __name__ == '__main__':
    main()
