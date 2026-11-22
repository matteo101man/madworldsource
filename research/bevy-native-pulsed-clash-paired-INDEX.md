# Observed native Rin/player pair

Source: bevy-native-pulsed-clash-feedback-observations.jsonl

144 concurrent observations. Query bevy-native-pulsed-clash-paired.graph.jsonl.

`native-pair-observation observes-role native-actor`; `native-pair-observation observes-motion native-motion`, with explicit role and complete content-hash match. Frame fields and native positions are sampled concurrently; do not claim frame-atomic synchronization or visual parity.
