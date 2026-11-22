//! AX HLE unit checks (gecko's own workspace cannot build its test harness here).
#[cfg(test)]
mod tests {
    use gecko::flipper::dsp::hle::*;
    use gecko::flipper::dsp::hle::pb;

    #[test]
    fn hash_ector_matches_dolphin_definition() {
        assert_eq!(hash_ector(&[]), 0);
        assert_eq!(hash_ector(&[1]), 8);
        assert_eq!(hash_ector(&[1, 1]), (8u32 ^ 1).rotate_left(3));
    }

    #[test]
    fn linear_resample_ratio_one_is_passthrough_delayed() {
        let mut src = (1..=200).map(|v| v as i16);
        let mut out = [0i16; 96];
        let mut last = [0; 4];
        let pos = resample(|| src.next().unwrap(), &mut out, &mut last, 0, 0x10000, 1, None);
        // Like Dolphin, the output is the oldest sample of the 4-sample window.
        assert_eq!(pos, 0);
        assert_eq!(out[..4], [0, 0, 0, 1]);
        assert_eq!(out[95], 93);
        assert_eq!(last, [93, 94, 95, 96]);
    }

    #[test]
    fn polyphase_unity_dc_gain() {
        // Sum of each 4-tap phase of the free coefficient ROM is ~0x7FFF: DC passes.
        let coef: Vec<i16> = (0..0x800)
            .map(|k| i16::from_be_bytes([gecko::flipper::dsp::FREE_COEF[2 * k], gecko::flipper::dsp::FREE_COEF[2 * k + 1]]))
            .collect();
        let mut out = [0i16; 96];
        let mut last = [10000; 4];
        resample(|| 10000, &mut out, &mut last, 0, 0xB000, 0, Some(&coef[..0x200]));
        assert!(out.iter().all(|&s| (s as i32 - 10000).abs() < 200), "{out:?}");
    }

    #[test]
    fn mixer_control_ramp_bits() {
        assert_eq!(convert_mixer_control(0x4), MIX_MAIN_L | MIX_MAIN_R | (MIX_MAIN_L << 1) | (MIX_MAIN_R << 1));
        assert_eq!(convert_mixer_control(0x1), MIX_MAIN_L);
    }

    #[test]
    fn layout_spans_cover_expected_sizes() {
        let len = |l: Layout| l.spans().iter().map(|(a, b)| b - a).sum::<usize>();
        assert_eq!(len(Layout::New), pb::WORDS - 5);
        assert_eq!(len(Layout::Mid), pb::WORDS - 11);
        assert_eq!(len(Layout::Old), pb::WORDS - 6);
    }
}
