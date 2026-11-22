"""Refresh local build-identity manifests (source hashes and host binary hashes).
Identity only, not fidelity evidence. Keeps existing manifest shapes and file lists."""
import hashlib, json, pathlib, sys
ROOT = pathlib.Path(__file__).resolve().parents[1]
CORE = ROOT / 'tools/upstream-native-core'

def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()

def refresh_sources():
    path = ROOT / 'research/native-core-local-files.json'
    doc = json.loads(path.read_text(encoding='utf-8'))
    missing = []
    for entry in doc['files']:
        p = CORE / entry['path']
        if p.exists():
            entry['sha256'] = sha256(p)
        else:
            missing.append(entry['path'])
    path.write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    return len(doc['files']), missing

def refresh_binaries():
    path = ROOT / 'evidence/native-host-binary-hashes.json'
    doc = json.loads(path.read_text(encoding='utf-8'))
    missing = []
    for entry in doc['binaries']:
        p = pathlib.Path(entry['path'])
        if p.exists():
            entry['size'] = p.stat().st_size
            entry['sha256'] = sha256(p)
        else:
            missing.append(entry['path'])
    path.write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    return len(doc['binaries']), missing

if __name__ == '__main__':
    n_src, miss_src = refresh_sources()
    n_bin, miss_bin = refresh_binaries()
    print(json.dumps({'sourceFiles': n_src, 'missingSources': miss_src, 'binaries': n_bin, 'missingBinaries': miss_bin}))
    sys.exit(1 if miss_src or miss_bin else 0)
