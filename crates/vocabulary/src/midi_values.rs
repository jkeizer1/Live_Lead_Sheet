//! The values a phrase cue's MIDI message carries (D-10): channel, note
//! number, velocity, controller number and value, program number. Each is
//! its own type, checked when made, so a cue can never hold a value MIDI
//! cannot send.
//!
//! These are the studio's terms, not an encoding: the ALSA output adapter
//! turns them into bytes (A-3).

use std::fmt;

/// The largest 7-bit data value.
const LARGEST_DATA_VALUE: u8 = 127;

/// A MIDI channel as musicians and devices number it: 1 to 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiChannel {
    channel_number: u8,
}

impl MidiChannel {
    /// # Errors
    ///
    /// When `channel_number` is not 1 to 16.
    pub fn new(channel_number: u8) -> Result<MidiChannel, MidiValueError> {
        check("channel", channel_number, 1, 16)?;
        Ok(MidiChannel { channel_number })
    }

    /// 1 to 16, as shown to the musician.
    #[must_use]
    pub const fn number(self) -> u8 {
        self.channel_number
    }

    /// 0 to 15, as sent in the status byte.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self.channel_number - 1
    }
}

/// A note number, 0 to 127 (60 is middle C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteNumber {
    note_number: u8,
}

impl NoteNumber {
    /// # Errors
    ///
    /// When `note_number` is above 127.
    pub fn new(note_number: u8) -> Result<NoteNumber, MidiValueError> {
        check("note number", note_number, 0, LARGEST_DATA_VALUE)?;
        Ok(NoteNumber { note_number })
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.note_number
    }
}

/// A note-on velocity, 1 to 127. 0 is refused: on the wire a note-on with
/// velocity 0 means note-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Velocity {
    velocity: u8,
}

impl Velocity {
    /// The velocity of a normal note when a studio configuration gives
    /// none: 64, which the MIDI 1.0 specification names as a mezzo-forte
    /// note and as the value for a device without velocity sensing (Jim,
    /// 11 Nov 2026).
    pub const MEZZO_FORTE: Velocity = Velocity { velocity: 64 };

    /// # Errors
    ///
    /// When `velocity` is 0 or above 127.
    pub fn new(velocity: u8) -> Result<Velocity, MidiValueError> {
        check("velocity", velocity, 1, LARGEST_DATA_VALUE)?;
        Ok(Velocity { velocity })
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.velocity
    }

    /// The velocity `eighths` eighths of the way from this one to `limit`,
    /// rounded to the nearest whole velocity (a tie rounds up). Song
    /// Notation, section 6: `>` and `<` are 3 eighths of the way, `>>` and
    /// `<<` 6 eighths (3/4). More than 8 eighths is taken as 8, the limit
    /// itself, so the result is always a valid velocity.
    ///
    /// Units: `(8 eighths/step × velocity ± eighths × room) eighths ÷ 8
    /// eighths/step` → velocity steps, with `room` the steps between this
    /// velocity and the limit.
    #[must_use]
    pub fn eighths_of_the_way_to(self, limit: VelocityLimit, eighths: u8) -> Velocity {
        let from: u32 = u32::from(self.velocity);
        let eighths: u32 = u32::from(eighths).min(EIGHTHS_PER_ONE);
        let moved_eighth_velocities: u32 = match limit {
            VelocityLimit::Loudest => {
                let room: u32 = u32::from(LARGEST_DATA_VALUE) - from;
                EIGHTHS_PER_ONE * from + eighths * room
            }
            VelocityLimit::Softest => {
                let room: u32 = from - u32::from(SMALLEST_VELOCITY);
                EIGHTHS_PER_ONE * from - eighths * room
            }
        };
        // + 4 eighth-velocities is half a velocity: rounds half up.
        let rounded: u32 = (moved_eighth_velocities + EIGHTHS_PER_ONE / 2) / EIGHTHS_PER_ONE;
        // Between the two velocities by construction, so 1 to 127 fits a u8.
        let rounded_velocity: u8 = u8::try_from(rounded).unwrap_or(LARGEST_DATA_VALUE);
        Velocity {
            velocity: rounded_velocity,
        }
    }
}

/// The two ends of the velocity range a mark moves a velocity toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityLimit {
    /// 127.
    Loudest,
    /// 1: velocity 0 means note-off.
    Softest,
}

const SMALLEST_VELOCITY: u8 = 1;
/// Eighths in one whole: 3/8 and 3/4 are written as 3 and 6 eighths.
const EIGHTHS_PER_ONE: u32 = 8;

/// A controller (CC) number, 0 to 127.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControllerNumber {
    controller_number: u8,
}

impl ControllerNumber {
    /// # Errors
    ///
    /// When `controller_number` is above 127.
    pub fn new(controller_number: u8) -> Result<ControllerNumber, MidiValueError> {
        check(
            "controller number",
            controller_number,
            0,
            LARGEST_DATA_VALUE,
        )?;
        Ok(ControllerNumber { controller_number })
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.controller_number
    }
}

/// A controller value, 0 to 127.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControllerValue {
    controller_value: u8,
}

impl ControllerValue {
    /// # Errors
    ///
    /// When `controller_value` is above 127.
    pub fn new(controller_value: u8) -> Result<ControllerValue, MidiValueError> {
        check("controller value", controller_value, 0, LARGEST_DATA_VALUE)?;
        Ok(ControllerValue { controller_value })
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.controller_value
    }
}

/// A program number as sent, 0 to 127. Many devices show it as 1 to 128;
/// the authoring view will say which numbering it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramNumber {
    program_number: u8,
}

impl ProgramNumber {
    /// # Errors
    ///
    /// When `program_number` is above 127.
    pub fn new(program_number: u8) -> Result<ProgramNumber, MidiValueError> {
        check("program number", program_number, 0, LARGEST_DATA_VALUE)?;
        Ok(ProgramNumber { program_number })
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.program_number
    }
}

fn check(what: &'static str, value: u8, smallest: u8, largest: u8) -> Result<(), MidiValueError> {
    if value < smallest || value > largest {
        return Err(MidiValueError {
            what,
            value,
            smallest,
            largest,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiValueError {
    pub what: &'static str,
    pub value: u8,
    pub smallest: u8,
    pub largest: u8,
}

impl fmt::Display for MidiValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MIDI {} {} is out of range. Use {} to {}",
            self.what, self.value, self.smallest, self.largest
        )
    }
}

impl std::error::Error for MidiValueError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_is_numbered_1_to_16_and_sent_as_0_to_15() {
        let channel: MidiChannel = MidiChannel::new(16).unwrap();
        assert_eq!(channel.number(), 16);
        assert_eq!(channel.wire_value(), 15);
        assert!(MidiChannel::new(0).is_err());
        assert!(MidiChannel::new(17).is_err());
    }

    #[test]
    fn velocity_zero_is_refused() {
        assert!(Velocity::new(0).is_err());
        assert_eq!(Velocity::new(127).unwrap().value(), 127);
    }

    #[test]
    fn marks_move_a_velocity_a_share_of_the_room_toward_a_limit() {
        // Song Notation, section 6, the table: normal 64, 90, 100.
        let moved = |normal: u8, limit: VelocityLimit, eighths: u8| -> u8 {
            Velocity::new(normal)
                .unwrap()
                .eighths_of_the_way_to(limit, eighths)
                .value()
        };
        // pp, p, f, ff at normal 64: 17, 40, 88, 111.
        assert_eq!(moved(64, VelocityLimit::Softest, 6), 17);
        assert_eq!(moved(64, VelocityLimit::Softest, 3), 40);
        assert_eq!(moved(64, VelocityLimit::Loudest, 3), 88);
        assert_eq!(moved(64, VelocityLimit::Loudest, 6), 111);
        // normal 90: 23, 57, 104, 118.
        assert_eq!(moved(90, VelocityLimit::Softest, 6), 23);
        assert_eq!(moved(90, VelocityLimit::Softest, 3), 57);
        assert_eq!(moved(90, VelocityLimit::Loudest, 3), 104);
        assert_eq!(moved(90, VelocityLimit::Loudest, 6), 118);
        // normal 100: 26, 63, 110, 120.
        assert_eq!(moved(100, VelocityLimit::Softest, 6), 26);
        assert_eq!(moved(100, VelocityLimit::Softest, 3), 63);
        assert_eq!(moved(100, VelocityLimit::Loudest, 3), 110);
        assert_eq!(moved(100, VelocityLimit::Loudest, 6), 120);
    }

    #[test]
    fn a_mark_never_reaches_the_limit_or_leaves_the_range() {
        for normal in 1..=127_u8 {
            for eighths in [0, 3, 6, 8, 200] {
                for limit in [VelocityLimit::Loudest, VelocityLimit::Softest] {
                    let moved: u8 = Velocity::new(normal)
                        .unwrap()
                        .eighths_of_the_way_to(limit, eighths)
                        .value();
                    assert!((1..=127).contains(&moved), "{normal} {eighths}");
                }
            }
        }
        // 3/4 of the way is short of the limit while there is room.
        let at_100: Velocity = Velocity::new(100).unwrap();
        assert!(
            at_100
                .eighths_of_the_way_to(VelocityLimit::Loudest, 6)
                .value()
                < 127
        );
        // No room: a velocity already at the limit stays there.
        let loudest: Velocity = Velocity::new(127).unwrap();
        assert_eq!(
            loudest
                .eighths_of_the_way_to(VelocityLimit::Loudest, 6)
                .value(),
            127
        );
    }

    #[test]
    fn seven_bit_values_stop_at_127() {
        assert!(NoteNumber::new(128).is_err());
        assert!(ControllerNumber::new(128).is_err());
        assert!(ControllerValue::new(128).is_err());
        assert!(ProgramNumber::new(128).is_err());
    }

    #[test]
    fn error_says_what_and_the_range() {
        assert_eq!(
            MidiChannel::new(17).unwrap_err().to_string(),
            "MIDI channel 17 is out of range. Use 1 to 16"
        );
    }
}
