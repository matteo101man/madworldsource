"""Bullet wound decals for mw-play (added art, not native): assets/native/effects/wound-entry.png, a small dark hole
with a torn red rim, and wound-exit.png, a larger ragged blow-out with spatter. Projected onto bodies where a pistol
round goes in and comes out (play_decal.rs, DecalRequest::at), and claw-marks.png, three torn parallel slashes left
where a clawed kung-fu grunt's hit lands on the player; bite-marks.png, two fang punctures in a bruise with drips,
left on the player's neck by Elise's bite, and mouth-blood.png, a smear with drips down the chin, on her mouth after she
drinks. Deterministic (seeded). Output stays local.
Usage: python -I tools/stage_wound_decals.py
"""
from pathlib import Path
import numpy as np
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/native/effects'
N = 128


def wound(radius, rag, spatter, seed):
    rng = np.random.default_rng(seed)
    y, x = np.mgrid[0:N, 0:N].astype(float)
    cx = cy = (N - 1) / 2
    ang = np.arctan2(y - cy, x - cx); r = np.hypot(x - cx, y - cy) / (N / 2)
    # a ragged edge: a few low harmonics of random phase
    edge = radius * (1 + sum(rng.uniform(0, rag) * np.cos(k * ang + rng.uniform(0, 6.3)) for k in range(2, 9)))
    t = r / edge
    rgba = np.zeros((N, N, 4))
    core = np.clip(1 - t / 0.45, 0, 1)                        # the hole: near black
    rim = np.clip(1 - np.abs(t - 0.75) / 0.35, 0, 1)          # torn flesh around it
    col = np.array([92., 8., 10.]) * (1 - core[..., None]) + np.array([18., 2., 3.]) * core[..., None]
    col = col * (0.75 + 0.35 * rim[..., None])
    alpha = np.clip((1.15 - t) / 0.25, 0, 1)
    # spatter flecks beyond the edge
    for _ in range(spatter):
        a, d, s = rng.uniform(0, 6.3), rng.uniform(1.05, 1.9) * radius * N / 2, rng.uniform(1.2, 4.) * radius * 4
        px, py = cx + np.cos(a) * d, cy + np.sin(a) * d
        dot = np.clip(1 - np.hypot(x - px, y - py) / s, 0, 1) ** 0.6
        alpha = np.maximum(alpha, dot); col = np.where(dot[..., None] > 0.05, np.array([110., 10., 12.]), col)
    rgba[..., :3] = col; rgba[..., 3] = alpha * 255
    im = Image.fromarray(rgba.clip(0, 255).astype(np.uint8), 'RGBA')
    return im.filter(ImageFilter.GaussianBlur(0.6))


def claws(seed):
    """Three parallel diagonal gashes, tapered at both ends, dark in the middle, with a little spatter."""
    rng = np.random.default_rng(seed)
    y, x = np.mgrid[0:N, 0:N].astype(float) / (N - 1)
    alpha = np.zeros((N, N)); core = np.zeros((N, N))
    for k, off in enumerate((-0.22, 0., 0.22)):
        # along u (the slash), across v; tilted 35 degrees
        a = np.radians(35.)
        u = (x - 0.5) * np.cos(a) + (y - 0.5) * np.sin(a)
        v = -(x - 0.5) * np.sin(a) + (y - 0.5) * np.cos(a) - off + 0.02 * np.sin(u * 9 + k)
        length = 0.42 - 0.06 * abs(k - 1)
        taper = np.clip(1 - (u / length) ** 2, 0, 1) ** 0.6
        width = 0.035 * taper + 1e-4
        g = np.clip(1 - np.abs(v) / width, 0, 1)
        alpha = np.maximum(alpha, g ** 0.7); core = np.maximum(core, np.clip(1 - np.abs(v) / (width * 0.45), 0, 1))
    for _ in range(10):
        px, py, r = rng.uniform(0.15, 0.85), rng.uniform(0.15, 0.85), rng.uniform(0.008, 0.02)
        alpha = np.maximum(alpha, np.clip(1 - np.hypot(x - px, y - py) / r, 0, 1))
    col = np.array([128., 10., 12.]) * (1 - core[..., None]) + np.array([30., 2., 4.]) * core[..., None]
    rgba = np.dstack([col, alpha * 255]).clip(0, 255).astype(np.uint8)
    return Image.fromarray(rgba, 'RGBA').filter(ImageFilter.GaussianBlur(0.5))


def bite(seed):
    """Two fang punctures (dark holes, red rims) in a faint bruise, each with a drip running down (image -v)."""
    rng = np.random.default_rng(seed)
    y, x = np.mgrid[0:N, 0:N].astype(float) / (N - 1)
    bruise = np.clip(1 - np.hypot((x - 0.5) / 0.34, (y - 0.42) / 0.22), 0, 1) ** 0.8
    alpha = bruise * 0.45; col = np.zeros((N, N, 3)) + np.array([88., 18., 30.])
    core = np.zeros((N, N))
    for k, px in enumerate((0.36, 0.64)):
        py = 0.4 + rng.uniform(-0.02, 0.02)
        d = np.hypot(x - px, y - py)
        rim = np.clip(1 - d / 0.07, 0, 1); hole = np.clip(1 - d / 0.035, 0, 1)
        # the drip: a thin tapering streak down from the hole, a bead at its end
        ln = rng.uniform(0.25, 0.4)
        u = (y - py) / ln; wob = px + 0.01 * np.sin(u * 7 + k)
        streak = np.clip(1 - np.abs(x - wob) / (0.016 * (1 - 0.6 * u.clip(0, 1)) + 1e-4), 0, 1) * ((u > 0) & (u < 1))
        bead = np.clip(1 - np.hypot(x - wob, y - (py + ln)) / 0.022, 0, 1)
        alpha = np.maximum.reduce([alpha, rim, streak, bead]); core = np.maximum(core, hole)
    col = col * (1 - core[..., None]) + np.array([20., 2., 4.]) * core[..., None]
    col = np.where((alpha > 0.5)[..., None] & (core < 0.2)[..., None], np.array([130., 8., 12.]), col)
    rgba = np.dstack([col, alpha * 255]).clip(0, 255).astype(np.uint8)
    return Image.fromarray(rgba, 'RGBA').filter(ImageFilter.GaussianBlur(0.6))


def mouth(seed):
    """A smear across the lips (wide, ragged) with three drips down the chin (image -v)."""
    rng = np.random.default_rng(seed)
    y, x = np.mgrid[0:N, 0:N].astype(float) / (N - 1)
    a = np.arctan2(y - 0.4, x - 0.5)
    edge = 1 + sum(rng.uniform(0, 0.12) * np.cos(k * a + rng.uniform(0, 6.3)) for k in range(2, 7))
    smear = np.clip((1.05 - np.hypot((x - 0.5) / 0.36, (y - 0.4) / 0.1) / edge) / 0.25, 0, 1)
    alpha = smear.copy()
    for k in range(3):
        px = 0.5 + (k - 1) * rng.uniform(0.1, 0.16); ln = rng.uniform(0.2, 0.42)
        u = (y - 0.42) / ln; wob = px + 0.008 * np.sin(u * 8 + k)
        streak = np.clip(1 - np.abs(x - wob) / (0.02 * (1 - 0.5 * u.clip(0, 1)) + 1e-4), 0, 1) * ((u > 0) & (u < 1))
        bead = np.clip(1 - np.hypot(x - wob, y - (0.42 + ln)) / 0.026, 0, 1)
        alpha = np.maximum.reduce([alpha, streak, bead])
    col = np.zeros((N, N, 3)) + np.array([140., 6., 12.])
    col = col * (0.7 + 0.3 * smear[..., None])
    rgba = np.dstack([col, alpha * 235]).clip(0, 255).astype(np.uint8)
    return Image.fromarray(rgba, 'RGBA').filter(ImageFilter.GaussianBlur(0.7))


OUT.mkdir(parents=True, exist_ok=True)
bite(3).save(OUT / 'bite-marks.png')
mouth(9).save(OUT / 'mouth-blood.png')
wound(0.28, 0.06, 4, 7).save(OUT / 'wound-entry.png')
wound(0.5, 0.16, 14, 11).save(OUT / 'wound-exit.png')
claws(5).save(OUT / 'claw-marks.png')
print('wrote', OUT / 'wound-entry.png', OUT / 'wound-exit.png', OUT / 'claw-marks.png', OUT / 'bite-marks.png', OUT / 'mouth-blood.png')
