"""Synthetic instructions compiled by the production RSO emitter for tests."""
from pathlib import Path
from rso_recomp import BASE, Image, RSORecompiler, Function
from recomp import write_if_changed

def generate():
    words = [0x3c601234, 0x38635678, 0x80830004, 0x4e800020, 0x48000001, 0x4e800020]
    data = bytes(248) + b''.join(w.to_bytes(4, 'big') for w in words)
    functions = [Function('synthetic_export_with_private_constructor', BASE + 248, 24)]
    relocations = {250: {'type': 6}, 254: {'type': 4}, 258: {'type': 4}, 264: {'type': 10}}
    compiler = RSORecompiler(Image(data, []), functions, relocations)
    compiler.seed_relocated_entries({4096: {'type': 1, 'section': 1, 'target': 264}})
    compiler.analyze()
    text = '// Synthetic fixture from the production RSO emitter.\n' + '\n'.join(compiler.emit_function(f) for f in functions)
    write_if_changed(Path(__file__).resolve().parents[1] / 'tests/rso_fixture.inc', text)

if __name__ == '__main__':
    generate()
