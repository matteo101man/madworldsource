"""Explorer-compatible forward kinematics for decoded MOT tracks (three.js conventions of exports/madworld/app.js).

Local bone transform = T(position) * Rz(z) * Ry(y) * Rx(x) (Euler order 'ZYX'). Position channels 0-2 override the
bind `relative` offset; rotation channels 3-5 are x/y/z. Track bone -1 is the actor root (translation only), the
parent of every bone whose `parent` is -1. Bone `parent` values are array indices.
"""
import math
import numpy as np


def rx(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])


def ry(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def rz(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])


def euler_matrix(x, y, z):
    return rz(z) @ ry(y) @ rx(x)


def matrix_euler(m, near=None):
    """ZYX euler of a rotation matrix; with `near`, the equivalent triple closest to it (unwrapped)."""
    sy = -max(-1.0, min(1.0, m[2, 0]))
    y = math.asin(sy)
    if abs(m[2, 0]) < 0.9999999:
        x, z = math.atan2(m[2, 1], m[2, 2]), math.atan2(m[1, 0], m[0, 0])
    else:
        x, z = 0.0, math.atan2(-m[0, 1], m[1, 1])
    if near is None:
        return (x, y, z)
    best = None
    for cand in ((x, y, z), (x + math.pi, math.pi - y, z + math.pi)):
        c = [v + round((n - v) / (2 * math.pi)) * 2 * math.pi for v, n in zip(cand, near)]
        d = sum((a - b) ** 2 for a, b in zip(c, near))
        if best is None or d < best[0]:
            best = (d, tuple(c))
    return best[1]


def axis_angle(axis, a):
    axis = np.asarray(axis, float)
    axis = axis / np.linalg.norm(axis)
    k = np.array([[0, -axis[2], axis[1]], [axis[2], 0, -axis[0]], [-axis[1], axis[0], 0]])
    return np.eye(3) + math.sin(a) * k + (1 - math.cos(a)) * (k @ k)


def align(u, v):
    """Minimal rotation taking direction u to direction v."""
    u = np.asarray(u, float) / np.linalg.norm(u)
    v = np.asarray(v, float) / np.linalg.norm(v)
    c = float(np.dot(u, v))
    axis = np.cross(u, v)
    s = float(np.linalg.norm(axis))
    if s < 1e-9:
        if c > 0:
            return np.eye(3)
        perp = np.cross(u, [1, 0, 0] if abs(u[0]) < 0.9 else [0, 1, 0])
        return axis_angle(perp, math.pi)
    return axis_angle(axis / s, math.atan2(s, c))


class Pose:
    """One frame: local rot/pos per bone index, with world transforms on demand."""

    def __init__(self, bones, local):
        self.bones = bones                 # model bones (id, parent index, relative)
        self.local = local                 # {(bone_id, channel): value}
        self.index = {b['id']: i for i, b in enumerate(bones)}
        self.refresh()

    def refresh(self):
        n = len(self.bones)
        self.wr = [None] * n
        self.wp = [None] * n
        root = np.array([self.local.get((-1, c), 0.0) for c in range(3)])
        for i, b in enumerate(self.bones):
            bid = b['id']
            pos = np.array([self.local.get((bid, c), b['relative'][c]) for c in range(3)])
            rot = euler_matrix(*(self.local.get((bid, c), 0.0) for c in (3, 4, 5)))
            p = b['parent']
            if p is None or p < 0:
                self.wr[i], self.wp[i] = rot, root + pos
            else:
                self.wr[i], self.wp[i] = self.wr[p] @ rot, self.wp[p] + self.wr[p] @ pos

    def world(self, bid):
        i = self.index[bid]
        return self.wp[i], self.wr[i]

    def set_world_rot(self, bid, wrot):
        i = self.index[bid]
        p = self.bones[i]['parent']
        parent = np.eye(3) if p is None or p < 0 else self.wr[p]
        near = tuple(self.local.get((bid, c), 0.0) for c in (3, 4, 5))
        x, y, z = matrix_euler(parent.T @ wrot, near)
        self.local[(bid, 3)], self.local[(bid, 4)], self.local[(bid, 5)] = x, y, z
        self.refresh()

    def rotate_world(self, bid, delta):
        self.set_world_rot(bid, delta @ self.world(bid)[1])


def two_bone_ik(pose, upper, lower, end, target, pole):
    """Swing `upper`/`lower` (minimal rotations) so `end` reaches `target` (clamped to reach), elbow toward `pole`."""
    s, _ = pose.world(upper)
    e, _ = pose.world(lower)
    h, _ = pose.world(end)
    la, lb = np.linalg.norm(e - s), np.linalg.norm(h - e)
    d = np.asarray(target) - s
    dist = min(np.linalg.norm(d), (la + lb) * 0.999)
    dn = d / np.linalg.norm(d)
    cos_a = (la * la + dist * dist - lb * lb) / (2 * la * dist)
    a = math.acos(max(-1.0, min(1.0, cos_a)))
    pv = np.asarray(pole) - s
    pv = pv - dn * np.dot(pv, dn)
    pv = pv / (np.linalg.norm(pv) + 1e-9)
    elbow = s + la * (math.cos(a) * dn + math.sin(a) * pv)
    pose.rotate_world(upper, align(e - s, elbow - s))
    e, _ = pose.world(lower)
    h, _ = pose.world(end)
    pose.rotate_world(lower, align(h - e, s + dn * dist - e))
