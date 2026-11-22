"""Validate a recorded native clash fixture, including original role contents."""
import argparse,subprocess,sys,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--label',default='bevy-native-clash-fixture');ap.add_argument('--require-natural',action='store_true');a=ap.parse_args()
    if not a.label.replace('-','').isalnum():raise ValueError('Replay label')
    for tool,args in [('index_native_clash.py',[a.label,'--require-success']),('audit_native_pair_contents.py',['--label',a.label])]:
        result=subprocess.run([sys.executable,str(ROOT/'tools'/tool),*args],capture_output=True,text=True,encoding='utf-8')
        print(result.stdout,end='')
        if result.returncode:print(result.stderr,end='');raise SystemExit(result.returncode)
    s=json.loads((ROOT/'evidence'/f'{a.label}-clash-summary.json').read_text(encoding='utf-8'))
    if s['fixtureRequested'] and not s['fixtureAccepted']:raise ValueError('Fixture not activated')
    if a.require_natural and (s['fixtureRequested'] or s['fixtureAccepted'] or s['fixtureTrace']):raise ValueError('Natural replay must have no fixture request or execution')
    if not s['fixtureRequested'] and not s['fixtureAccepted'] and not s['fixtureTrace']:print('NATIVE_NATURAL_CLASH_TRACE_AUDIT PASSED; ordinary PC input, native contact, native success/follow-up/return; no full-fight or visual/timing proof.')
    else:print('NATIVE_CLASH_FIXTURE_TRACE_AUDIT PASSED; original-API setup, shared held PC input, native success/follow-up/return. Not natural contact, full fight or visual/timing proof.')
if __name__=='__main__':main()
