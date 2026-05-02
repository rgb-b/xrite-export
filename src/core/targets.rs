//! Dot gain target table and step-value interpolation.
//!
//! The 16 anchor points represent standard flexographic dot gain targets.
//! For any step value not in the table, `interpolate_target` performs linear
//! interpolation between the two nearest anchors.
//!
//! Steps outside the range 0.4–100 return `None` (no deviation shown).

/// Fixed dot gain anchor points: (nominal_step, target_value).
/// Must remain sorted ascending by step value.
pub const DOT_GAIN_TARGETS: &[(f64, f64)] = &[
    (0.4,  1.0),
    (0.8,  2.0),
    (1.0,  3.0),
    (3.0,  9.0),
    (5.0,  13.0),
    (10.0, 22.0),
    (20.0, 37.0),
    (30.0, 51.0),
    (40.0, 62.0),
    (50.0, 72.0),
    (60.0, 81.0),
    (70.0, 88.0),
    (80.0, 93.0),
    (90.0, 97.0),
    (95.0, 99.0),
    (100.0, 100.0),
];

/// Returns the interpolated dot gain target for a given step value.
///
/// - Exact matches return the known target directly.
/// - Values between anchors are linearly interpolated.
/// - Values outside [0.4, 100.0] return `None`.
pub fn interpolate_target(step: f64) -> Option<f64> {
    let anchors = DOT_GAIN_TARGETS;

    // Out of range
    if step < anchors.first()?.0 || step > anchors.last()?.0 {
        return None;
    }

    // Exact match
    for &(s, t) in anchors {
        if (s - step).abs() < f64::EPSILON {
            return Some(t);
        }
    }

    // Linear interpolation between bracketing anchors
    for window in anchors.windows(2) {
        let (s0, t0) = window[0];
        let (s1, t1) = window[1];
        if step >= s0 && step <= s1 {
            let ratio = (step - s0) / (s1 - s0);
            return Some(t0 + ratio * (t1 - t0));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_anchors() {
        assert_eq!(interpolate_target(50.0), Some(72.0));
        assert_eq!(interpolate_target(100.0), Some(100.0));
        assert_eq!(interpolate_target(0.4), Some(1.0));
    }

    #[test]
    fn interpolated_midpoint() {
        // Midpoint between (40, 62) and (50, 72) → step 45 → target 67
        let t = interpolate_target(45.0).unwrap();
        assert!((t - 67.0).abs() < 0.01, "expected ~67.0, got {t}");
    }

    #[test]
    fn interpolated_75_step() {
        // Between (70, 88) and (80, 93): ratio = 0.5 → target = 90.5
        let t = interpolate_target(75.0).unwrap();
        assert!((t - 90.5).abs() < 0.01, "expected ~90.5, got {t}");
    }

    #[test]
    fn out_of_range() {
        assert_eq!(interpolate_target(0.0), None);
        assert_eq!(interpolate_target(101.0), None);
        assert_eq!(interpolate_target(-5.0), None);
    }
}
