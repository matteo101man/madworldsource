"""Compare owned native ev21 applied packets with staged source camera samples.

The clock hook observes the preceding camera application. Its first observation
is the old scene camera, not ev21. This does not prove scene handoff or pixels.
"""
from pathlib import Path
import argparse,collections,json,math
ROOT=Path(__file__).resolve().parents[2]

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('record',type=Path)
    parser.add_argument('--out',type=Path,default=ROOT/'research/EV21-CAMERA-COMPARE.json')
    args=parser.parse_args()
    source_path=ROOT/'assets/native/events/ev21/event.json'
    source=json.loads(source_path.read_text())
    samples=[p for cut in source['cameras']for p in cut['samples']]
    items=[json.loads(line)for line in args.record.read_text().splitlines()if line.strip()]
    if not items or items[0].get('schema')!='mw-event-record/1':raise ValueError('Native event-record schema')
    rows=items[1:]
    if [r['before']for r in rows]!=list(range(len(samples))):raise ValueError('Capture does not cover the complete source clock')
    errors={name:0. for name in ['eye','target','up','fovDegrees']}
    for row in rows[1:]:
        p=samples[row['before']-1];yaw=math.atan2(p[0]-p[3],p[2]-p[5])
        up=[-math.cos(yaw)*math.sin(p[6]),math.cos(p[6]),math.sin(yaw)*math.sin(p[6])]
        for name,expected in [('eye',p[:3]),('target',p[3:6]),('up',up)]:
            errors[name]=max(errors[name],max(abs(a-b)for a,b in zip(row['applied'][name],expected)))
        errors['fovDegrees']=max(errors['fovDegrees'],abs(row['applied']['fovDegrees']-math.degrees(p[7])))
    cadence=dict(collections.Counter(str(r['framesPerUpdate'])for r in rows))
    if cadence!={'2':len(rows)}:raise ValueError('Observed cadence differs from ev21 two-field update')
    if any(not math.isfinite(v)or v>0.00002 for v in errors.values()):raise ValueError(f'Camera fields diverge: {errors}')
    data=dict(schema='mw-event-camera-comparison/1',source=str(source_path),record=str(args.record),rows=len(rows),compared=len(rows)-1,
              maximumAbsoluteErrors=errors,framesPerUpdate=cadence,
              scope='1079 observed applied packets match source frames0..1078. Clock hook is one camera application behind; first row is preceding scene camera. No final lifecycle/handoff, actor, audio or pixel equivalence claim.')
    args.out.write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(compared=data['compared'],maximumAbsoluteErrors=errors)))

if __name__=='__main__':main()
