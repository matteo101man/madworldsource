"""Skin tones for the c503 colour actors (added art, not the pack).

python -I tools/stage_boss_skin.py   (after tools/stage_actor.py has written <model>-color.glb)
The explorer's colour export gives each mesh one guessed vertex colour, and the skin meshes of the Black Baron (em0623),
the oirans (em0960) and Elise (em0630) came out lavender / violet / mauve, which the lit modern look turns purple. The meshes were identified
by their UV footprints on the atlas (torso, arms, fingers, face; the oirans' face and neck / arms). Their non-black
vertex colours (the ink stays black) are rewritten in place: a dark brown for the Baron, a pale tone for the oirans.
Idempotent; only COLOR_0 bytes change.
"""
import json, struct
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
# (model, mesh indices, skin RGB, only vertices of this colour (None: every non-ink vertex))
SKIN = [
    ('baron', [2, 6, 7, 10, 13, 14], (0.36, 0.22, 0.14), None),
    ('oiran', [5, 12], (0.97, 0.88, 0.82), None),
    # Elise: her hands (0, 1), face (6) and bare chest / arms (15) carry a mauve (0.625, 0.375, 0.625) that the lit look
    # turns red; a vampire's pale skin [the mixed meshes keep their other colours]
    ('elise', [0, 1, 6, 15], (0.93, 0.87, 0.86), (0.625, 0.375, 0.625)),
    # her torso and thighs (15: the white thorn-tattooed skin on the atlas, the corset front and the bare thighs above
    # the boots) were guessed a dark crimson (0.184, 0.034, 0.076): all of it is skin
    ('elise', [15], (0.93, 0.87, 0.86), (0.184, 0.034, 0.076)),
]
TYPES = {5126: (np.float32, 1.), 5121: (np.uint8, 255.), 5123: (np.uint16, 65535.)}


def main():
    for model, meshes, rgb, only in SKIN:
        path = ROOT / f'assets/native/{model}-color.glb'
        glb = bytearray(path.read_bytes())
        jl = struct.unpack_from('<I', glb, 12)[0]
        doc = json.loads(glb[20:20 + jl]); base = 20 + jl + 8
        changed = 0
        for i in meshes:
            for pr in doc['meshes'][i]['primitives']:
                a = doc['accessors'][pr['attributes']['COLOR_0']]; v = doc['bufferViews'][a['bufferView']]
                n = {'VEC3': 3, 'VEC4': 4}[a['type']]
                dt, scale = TYPES[a['componentType']]; size = np.dtype(dt).itemsize
                stride = v.get('byteStride', 0) or size * n
                o = base + v.get('byteOffset', 0) + a.get('byteOffset', 0)
                for k in range(a['count']):
                    at = o + k * stride
                    c = np.frombuffer(bytes(glb[at:at + size * 3]), dtype=dt).astype(float) / scale
                    if only is None and c.max() < 0.2: continue   # the ink
                    if only is not None and np.abs(c - np.array(only)).max() > 0.02: continue   # (a named colour, ink or not)
                    glb[at:at + size * 3] = (np.array(rgb) * scale).astype(dt).tobytes()
                    changed += 1
        path.write_bytes(bytes(glb))
        print(model, 'skin vertices', changed)


if __name__ == '__main__':
    main()
