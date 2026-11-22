# Remote speaker API vocabulary

Reference: Dolphin GPL-2.0-or-later, commit257a6d0eac1d915216dbc974e9390af3050af2b9. research/remote-speaker-reference.json pins downloaded source URLs/hashes. The Rust speaker module retains attribution. The PC stereo mixer is a host adaptation.

Report0x14 updates enabled status; report0x19 updates mute. Acknowledge only when requested by bit1; status bit2 reflects enabled. Report0x18 carries up to20 data bytes, length from control bits3..7, with no normal ACK. Speaker bus space0xA2/0xA3 is separate from Nunchuk0xA4/0xA5; ignore the slave-address read/write bit. Speaker writes cannot enter the extension-key observer.

Registers: 0 sample data;1 stop/playback flags;2 format;3..4 little-endian rate divisor;5 volume;6 unknown;7 decoder flags/reset;8 playback enable;9 input enable. Writes while disabled have no effect. The decoder uses Yamaha ADPCM0x00, signed8 PCM0x40 or little-endian16 PCM0x60/0xE0. Other formats are unsupported. ADPCM starts predictor0/step127, saturates predictor to signed16 and step127..24576. Data may advance decoder state while muted; only enabled playback, unmuted, format-bit0 clear emits output. Native hardware FIFO/buffer timing is not emulated or verified.

PC route: native USB ioctlv -> Bluetooth HID -> SpeakerBlock -> separate mono AudioSink method -> SpeakerMixer -> original installed stereo sink/Bevy AudioBridge. AID is the output clock; mono uses bounded linear resampling and source volume, centered in both channels. With no remote audio, AID blocks pass through byte-for-byte. Queue overflow and saturation are observable numeric diagnostics. This is a PC output adaptation; original acoustics, bit/mix/event timing equivalence remain unverified.

Speaker hardware/codec state is saved in snapshot version5. Version1..4 loads default speaker state without consuming the new fields. Pending host output and numeric trace counters are transient.

MW_TRACE_REMOTE_SPEAKER=1 enables numeric report/register/config/output counters; it does not print audio or extension-key payloads. tools/index_remote_audio.py summarizes completed labelled runs. Run cargo test with native-platform/Cargo.toml for the native mixer/USB suite and -p gecko --lib for codec/protocol/snapshot tests.
