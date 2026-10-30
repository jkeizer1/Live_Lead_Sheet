//! What the wheel shader needs: where the wheel is, and what each of its
//! seven wedges shows (EFS-1.10).

use crate::theme;

/// Wedges in the wheel: the seven degrees of a key.
pub const WEDGES_IN_WHEEL: usize = 7;

/// A degree of the key, as a wedge of the wheel. Listed in ring order,
/// clockwise from the top: I – IV – vii° – iii – vi – ii – V (the diatonic
/// cycle of fifths, EFS-1.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelDegree {
    /// I
    One,
    /// IV
    Four,
    /// vii°
    Seven,
    /// iii
    Three,
    /// vi
    Six,
    /// ii
    Two,
    /// V
    Five,
}

impl WheelDegree {
    /// Every degree, in ring order: `RING[wedge]` is that wedge's degree.
    pub const RING: [WheelDegree; WEDGES_IN_WHEEL] = [
        WheelDegree::One,
        WheelDegree::Four,
        WheelDegree::Seven,
        WheelDegree::Three,
        WheelDegree::Six,
        WheelDegree::Two,
        WheelDegree::Five,
    ];

    /// The degree's wedge: its place in ring order, 0 at the top.
    #[must_use]
    pub fn wedge(self) -> usize {
        match self {
            WheelDegree::One => 0,
            WheelDegree::Four => 1,
            WheelDegree::Seven => 2,
            WheelDegree::Three => 3,
            WheelDegree::Six => 4,
            WheelDegree::Two => 5,
            WheelDegree::Five => 6,
        }
    }
}

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

/// The wheel, in window pixels measured from the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInstance {
    pub center_x_pixels: f32,
    pub center_y_pixels: f32,
    pub inner_radius_pixels: f32,
    pub outer_radius_pixels: f32,
    /// The lead part's current chord: its wedge brightened and outlined.
    /// `None` when the chord is outside the key.
    pub current: Option<WheelDegree>,
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
        let current_wedge: Option<usize> = self.current.map(WheelDegree::wedge);
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
            // The shader reads the mode as 0 idle, 1 current.
            let mode: f32 = if current_wedge == Some(wedge) {
                1.0
            } else {
                0.0
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

    /// Where each part of the uniform starts, in bytes (4 bytes per float).
    const FIRST_WEDGE_BYTE: usize = 8 * 4;
    const BYTES_PER_WEDGE: usize = 8 * 4;

    fn wheel(color_by_function: bool) -> WheelInstance {
        WheelInstance {
            center_x_pixels: 100.0,
            center_y_pixels: 100.0,
            inner_radius_pixels: 40.0,
            outer_radius_pixels: 80.0,
            current: Some(WheelDegree::Four),
            color_by_function,
        }
    }

    fn float_at(bytes: &[u8], at: usize) -> f32 {
        f32::from_ne_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    #[test]
    fn the_ring_lists_each_degree_at_its_wedge() {
        for (wedge, degree) in WheelDegree::RING.iter().enumerate() {
            assert_eq!(degree.wedge(), wedge);
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
        // Wedge 0 (tonic) and wedge 2 (dominant) differ with colour on and
        // match with it off.
        let tint = |bytes: &Vec<u8>, wedge: usize| -> Vec<u8> {
            let at: usize = FIRST_WEDGE_BYTE + wedge * BYTES_PER_WEDGE;
            bytes[at..at + 16].to_vec()
        };
        assert_ne!(tint(&on, 0), tint(&on, 2));
        assert_eq!(tint(&off, 0), tint(&off, 2));
    }

    #[test]
    fn only_the_current_wedge_is_marked() {
        let bytes: Vec<u8> = wheel(true).uniform_bytes(2560.0, 1440.0);
        for wedge in 0..WEDGES_IN_WHEEL {
            let mode: f32 = float_at(&bytes, FIRST_WEDGE_BYTE + wedge * BYTES_PER_WEDGE + 16);
            // IV is wedge 1.
            let expected: f32 = if wedge == 1 { 1.0 } else { 0.0 };
            assert!((mode - expected).abs() < f32::EPSILON, "wedge {wedge}");
        }
    }
}
