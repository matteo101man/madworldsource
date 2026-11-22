# Native input boundary

input-pcode.graph.jsonl contains15 bounded resident functions, indexed by input-pcode-report.json. Query: python tools/query_input.py ROOT ADDRESS. input-init-pseudocode.json retains focused dependencies and normalized branch edges. Foreign tail-callee operations are excluded; raw branch operands may invert after normalization.

Bootstrap call chain: main ->8020ee00 ->8036e260 ->8036e26c ->8032b15c ->80338f88 ->8033f708. First initialization loop at8033f750 repeatedly calls no-op8033e0c8 and waits for global80717ac8 to become nonzero. Native core reaches this with IOS reporting missing /dev/usb/oh1/57e/305. This is boundary evidence, not a complete KPAD/WPAD naming proof or PC mapping.

A virtual IOS USB V0 HCI controller now provides bounded initialization replies, queued event buffers, explicit unsupported-command errors and no paired devices. It is a platform adapter, not recovered game logic. It does not yet supply ACL/HID reports, keyboard/gamepad input, motion gesture semantics, connection callbacks or PC UI. No gameplay completion claim follows from a successful HCI reset.


Current adapter supersedes the early HCI-only limitation above: Rust Gecko virtual remote supplies ACL/HID reports with a Nunchuk. Explicit MW_INPUT_FILE packet: buttonsBE16, nunchukButtons, stickX, stickY, shake,0,0. Neutral stick128/128. Gecko logical A0x0008, B0x0004, One0x0002, Two0x0001, Plus0x1000; these are virtual remote wire buttons, not the final PC controls. The source game sees A as pad hold00000010 in the current boot. Windows reader shares delete/read/write; producer retries atomic replacement and always cleans up its owned process. Synthetic concurrent replacement test passes. No desktop input automation, finished keyboard/gamepad mapping, motion-action balancing, or PC prompts yet.

The new optional madworld-native host publishes the same eight-byte packet atomically from focused Bevy window input. The original CPU worker alone reads host packets; standalone diagnostic processes continue reading their own explicit input file. Unfocused windows publish neutral state. Initial mappings are listed in the root README and remain provisional hardware bindings rather than validated native action names. This adds no global keyboard hook, desktop input automation, completed gesture balancing, or replacement HUD prompts.
