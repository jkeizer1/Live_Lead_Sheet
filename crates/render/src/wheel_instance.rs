//! What the wheel shader needs: where the wheel is, and what each of its
//! seven wedges shows (EFS-1.10).

use crate::theme;

/// Wedges in the wheel: the seven degrees of a key.
pub const WEDGES_IN_WHEEL: usize = 7;

/// A degree's function, which tints its wedge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WedgeFunction {
    Tonic,
    Subdominant,
    Dominant,
}

/// The function of each wedge, in ring order: I – IV – vii° – iii – vi – ii
/// – V (the diatonic cycle of fifths).
pub const WEDGE_FUNCTIONS: [WedgeFunction; WEDGES_IN_WHEEL] = [
    WedgeFunction::Tonic,
    WedgeFunction::Subdominant,
    WedgeFunction::Dominant,
    WedgeFunction::Tonic,
    WedgeFunction::Tonic,
    WedgeFunction::Subdominant,
    WedgeFunction::Dominant,
];

/// How a wedge is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WedgeState {
    /// Faint: part of the key, not playing.
    Idle,
    /// Filled: the lead part's current chord.
    Current,
    /// Outlined: the lead part's next chord.
    Next,
}

/// The wheel, in window pixels measured from the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInstance {
    pub center_x_pixels: f32,
    pub center_y_pixels: f32,
    pub inner_radius_pixels: f32,
    pub outer_radius_pixels: f32,
    /// In ring order, the first wedge (I) centred at the top.
    pub wedge_states: [WedgeState; WEDGES_IN_WHEEL],
    /// Colour by function; off draws every wedge in one neutral tint.
    pub color_by_function: bool,
}

/// Bytes of the uniform the shader reads: viewport (4 floats), geometry
/// (4 floats), then per wedge a colour (4 floats) and a mode (4 floats).
pub const WHEEL_UNIFORM_BYTES: usize = (4 + 4 + WEDGES_IN_WHEEL * 8) * 4;

impl WheelInstance {
    /// The uniform's bytes, in the order of the shader's `Wheel` struct.
    /// Native byte order: the GPU is on the same machine.
    #[must_use]
    pub fn uniform_bytes(&self, width_pixels: f32, height_pixels: f32) -> Vec<u8> {
        let mut floats: Vec<f32> = vec![
            width_pixels,
            height_pixels,
            0.0,
            0.0,
            self.center_x_pixels,
            self.center_y_pixels,
            self.inner_radius_pixels,
            self.outer_radius_pixels,
        ];
        for (wedge, function) in WEDGE_FUNCTIONS.iter().enumerate() {
            let tint: [f32; 4] = if self.color_by_function {
                match function {
                    WedgeFunction::Tonic => theme::WHEEL_TONIC,
                    WedgeFunction::Subdominant => theme::WHEEL_SUBDOMINANT,
                    WedgeFunction::Dominant => theme::WHEEL_DOMINANT,
                }
            } else {
                theme::WHEEL_NEUTRAL
            };
            floats.extend_from_slice(&tint);
            // The shader reads the mode as 0 idle, 1 current, 2 next.
            let mode: f32 = match self.wedge_states[wedge] {
                WedgeState::Idle => 0.0,
                WedgeState::Current => 1.0,
                WedgeState::Next => 2.0,
            };
            floats.extend_from_slice(&[mode, 0.0, 0.0, 0.0]);
        }
        let mut bytes: Vec<u8> = Vec::with_capacity(WHEEL_UNIFORM_BYTES);
        for float in floats {
            bytes.extend_from_slice(&float.to_ne_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(color_by_function: bool) -> WheelInstance {
        WheelInstance {
            center_x_pixels: 100.0,
            center_y_pixels: 100.0,
            inner_radius_pixels: 40.0,
            outer_radius_pixels: 80.0,
            wedge_states: [WedgeState::Idle; WEDGES_IN_WHEEL],
            color_by_function,
        }
    }

    #[test]
    fn uniform_is_the_declared_number_of_bytes() {
        assert_eq!(
            wheel(true).uniform_bytes(2560.0, 1440.0).len(),
            WHEEL_UNIFORM_BYTES
        );
    }

    #[test]
    fn colour_off_makes_every_wedge_the_same_tint() {
        let on: Vec<u8> = wheel(true).uniform_bytes(2560.0, 1440.0);
        let off: Vec<u8> = wheel(false).uniform_bytes(2560.0, 1440.0);
        assert_ne!(on, off);
        // Wedge 0 (tonic) and wedge 1 (subdominant) differ with colour on and
        // match with it off. Wedge n's tint starts at byte 32 + n × 32.
        assert_ne!(on[32..48], on[64..80]);
        assert_eq!(off[32..48], off[64..80]);
    }
}
