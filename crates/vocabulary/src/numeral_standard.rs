//! How a song's chord numerals find their root in a minor key
//! (`minor_key_treatment_decision.md`, decision 1; Song Notation, "Chord
//! numerals in a minor key"). The two practices count a numeral's step in
//! different scales, so the same numeral can name different roots.

/// The practice a song declares with `chord_numeral_standard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumeralStandard {
    /// Counts the step in the key's own natural minor scale: in C minor the
    /// chord on E♭ is `III`.
    KeyDependent,
    /// Counts the step in the major scale on the tonic, with a flat for a
    /// lowered step: in C minor the chord on E♭ is `bIII`.
    MajorReferential,
}

impl NumeralStandard {
    /// The word a song writes after `chord_numeral_standard`.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            NumeralStandard::KeyDependent => "key_dependent",
            NumeralStandard::MajorReferential => "major_referential",
        }
    }

    /// The practice a song's word names, or nothing for any other word.
    #[must_use]
    pub fn from_text(text: &str) -> Option<NumeralStandard> {
        [
            NumeralStandard::KeyDependent,
            NumeralStandard::MajorReferential,
        ]
        .into_iter()
        .find(|standard: &NumeralStandard| standard.text() == text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_practice_reads_back_from_its_word() {
        assert_eq!(
            NumeralStandard::from_text("key_dependent"),
            Some(NumeralStandard::KeyDependent)
        );
        assert_eq!(
            NumeralStandard::from_text("major_referential"),
            Some(NumeralStandard::MajorReferential)
        );
        assert_eq!(NumeralStandard::from_text("classical"), None);
    }
}
