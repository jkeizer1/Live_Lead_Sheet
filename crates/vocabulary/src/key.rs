//! A key: tonic and mode. V1 has major keys only; minor keys and modes are
//! not designed yet (Song Notation, Q12), so `Mode` has one variant and
//! adding another makes the compiler list every place that must handle it.

use std::fmt;

use crate::pitch::NoteName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Major,
}

impl Mode {
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Mode::Major => "major",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    tonic: NoteName,
    mode: Mode,
}

impl Key {
    #[must_use]
    pub const fn new(tonic: NoteName, mode: Mode) -> Key {
        Key { tonic, mode }
    }

    #[must_use]
    pub const fn tonic(self) -> NoteName {
        self.tonic
    }

    #[must_use]
    pub const fn mode(self) -> Mode {
        self.mode
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.tonic, self.mode.text())
    }
}
