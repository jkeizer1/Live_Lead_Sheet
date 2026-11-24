//! Moving a written pitch by an interval: by steps of the key's scale
//! (diatonic), by semitones (chromatic), or by octaves. The result is
//! spelled the way the key wants it, so a composer thinks in intervals and
//! the letters follow.

use vocabulary::key::Key;
use vocabulary::pitch::{Accidental, NoteLetter, NoteName, Pitch};

use crate::key_signature::{KeySignature, LETTERS, SEMITONES_PER_OCTAVE, pitch_class};

const LETTERS_PER_OCTAVE: i16 = 7;

fn letter_index(letter: NoteLetter) -> i16 {
    match letter {
        NoteLetter::C => 0,
        NoteLetter::D => 1,
        NoteLetter::E => 2,
        NoteLetter::F => 3,
        NoteLetter::G => 4,
        NoteLetter::A => 5,
        NoteLetter::B => 6,
    }
}

fn letter_at(index: i16) -> NoteLetter {
    // rem_euclid by 7 gives 0 to 6, a valid index into the seven letters.
    LETTERS[usize::try_from(index.rem_euclid(LETTERS_PER_OCTAVE)).unwrap_or(0)]
}

/// Semitones above C0 of a pitch, the same count the MIDI note number
/// makes less 12: octave × 12 semitones/octave + letter + accidental.
fn semitones_above_c0(pitch: Pitch) -> i16 {
    i16::from(pitch.octave()) * SEMITONES_PER_OCTAVE
        + pitch.name().letter().semitones_above_c()
        + pitch.name().accidental().semitone_offset()
}

/// The pitch `steps` scale steps above (below when negative) `pitch`, on the
/// key's own note for the new letter: in C major E4 up one step is F4, in
/// D major E4 up one step is F♯4.
#[must_use]
pub fn diatonic_step(pitch: Pitch, key: Key, steps: i16) -> Pitch {
    let signature: KeySignature = KeySignature::of(key);
    // Letters counted from C0, so moving past B moves the octave.
    let letters_from_c0: i16 = i16::from(pitch.octave()) * LETTERS_PER_OCTAVE
        + letter_index(pitch.name().letter())
        + steps;
    let octave: i16 = letters_from_c0.div_euclid(LETTERS_PER_OCTAVE);
    let letter: NoteLetter = letter_at(letters_from_c0);
    // Octaves of written music stay within −1 to 9; anything beyond is
    // outside MIDI and is refused when the song is read back.
    let octave: i8 = i8::try_from(octave).unwrap_or(if octave < 0 { i8::MIN } else { i8::MAX });
    Pitch::new(
        NoteName::new(letter, signature.accidental_for(letter)),
        octave,
    )
}

/// The pitch `semitones` above (below when negative) `pitch`. A note of the
/// key is spelled as the key spells it; a note outside the key is spelled
/// with a sharp when the move goes up and a flat when it goes down, as a
/// chromatic passing note is written.
#[must_use]
pub fn chromatic_step(pitch: Pitch, key: Key, semitones: i16) -> Pitch {
    let target: i16 = semitones_above_c0(pitch) + semitones;
    let target_class: i16 = target.rem_euclid(SEMITONES_PER_OCTAVE);
    let signature: KeySignature = KeySignature::of(key);
    let in_key: Option<NoteName> = LETTERS
        .iter()
        .map(|letter: &NoteLetter| NoteName::new(*letter, signature.accidental_for(*letter)))
        .find(|name: &NoteName| pitch_class(*name) == target_class);
    let direction: Accidental = if semitones >= 0 {
        Accidental::Sharp
    } else {
        Accidental::Flat
    };
    let spelled_with = |accidental: Accidental| -> Option<NoteName> {
        LETTERS
            .iter()
            .map(|letter: &NoteLetter| NoteName::new(*letter, accidental))
            .find(|name: &NoteName| pitch_class(*name) == target_class)
    };
    // The seven naturals and the seven sharps (or flats) together name all
    // twelve pitch classes, so the chain always finds a name; C is never
    // reached and stands only to end the chain.
    let name: NoteName = in_key
        .or_else(|| spelled_with(Accidental::Natural))
        .or_else(|| spelled_with(direction))
        .unwrap_or_else(|| NoteName::new(NoteLetter::C, Accidental::Natural));
    // The octave is the one whose C lies at or below the target, adjusted
    // for B♯ and C♭, whose letters sit across the octave line.
    let name_semitones: i16 =
        name.letter().semitones_above_c() + name.accidental().semitone_offset();
    let octave: i16 = (target - name_semitones).div_euclid(SEMITONES_PER_OCTAVE);
    let octave: i8 = i8::try_from(octave).unwrap_or(if octave < 0 { i8::MIN } else { i8::MAX });
    Pitch::new(name, octave)
}

/// The same pitch `octaves` octaves higher (lower when negative).
#[must_use]
pub fn octave_step(pitch: Pitch, octaves: i8) -> Pitch {
    Pitch::new(pitch.name(), pitch.octave().saturating_add(octaves))
}

/// The pitch `steps` chord tones above (below when negative) `pitch`: the
/// next note up or down that is one of the chord's `tones`, spelled as the
/// chord spells it. Over a C chord, E2 up one tone is G2 and down one is
/// C2; a note outside the chord goes to the nearest chord tone that way.
/// `None` when `tones` is empty.
#[must_use]
pub fn chord_tone_step(pitch: Pitch, tones: &[NoteName], steps: i16) -> Option<Pitch> {
    if tones.is_empty() {
        return None;
    }
    let here_semitones: i16 = semitones_above_c0(pitch);
    // Every chord tone in the octaves around the note, far enough for the
    // steps asked: a chord has at least one tone per octave.
    let reach_octaves: i16 = i16::try_from(steps.unsigned_abs()).unwrap_or(i16::MAX) + 2;
    let octave: i16 = i16::from(pitch.octave());
    let mut candidates: Vec<(i16, Pitch)> = Vec::new();
    for candidate_octave in
        octave.saturating_sub(reach_octaves)..=octave.saturating_add(reach_octaves)
    {
        // Octaves beyond an i8 are far outside MIDI; none is a candidate.
        let Ok(candidate_octave) = i8::try_from(candidate_octave) else {
            continue;
        };
        for tone in tones {
            let candidate: Pitch = Pitch::new(*tone, candidate_octave);
            candidates.push((semitones_above_c0(candidate), candidate));
        }
    }
    candidates.sort_by_key(|(semitones, _)| *semitones);
    candidates.dedup_by_key(|(semitones, _)| *semitones);
    let count: usize = usize::from(steps.unsigned_abs());
    match steps {
        0 => Some(pitch),
        1.. => candidates
            .iter()
            .filter(|(semitones, _)| *semitones > here_semitones)
            .nth(count - 1)
            .map(|(_, candidate)| *candidate),
        _ => candidates
            .iter()
            .rev()
            .filter(|(semitones, _)| *semitones < here_semitones)
            .nth(count - 1)
            .map(|(_, candidate)| *candidate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::key::Mode;

    fn pitch(letter: NoteLetter, accidental: Accidental, octave: i8) -> Pitch {
        Pitch::new(NoteName::new(letter, accidental), octave)
    }

    fn key(letter: NoteLetter, accidental: Accidental) -> Key {
        Key::new(NoteName::new(letter, accidental), Mode::Major)
    }

    #[test]
    fn diatonic_steps_follow_the_key() {
        let c: Key = key(NoteLetter::C, Accidental::Natural);
        let d: Key = key(NoteLetter::D, Accidental::Natural);
        let e4: Pitch = pitch(NoteLetter::E, Accidental::Natural, 4);
        assert_eq!(diatonic_step(e4, c, 1).to_string(), "F4");
        assert_eq!(diatonic_step(e4, d, 1).to_string(), "F#4");
        assert_eq!(
            diatonic_step(pitch(NoteLetter::B, Accidental::Natural, 3), c, 1).to_string(),
            "C4"
        );
        assert_eq!(
            diatonic_step(pitch(NoteLetter::C, Accidental::Natural, 4), c, -1).to_string(),
            "B3"
        );
        assert_eq!(
            diatonic_step(pitch(NoteLetter::C, Accidental::Natural, 2), c, 4).to_string(),
            "G2"
        );
    }

    #[test]
    fn chromatic_steps_spell_by_key_and_direction() {
        let c: Key = key(NoteLetter::C, Accidental::Natural);
        let f: Key = key(NoteLetter::F, Accidental::Natural);
        let c4: Pitch = pitch(NoteLetter::C, Accidental::Natural, 4);
        assert_eq!(chromatic_step(c4, c, 1).to_string(), "C#4");
        assert_eq!(
            chromatic_step(pitch(NoteLetter::D, Accidental::Natural, 4), c, -1).to_string(),
            "Db4"
        );
        assert_eq!(
            chromatic_step(pitch(NoteLetter::A, Accidental::Natural, 4), f, 1).to_string(),
            "Bb4"
        );
        assert_eq!(chromatic_step(c4, c, -1).to_string(), "B3");
        assert_eq!(
            chromatic_step(pitch(NoteLetter::B, Accidental::Natural, 3), c, 1).to_string(),
            "C4"
        );
        assert_eq!(chromatic_step(c4, c, 7).to_string(), "G4");
    }

    #[test]
    fn chord_tone_steps_walk_the_chord() {
        let tones: Vec<NoteName> = [NoteLetter::C, NoteLetter::E, NoteLetter::G]
            .iter()
            .map(|letter: &NoteLetter| NoteName::new(*letter, Accidental::Natural))
            .collect();
        let e2: Pitch = pitch(NoteLetter::E, Accidental::Natural, 2);
        let step = |from: Pitch, steps: i16| -> String {
            chord_tone_step(from, &tones, steps).map_or_else(String::new, |p| p.to_string())
        };
        assert_eq!(step(e2, 1), "G2");
        assert_eq!(step(e2, -1), "C2");
        assert_eq!(step(e2, 2), "C3");
        assert_eq!(step(pitch(NoteLetter::C, Accidental::Natural, 3), -1), "G2");
        // A note outside the chord goes to the nearest chord tone that way.
        assert_eq!(step(pitch(NoteLetter::F, Accidental::Natural, 2), 1), "G2");
        assert_eq!(step(pitch(NoteLetter::F, Accidental::Natural, 2), -1), "E2");
        assert_eq!(chord_tone_step(e2, &[], 1), None);
    }
}
