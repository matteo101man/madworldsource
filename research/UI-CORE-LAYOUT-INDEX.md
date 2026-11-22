# Active common HUD index

Reproduce with tools/index_live_ui_layout.py. ui-live-core-layout.json and ui-live-core-layout.graph.jsonl index six pages, 42 groups and 546 sprites from standalone etc/core_us.idd. UI-CORE-LAYOUT-API.md defines the vocabulary. Source file length, SHA256, chunk/table offsets, UVs and exact per-page pixel bounds are included.

The active native DI trace and loaded-pixel audit are described in UI-PC-PROMPTS.md. IDT animation/transform command semantics remain undecoded. TUV group IDs are not native navigation-kind IDs.

The initial, inactive core2_us.dat#core.idd candidate has 26 groups/201 sprites. tools/index_ui_layout.py preserves that historical table in ui-core-layout.json/.graph.jsonl and separate UI-ARCHIVED-CORE-LAYOUT documents.
