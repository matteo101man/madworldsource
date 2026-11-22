# Common HUD layout index

Reproduce with tools/index_ui_layout.py. ui-core-layout.json has 26 groups and 201 source sprite rectangles; ui-core-layout.graph.jsonl provides typed atlas/group/rectangle nodes. Source member SHA and byte ranges are included.

TUV group descriptors are >4H (group, first, count, offset). Offset is relative to the descriptor-table beginning. Sprite records are >HH4f (global ID, unknown16, normalized UV bounds). All UV edges align to exact source pixels. IDT animation commands and unknown16 semantics remain undecoded; group IDs do not identify navigation kinds.
