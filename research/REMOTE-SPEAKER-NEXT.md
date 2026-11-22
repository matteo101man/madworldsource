# Remote speaker next verification

The former trivial-ACK backend is replaced with speaker enable/status/mute, isolated register handling, gated Yamaha ADPCM/PCM decoding and a separate PC mono mixing lane. See REMOTE-SPEAKER-API.md and remote-speaker.graph.jsonl. Synthetic native USB ACL -> codec -> stereo sink passes. Hardware suite21, upstream codec/protocol/snapshot suite8 and frontend8 tests pass. Version5 speaker snapshots retain version1..4 loading defaults.

Baseline300s native replay: no0x18 reports. After proper protocol state/ACK handling, a300s replay receives44,710 full20-byte speaker-data packets and completes5,814 GPU presentations. No played speaker blocks emerge. Latest30s state trace confirms source config: ADPCM format0, divisor2000 (6kHz), volume88, input/playback enabled, but source mute=true before the first data packet and throughout observations. Mixer final counters are0 received/mixed/dropped/clipped. Do not force unmute or label these idle packets as audible game speech. This is a meaningful protocol fix and a narrowed unknown, not recovered game sound coverage.

Next: identify the original speaker-sound event dispatcher / a naturally requested unmute and verify its decoded samples alongside AID. The original hardware FIFO, callback scheduling, complete audio events, bit/mix/timing comparison and human listening remain unverified. Raw AID continues unchanged while the remote lane is silent. MW_AUDIO_WAV now describes PC stereo output including the remote lane when active; historical captures predate that lane.

Full native competition completion is still another gameplay gap. Prefer bounded recovery of eligibility/input windows or an explicitly labelled original-API fixture to repeated random combat replays.
