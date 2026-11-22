"""Per-pixel orthographic front render (z-buffer, texture sampled through interpolated UVs) of Kreely's bind mesh.

python -I tools/kreely/ortho_texture_render.py <out.png> <texture> <x0> <x1> <y0> <y1> [width] [--grid G]
Coordinates are fractions of his height (y up, x to his left), the same units as the face landmarks in kreely.build.json.
"""
import argparse, json
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw

BUILD = Path(__file__).resolve().parents[2] / 'assets/characters/kreely/build'


def render(tex_path, x0, x1, y0, y1, W, model=None):
    model = model or json.loads((BUILD / 'kreely.model.json').read_text(encoding='utf-8'))
    m = model['meshes'][0]; P = np.array(m['positions']); top = P[:, 1].max(); P = P / top
    I = np.array(m['indices']).reshape(-1, 3); uv = np.array(m['uvs'])
    tex = np.asarray(Image.open(tex_path).convert('RGB')); th, tw, _ = tex.shape
    H = int(round(W * (y1 - y0) / (x1 - x0)))
    img = np.zeros((H, W, 3), np.uint8); img[:] = (30, 60, 30); zb = np.full((H, W), -1e9)
    sx = lambda x: (x - x0) / (x1 - x0) * W; sy = lambda y: (y1 - y) / (y1 - y0) * H
    for t in I:
        p = P[t]
        if p[:, 0].max() < x0 or p[:, 0].min() > x1 or p[:, 1].max() < y0 or p[:, 1].min() > y1: continue
        a, b, c = np.c_[sx(p[:, 0]), sy(p[:, 1])]
        d = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(d) < 1e-12: continue
        lo = np.floor(np.minimum(np.minimum(a, b), c)).astype(int); hi = np.ceil(np.maximum(np.maximum(a, b), c)).astype(int)
        lo = np.maximum(lo, 0); hi = np.minimum(hi, [W - 1, H - 1])
        if (hi < lo).any(): continue
        gx, gy = np.meshgrid(np.arange(lo[0], hi[0] + 1) + .5, np.arange(lo[1], hi[1] + 1) + .5)
        l1 = ((gx - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (gy - a[1])) / d
        l2 = ((b[0] - a[0]) * (gy - a[1]) - (gx - a[0]) * (b[1] - a[1])) / d; l0 = 1 - l1 - l2
        ins = (l0 >= 0) & (l1 >= 0) & (l2 >= 0)
        if not ins.any(): continue
        z = l0 * p[0, 2] + l1 * p[1, 2] + l2 * p[2, 2]
        yy, xx = gy[ins].astype(int), gx[ins].astype(int); z = z[ins]
        win = z > zb[yy, xx]
        if not win.any(): continue
        yy, xx, z = yy[win], xx[win], z[win]
        u = (l0[ins][win] * uv[t[0], 0] + l1[ins][win] * uv[t[1], 0] + l2[ins][win] * uv[t[2], 0])
        v = (l0[ins][win] * uv[t[0], 1] + l1[ins][win] * uv[t[1], 1] + l2[ins][win] * uv[t[2], 1])
        zb[yy, xx] = z
        img[yy, xx] = tex[(np.clip(v, 0, 1) * (th - 1)).astype(int), (np.clip(u, 0, 1) * (tw - 1)).astype(int)]
    return Image.fromarray(img), (sx, sy)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('out'); ap.add_argument('texture')
    for k in ('x0', 'x1', 'y0', 'y1'): ap.add_argument(k, type=float)
    ap.add_argument('width', type=int, nargs='?', default=1000); ap.add_argument('--grid', type=float, default=0)
    a = ap.parse_args()
    img, (sx, sy) = render(a.texture, a.x0, a.x1, a.y0, a.y1, a.width)
    if a.grid:
        dr = ImageDraw.Draw(img)
        for g in np.arange(np.ceil(a.y0 / a.grid) * a.grid, a.y1, a.grid):
            dr.line([(0, sy(g)), (8, sy(g))], fill=(255, 255, 0)); dr.text((10, sy(g) - 6), f'{g:.4f}', fill=(255, 255, 0))
        for g in np.arange(np.ceil(a.x0 / a.grid) * a.grid, a.x1, a.grid):
            dr.line([(sx(g), img.height - 8), (sx(g), img.height)], fill=(0, 255, 255)); dr.text((sx(g) - 12, img.height - 22), f'{g:.4f}', fill=(0, 255, 255))
    img.save(a.out)


if __name__ == '__main__':
    main()
