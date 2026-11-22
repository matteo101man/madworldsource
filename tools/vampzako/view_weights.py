"""Weight maps of a built vampire zako: front, back and side views, each vertex coloured by its dominant bone (and,
with --bone ID, that bone's weight as a heat map). Flat-shaded triangles painted back to front.

python -I tools/vampzako/view_weights.py N [--bone ID ...] [--out PATH]
"""
import argparse, colorsys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw

PORT = Path(__file__).resolve().parents[2]


def colour(bid):
    h = (bid * 0.6180339887) % 1.0
    r, g, b = colorsys.hsv_to_rgb(h, 0.75, 0.95); return (int(r * 255), int(g * 255), int(b * 255))


def view(S, T, cols, axis, flip, size=520):
    # axis: 2 front (looking down -z), 0 side; flip mirrors to look from the other side
    h = S[:, 1]; w = S[:, 0] if axis == 2 else S[:, 2]
    if flip: w = -w
    depth = S[:, axis] * (-1 if flip else 1)
    used = np.unique(T); lo, hi = h[used].min(), h[used].max()
    sc = (size - 20) / max(hi - lo, np.ptp(w[used]))
    w = w - (w[used].max() + w[used].min()) / 2
    img = Image.new('RGB', (size, size), (24, 24, 30)); d = ImageDraw.Draw(img)
    order = np.argsort(depth[T].mean(1))
    for t in order:
        pts = [(size / 2 + w[v] * sc, size - 10 - (h[v] - lo) * sc) for v in T[t]]
        c = np.mean([cols[v] for v in T[t]], 0).astype(int)
        d.polygon(pts, fill=tuple(c))
    return img


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('n', type=int); ap.add_argument('--bone', type=int, nargs='*', default=[])
    ap.add_argument('--out'); a = ap.parse_args()
    z = np.load(PORT / f'assets/characters/vampzako/build/work{a.n}/skin.npz')
    S, T, J, W = z['S'], z['T'], z['joints'], z['weights']
    rows = []
    dom = J[np.arange(len(J)), np.argmax(W, 1)]
    cols = np.array([colour(b) for b in dom])
    rows.append([view(S, T, cols, 2, False), view(S, T, cols, 2, True), view(S, T, cols, 0, False)])
    for b in a.bone:
        w = (W * (J == b)).sum(1)
        hc = np.stack([w * 255, w * 120, (1 - w) * 90], 1)
        rows.append([view(S, T, hc, 2, False), view(S, T, hc, 2, True), view(S, T, hc, 0, False)])
    out = Image.new('RGB', (520 * 3, 520 * len(rows)))
    for r, row in enumerate(rows):
        for c, im in enumerate(row): out.paste(im, (520 * c, 520 * r))
    path = a.out or str(PORT / f'assets/characters/vampzako/build/weights{a.n}.png'); out.save(path); print(path)


if __name__ == '__main__':
    main()
