"""Read constants and strings from the owned main.dol by virtual address.

python -I tools/pl0000/dol.py f:8056a098 s:80574a6e ...   (f = f32, d = f64, u = u32, h = s16, s = C string)
"""
import struct, sys
from pathlib import Path

DOL = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol')


class Dol:
    def __init__(self, path=DOL):
        self.b = path.read_bytes(); w = lambda o: struct.unpack_from('>I', self.b, o)[0]
        self.secs = [(w(0x48 + i * 4), w(i * 4), w(0x90 + i * 4)) for i in range(18)]

    def off(self, addr):
        for a, o, n in self.secs:
            if n and a <= addr < a + n: return o + addr - a
        raise KeyError(hex(addr))

    def f32(self, a): return struct.unpack_from('>f', self.b, self.off(a))[0]
    def f64(self, a): return struct.unpack_from('>d', self.b, self.off(a))[0]
    def u32(self, a): return struct.unpack_from('>I', self.b, self.off(a))[0]
    def s16(self, a): return struct.unpack_from('>h', self.b, self.off(a))[0]
    def cstr(self, a):
        o = self.off(a); e = self.b.index(b'\0', o); return self.b[o:e].decode('ascii', 'replace')


if __name__ == '__main__':
    d = Dol()
    for arg in sys.argv[1:]:
        k, a = arg.split(':'); a = int(a, 16)
        v = {'f': d.f32, 'd': d.f64, 'u': d.u32, 'h': d.s16, 's': d.cstr}[k](a)
        print(f'{arg} = {v!r}')
