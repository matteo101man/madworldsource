"""Stage the source MOTs behind Kreely's handgun overlays for the native Rust slice.

python -I tools/kreely/stage_kreely_native_clips.py
Reads assets/native/kreely-rig.json (overlay keys "<clip id>[|variant]") and writes assets/native/kreely-gun-motion.json:
{clips: [{id, name, frames, tracks}]}: the untrimmed original clips (Jude em0400 gameplay MOTs, Leonardo em0940 event
cuts), decoded with the explorer's MOT reader. Every clip's source bytes are hash-checked against the catalog / events
index. Overlays (trim ranges, mirroring, prop and arm tracks, floor lift, facing) stay in kreely-rig.json.
"""
import hashlib, json, sys
from pathlib import Path

PORT = Path(__file__).resolve().parents[2]
EXPLORER = PORT.parents[1] / 'exports' / 'madworld'
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import motion


def main():
    rig = json.loads((PORT / 'assets/native/kreely-rig.json').read_text(encoding='utf-8'))
    cat = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    ev = json.loads((EXPLORER / 'events.json').read_text(encoding='utf-8'))
    items = {c['id']: c for c in cat['clips'] + ev['clips']}; banks = {b['id']: b for b in cat['banks']}
    out, report = [], []
    for cid in sorted({k.split('|')[0] for k in rig['overlays']}):
        it = items[cid]; path = it.get('sourcePath') or banks[it['bank']]['path']
        with open(path, 'rb') as f: f.seek(it['offset']); data = f.read(it['size'])
        assert hashlib.sha256(data).hexdigest() == it['sha256'], f'{cid}: source changed'
        m = motion(data)
        out.append(dict(id=cid, name=it['name'], frames=m['frames'], tracks=m['tracks']))
        report.append(dict(id=cid, name=it['name'], frames=m['frames'], sha256=it['sha256']))
    dst = PORT / 'assets/native/kreely-gun-motion.json'
    dst.write_text(json.dumps(dict(clips=out), separators=(',', ':')), encoding='utf-8')
    manifest = PORT / 'research/kreely-staging-manifest.json'
    man = json.loads(manifest.read_text(encoding='utf-8')) if manifest.exists() else {}
    man['gunMotion'] = dict(path=str(dst), sha256=hashlib.sha256(dst.read_bytes()).hexdigest(), clips=report,
                            authority='Original MOT bytes (Jude em0400 gameplay, Leonardo em0940 ending event cuts), hash-checked')
    manifest.write_text(json.dumps(man, indent=2), encoding='utf-8')
    print(json.dumps(dict(clips=len(out), bytes=dst.stat().st_size)))


if __name__ == '__main__':
    main()
