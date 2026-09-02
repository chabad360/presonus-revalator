use std::ops::RangeInclusive;

const AUDIO_TAPER_CURVE: f32 = 3.0;

/// Mapping between a control's stored value and its normalized position.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ValueScale {
    /// Equal value increments occupy equal distances.
    #[default]
    Linear,
    /// Equal ratios occupy equal distances.
    ///
    /// Both range endpoints must be finite and greater than zero. This is useful
    /// for values such as frequency. Values stored in decibels should use
    /// [`Self::Linear`] for knobs or [`Self::AudioTaper`] for audio faders.
    Logarithmic,
    /// An inverse-log-style taper for faders whose stored values are decibels.
    ///
    /// This compresses the large attenuation range near the bottom of the
    /// fader, leaving more travel around unity gain where fine control matters.
    AudioTaper,
}

impl ValueScale {
    pub(crate) fn normalize(self, value: f32, range: &RangeInclusive<f32>) -> f32 {
        let (start, end) = (*range.start(), *range.end());
        assert_valid_range(start, end);

        if value.is_nan() {
            return 0.0;
        }

        let normalized = match self {
            Self::Linear => (value - start) / (end - start),
            Self::Logarithmic => {
                assert_logarithmic_range(start, end);
                (value.max(f32::MIN_POSITIVE).ln() - start.ln()) / (end.ln() - start.ln())
            }
            Self::AudioTaper => ((value - start) / (end - start)).powf(AUDIO_TAPER_CURVE),
        };

        normalized.clamp(0.0, 1.0)
    }

    pub(crate) fn denormalize(self, normalized: f32, range: &RangeInclusive<f32>) -> f32 {
        let (start, end) = (*range.start(), *range.end());
        assert_valid_range(start, end);
        let normalized = normalized.clamp(0.0, 1.0);

        match self {
            Self::Linear => start + normalized * (end - start),
            Self::Logarithmic => {
                assert_logarithmic_range(start, end);
                (start.ln() + normalized * (end.ln() - start.ln())).exp()
            }
            Self::AudioTaper => start + normalized.powf(1.0 / AUDIO_TAPER_CURVE) * (end - start),
        }
    }
}

pub(crate) fn snap_to_notch(
    value: f32,
    scale: ValueScale,
    range: &RangeInclusive<f32>,
    notches: &[f32],
    tolerance: f32,
) -> f32 {
    let position = scale.normalize(value, range);
    notches
        .iter()
        .copied()
        .filter(|notch| range.contains(notch))
        .map(|notch| (notch, (scale.normalize(notch, range) - position).abs()))
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .filter(|(_, distance)| *distance <= tolerance)
        .map_or(value, |(notch, _)| notch)
}

/// Converts a linear amplitude ratio to dBFS.
///
/// Zero and negative amplitudes map to negative infinity.
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude <= 0.0 || !amplitude.is_finite() {
        f32::NEG_INFINITY
    } else {
        20.0 * amplitude.log10()
    }
}

/// Converts a decibel value to a linear amplitude ratio.
pub fn db_to_amplitude(db: f32) -> f32 {
    if db == f32::NEG_INFINITY {
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}

fn assert_valid_range(start: f32, end: f32) {
    assert!(
        start.is_finite() && end.is_finite() && start < end,
        "control range must contain two increasing finite values"
    );
}

fn assert_logarithmic_range(start: f32, end: f32) {
    assert!(
        start > 0.0 && end > 0.0,
        "logarithmic controls require a strictly positive range"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logarithmic_midpoint_is_geometric_mean() {
        let range = 20.0..=20_000.0;
        let midpoint = ValueScale::Logarithmic.denormalize(0.5, &range);

        assert!((midpoint - 632.4555).abs() < 0.001);
        assert!((ValueScale::Logarithmic.normalize(midpoint, &range) - 0.5).abs() < 0.000_001);
    }

    #[test]
    fn audio_taper_puts_deep_attenuation_near_the_bottom() {
        let range = -96.0..=10.0;
        let midpoint = ValueScale::AudioTaper.denormalize(0.5, &range);

        assert!((midpoint - -11.8677).abs() < 0.001);
        assert!(ValueScale::AudioTaper.normalize(-60.0, &range) < 0.05);
        assert!((ValueScale::AudioTaper.normalize(midpoint, &range) - 0.5).abs() < 0.000_001);
    }

    #[test]
    fn linear_mapping_clamps_out_of_range_values() {
        let range = -60.0..=6.0;

        assert_eq!(ValueScale::Linear.normalize(-100.0, &range), 0.0);
        assert_eq!(ValueScale::Linear.normalize(20.0, &range), 1.0);
        assert_eq!(ValueScale::Linear.normalize(f32::NAN, &range), 0.0);
        assert_eq!(ValueScale::Linear.denormalize(0.5, &range), -27.0);
    }

    #[test]
    fn amplitude_and_decibel_conversions_round_trip() {
        for amplitude in [0.001, 0.1, 0.5, 1.0, 2.0] {
            let actual = db_to_amplitude(amplitude_to_db(amplitude));
            assert!((actual - amplitude).abs() < 0.000_001);
        }

        assert_eq!(amplitude_to_db(0.0), f32::NEG_INFINITY);
        assert_eq!(db_to_amplitude(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    #[should_panic(expected = "strictly positive")]
    fn logarithmic_mapping_rejects_zero() {
        ValueScale::Logarithmic.normalize(0.5, &(0.0..=1.0));
    }
}
