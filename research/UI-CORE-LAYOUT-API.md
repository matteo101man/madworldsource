# UI layout vocabulary

ui-resource: owned IDD with provenance. ui-atlas: source texture page, format, dimensions and byte range. ui-uv-group: contiguous source descriptor range. ui-sprite-rect: exact x0,y0,x1,y1 bounds, texture page and record offset. contains: resource/group membership. uses-texture: sprite to its page.

Active table: ui-live-core-layout.json / .graph.jsonl, reproduced by index_live_ui_layout.py. Archived single-page candidate: ui-core-layout.json / .graph.jsonl, reproduced by index_ui_layout.py. Both use TUV >4H group descriptors and >HH4f sprite records. Group offset is relative to the descriptor-table beginning. The second sprite u16 indexes a texture page; single-page records all use zero. This table describes layout; IDT transforms/event timing remain undecoded.
