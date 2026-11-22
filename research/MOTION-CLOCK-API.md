# Original motion clock API

Query motion-clock-api.graph.jsonl with gk graph. Detailed dependency evidence is motion-timing-pcode.graph.jsonl; motion-clock-pseudocode.json contains only bounded per-function dependency summaries. Current scope:80079210,80079388,8025AA04,802163C8,8021642C.

80079388 obtains a step through8025AA04, stores it in field68, preserves previous frame58 into5C and adds the step to58. Its countdown at60 uses cScene getSpeedRate802163C8 at original scene object8069BCA8. 8025AA04 tests word r3+38 against1 and selects an incoming FP value or float r3+34. Implicit FP arguments/object ownership remain uncertain; do not assign a frames-per-wall-second rule.

getSpeedRate returns scene float18, except a nonzero result from flag helper8006A66C(global80697E64,0x12) with nonzero rate selects1.0. Owned DOL numeric constants805622CC=0 and805622D8=1. Flag semantic name remains unknown. setSpeedRate compares incoming priority against float24 and float28; accepted priority goes to24. Zero speed clears18/1C. Nonzero input stores18, initializing1C if previously zero. All callers/priority names remain unclassified.

read_native_core.py now records scene18/1C/24/28 plus actual motion step68. These concurrent read-only observations are not frame atomic and do not evaluate the flag override. The source AA/AC observations of6–7 MOT units per wall second do not establish original console FPS or a clock bug. No game clock, animation, HP or contact rule is changed by this analysis.
