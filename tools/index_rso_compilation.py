"""Index compiled RSO entries without exposing generated game C++ bodies."""
import bisect
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPORT = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPORT / 'builders'))
from native_graph import rso

def main():
    source = EXPORT / 'extracted/disc/DATA/files/dll/Rel/enemyRSO.rso'
    raw = source.read_bytes()
    _, exports, _, _, internal = rso(raw)
    entries = sorted(int(value, 16) for value in re.findall(r'PPC_FUNC\(f_([0-9A-F]+)\)',
        (ROOT / 'tools/upstream-native-core/port/generated-rso/rso_funcs.h').read_text()))
    named = {0x81000000 + e['offset']: e['name'] for e in exports if e['section'] == 1}
    spans = sorted(named)
    rows = [{'meta': {'sourceSha256': hashlib.sha256(raw).hexdigest(), 'canonicalBase': '81000000',
                      'authority': 'Original RSO exports, text-pointer relocations and compiler-discovered direct entries; canonical addresses are not runtime placement'}}]
    for address in entries:
        parent = spans[bisect.bisect_right(spans, address) - 1]
        ident = f'entry:{address:08x}'
        rows.append({'n': {'id': ident, 'kind': 'native-code-entry', 'label': named.get(address, 'unnamed private entry'),
                           'attrs': {'canonicalAddress': f'{address:08x}', 'fileOffset': address - 0x81000000, 'exported': address in named}}})
        if parent != address:
            rows.append({'e': {'src': ident, 'dst': f'entry:{parent:08x}', 'kind': 'inside-export-span',
                               'attrs': {'semanticIdentity': 'unknown; enclosing export is a compilation span, not proof of identity'}}})
    references = 0
    for offset, relocation in internal.items():
        if relocation['type'] != 1 or relocation['section'] != 1:
            continue
        target = 0x81000000 + relocation['target']
        if target not in entries:
            raise ValueError('Original text pointer has no compiled entry')
        ident = f'reloc:{offset:x}'
        rows.append({'n': {'id': ident, 'kind': 'native-pointer-relocation', 'label': 'ADDR32 text pointer', 'attrs': {'fileOffset': offset}}})
        rows.append({'e': {'src': ident, 'dst': f'entry:{target:08x}', 'kind': 'points-to-code', 'attrs': {'authority': 'Original internal RSO relocation'}}})
        references += 1
    (ROOT / 'research/rso-compiled-entries.graph.jsonl').write_text('\n'.join(json.dumps(row) for row in rows) + '\n', encoding='utf-8')
    (ROOT / 'research/RSO-COMPILED-INDEX.md').write_text(
        '# Compiled enemy module entries\n\n' + f'{len(named)} exports; {len(entries)} total compiled entries; {references} original ADDR32 text references.\n\n'
        'Query `native-code-entry` by canonicalAddress/fileOffset; follow `points-to-code` for original table references. '
        '`inside-export-span` is a bounds relation, never a character or gameplay identity. Generated game C++ bodies stay on D. '
        'The original loader chooses the runtime base, links BSS/imports and patches instructions; compiled code consumes those live operands. '
        'No native timing or full module behavior equivalence is implied.\n', encoding='utf-8')
    print(json.dumps({'exports': len(named), 'entries': len(entries), 'pointerReferences': references}))

if __name__ == '__main__':
    main()
