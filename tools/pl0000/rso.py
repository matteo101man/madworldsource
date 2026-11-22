"""Read constants and strings from the linked enemy RSO (tools/enemy-analysis.bin, layout research/enemy-link.json).

python -I tools/pl0000/rso.py f:8114fe24 s:811501b9 ...   (f = f32, d = f64, u = u32, s = C string)
"""
import json, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class Rso:
    def __init__(self):
        self.b = (ROOT / 'tools/enemy-analysis.bin').read_bytes()
        self.secs = [s for s in json.loads((ROOT / 'research/enemy-link.json').read_text())['sections'] if not s['zero']]

    def off(self, a):
        for s in self.secs:
            if s['address'] <= a < s['address'] + s['size']: return s['offset'] + a - s['address']
        raise KeyError(hex(a))

    def f32(self, a): return struct.unpack_from('>f', self.b, self.off(a))[0]
    def f64(self, a): return struct.unpack_from('>d', self.b, self.off(a))[0]
    def u32(self, a): return struct.unpack_from('>I', self.b, self.off(a))[0]
    def cstr(self, a):
        o = self.off(a); e = self.b.index(b'\0', o); return self.b[o:e].decode('ascii', 'replace')


if __name__ == '__main__':
    r = Rso()
    for arg in sys.argv[1:]:
        k, a = arg.split(':'); a = int(a, 16)
        print(f'{arg} = {({"f": r.f32, "d": r.f64, "u": r.u32, "s": r.cstr}[k](a))!r}')
