# Native motion cues

`motion-sequence contains motion-cue`: 32-byte header, u16 count at6, 48-byte events, native type u8 at0 and frame f32 at20. Native authority: getFirstEventFrame at0x8025b0c0; init_snd at0x80259070 binds the event array at+32. Other words, header fields and trailing bytes are preserved untyped. Earlier frame-at4 interpretation was incorrect and has been replaced. Shorter legacy layouts remain explicit unsupported records. Payload opcode semantics are being linked through the original motion-rig dispatcher.
