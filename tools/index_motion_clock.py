from pathlib import Path
import sys,json
root=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');sys.path.insert(0,str(root/'tools'))
from query_boot import summarize
from save_typed_graph import save_graph
functions=[('80079210','cByMotion move',{'calls':['80079388','80259854'],'unknown':'Mode dispatch and external timing context remain partial'}),('80079388','Motion step helper',{'pseudocode':['step = helper8025AA04(implicit FP input from field64; ABI incomplete)','field68 = step','field5C = previous field58','field58 += step','field60 countdown uses scene getSpeedRate802163C8'],'sceneObject':'0x8069bca8','unknown':'Implicit helper FP ABI and complete rate provenance'}),('8025aa04','Motion step selector',{'pseudocode':['Test word at r3+38 against1','Return incoming FP value or float at r3+34'],'unknown':'Object type and implicit input ownership; do not equate with wall seconds'}),('802163c8','cScene getSpeedRate',{'pseudocode':['rate = scene.float18','if helper8006A66C(global80697E64,0x12)!=0 and rate!=0: return1','return rate'],'constants':{'0x805622cc':0.0,'0x805622d8':1.0},'unknown':'Flag0x12 semantic name and lifetime of rate changes'}),('8021642c','cScene setSpeedRate',{'pseudocode':['Reject if priority exceeds scene.float24 or scene.float28','Accepted priority stored as float24','Zero input clears float18 and float1C','Nonzero input stores float18; float1C initialized if previously zero'],'unknown':'Priority enum names and all callers remain unclassified'})]
records=[dict(type='node',id='motion-clock',kind='timing_api',label='Original motion stepping',status='partial',scope='Recovered field dependencies; no wall-clock or console parity claim')]
queries=[]
for pc,label,attrs in functions:
 ident='motion:'+pc;records.append(dict(type='node',id=ident,kind='native_function',label=label,address='0x'+pc,**attrs));records.append(dict(type='edge',source='motion-clock',target=ident,kind='contains'));queries.append(summarize(root,pc,graph_name='motion-timing-pcode.graph.jsonl'))
save_graph(root/'research/motion-clock-api.graph.jsonl',records,'Bounded original Pcode dependencies; explicit unknown ABI/timing')
(root/'research/motion-clock-pseudocode.json').write_text(json.dumps(queries,indent=2)+'\n',encoding='utf-8')
(root/'research/MOTION-CLOCK-API.md').write_text('''# Original motion clock API

Query motion-clock-api.graph.jsonl with gk graph. Detailed dependency evidence is motion-timing-pcode.graph.jsonl; motion-clock-pseudocode.json contains only bounded per-function dependency summaries. Current scope:80079210,80079388,8025AA04,802163C8,8021642C.

80079388 obtains a step through8025AA04, stores it in field68, preserves previous frame58 into5C and adds the step to58. Its countdown at60 uses cScene getSpeedRate802163C8 at original scene object8069BCA8. 8025AA04 tests word r3+38 against1 and selects an incoming FP value or float r3+34. Implicit FP arguments/object ownership remain uncertain; do not assign a frames-per-wall-second rule.

getSpeedRate returns scene float18, except a nonzero result from flag helper8006A66C(global80697E64,0x12) with nonzero rate selects1.0. Owned DOL numeric constants805622CC=0 and805622D8=1. Flag semantic name remains unknown. setSpeedRate compares incoming priority against float24 and float28; accepted priority goes to24. Zero speed clears18/1C. Nonzero input stores18, initializing1C if previously zero. All callers/priority names remain unclassified.

read_native_core.py now records scene18/1C/24/28 plus actual motion step68. These concurrent read-only observations are not frame atomic and do not evaluate the flag override. The source AA/AC observations of6–7 MOT units per wall second do not establish original console FPS or a clock bug. No game clock, animation, HP or contact rule is changed by this analysis.
''',encoding='utf-8')
print('Updated bounded motion-clock API, six nodes/five edges')
