//! The words a line element can carry after its colon, and the words of
//! a key: chord names (`ii7`, `bVII7`), pitches (`C2`, `F#2`, `Bb1`),
//! notes in brackets (`[D3 F3 A3 C4]`) and cue names (`"Start SQ-1"`)
//! (Song Notation, sections 2 and 4).

use vocabulary::key::{Key, Mode};
use vocabulary::pitch::{Accidental, NoteLetter, NoteName, Pitch};
use vocabulary::roman_chord::{ChordKind, RomanChord, ScaleDegree};
use vocabulary::song::{ElementContents, VoiceKind};

/// `C2`, `F#2`, `Bb1` → a pitch, checked to be a note a MIDI device has.
///
/// # Errors
///
/// A message saying what is wrong and how to write it.
pub fn parse_pitch(text: &str) -> Result<Pitch, String> {
    let wrong = || {
        format!(
            "\"{text}\" is not a note. Write a letter A to G, then # or b if needed, then \
             the octave, e.g. C2, F#2 or Bb1"
        )
    };
    let mut characters = text.chars();
    let letter: NoteLetter = characters
        .next()
        .and_then(NoteLetter::from_letter)
        .ok_or_else(wrong)?;
    let rest: &str = characters.as_str();
    let (accidental, octave_text): (Accidental, &str) = if let Some(after) = rest.strip_prefix('#')
    {
        (Accidental::Sharp, after)
    } else if let Some(after) = rest.strip_prefix('b') {
        (Accidental::Flat, after)
    } else {
        (Accidental::Natural, rest)
    };
    let octave: i8 = octave_text.parse::<i8>().map_err(|_| wrong())?;
    let pitch: Pitch = Pitch::new(NoteName::new(letter, accidental), octave);
    pitch
        .note_number()
        .map_err(|error| format!("{pitch}: {error}"))?;
    Ok(pitch)
}

/// `[D3 F3 A3 C4]` → pitches.
///
/// # Errors
///
/// When the brackets are wrong, empty, or a note is not a note.
pub fn parse_notes(text: &str) -> Result<Vec<Pitch>, String> {
    let inner: &str = text
        .strip_prefix('[')
        .and_then(|rest: &str| rest.strip_suffix(']'))
        .ok_or_else(|| format!("\"{text}\" is not a list of notes. Write e.g. [D3 F3 A3 C4]"))?;
    let pitches: Vec<Pitch> = inner
        .split_whitespace()
        .map(parse_pitch)
        .collect::<Result<Vec<Pitch>, String>>()?;
    if pitches.is_empty() {
        return Err(String::from(
            "no notes between [ and ]. Write e.g. [D3 F3 A3 C4]",
        ));
    }
    Ok(pitches)
}

/// `ii7`, `bVII7`, `viidim`, `iim7b5` → a chord.
///
/// # Errors
///
/// A message naming the chord and suggesting what to write.
pub fn parse_chord(text: &str) -> Result<RomanChord, String> {
    let not_a_chord = || {
        format!(
            "\"{text}\" is not a chord. Write a Roman numeral, with b or # before it if \
             needed, and what follows, e.g. I, ii7, V7, bVII, vii° or IV7b9"
        )
    };
    let (accidental, after_accidental): (Accidental, &str) =
        if let Some(rest) = text.strip_prefix('b') {
            (Accidental::Flat, rest)
        } else if let Some(rest) = text.strip_prefix('#') {
            (Accidental::Sharp, rest)
        } else {
            (Accidental::Natural, text)
        };
    let numeral_length: usize = after_accidental
        .find(|character: char| !"IiVv".contains(character))
        .unwrap_or(after_accidental.len());
    let (numeral, suffix): (&str, &str) = after_accidental.split_at(numeral_length);
    let degree: ScaleDegree = degree_of(&numeral.to_uppercase()).ok_or_else(not_a_chord)?;
    let is_lower: bool = numeral.chars().all(char::is_lowercase);
    let kind: ChordKind = chord_kind_of(is_lower, suffix).ok_or_else(|| {
        if is_lower {
            format!(
                "\"{text}\" is not a chord. After a lower case numeral write nothing, 7, \
                 ° or dim, °7 or dim7, ø7 or m7b5"
            )
        } else {
            format!(
                "\"{text}\" is not a chord. After an upper case numeral write nothing, 7, \
                 M7 or maj7, + or 7b9"
            )
        }
    })?;
    Ok(RomanChord::new(accidental, degree, kind))
}

fn degree_of(numeral: &str) -> Option<ScaleDegree> {
    match numeral {
        "I" => Some(ScaleDegree::First),
        "II" => Some(ScaleDegree::Second),
        "III" => Some(ScaleDegree::Third),
        "IV" => Some(ScaleDegree::Fourth),
        "V" => Some(ScaleDegree::Fifth),
        "VI" => Some(ScaleDegree::Sixth),
        "VII" => Some(ScaleDegree::Seventh),
        _ => None,
    }
}

/// Lower case numerals are minor-type chords, upper case major-type; the
/// ASCII spellings are accepted for `°` and `ø` (Song Notation, section 2).
fn chord_kind_of(is_lower: bool, suffix: &str) -> Option<ChordKind> {
    if is_lower {
        match suffix {
            "" => Some(ChordKind::Minor),
            "7" => Some(ChordKind::MinorSeventh),
            "°" | "dim" => Some(ChordKind::Diminished),
            "°7" | "dim7" => Some(ChordKind::DiminishedSeventh),
            "ø7" | "ø" | "m7b5" => Some(ChordKind::HalfDiminishedSeventh),
            _ => None,
        }
    } else {
        match suffix {
            "" => Some(ChordKind::Major),
            "7" => Some(ChordKind::DominantSeventh),
            "M7" | "maj7" => Some(ChordKind::MajorSeventh),
            "+" => Some(ChordKind::Augmented),
            "7b9" => Some(ChordKind::DominantSeventhFlatNinth),
            _ => None,
        }
    }
}

/// `C`, `F#`, `Bb` → a note name.
///
/// # Errors
///
/// When the text is not a letter with an optional # or b.
pub fn parse_note_name(text: &str) -> Result<NoteName, String> {
    let wrong = || {
        format!(
            "\"{text}\" is not a key note. Write a letter A to G, then # or b if needed, e.g. C, F# or Bb"
        )
    };
    let mut characters = text.chars();
    let letter: NoteLetter = characters
        .next()
        .and_then(NoteLetter::from_letter)
        .ok_or_else(wrong)?;
    let accidental: Accidental = match characters.as_str() {
        "" => Accidental::Natural,
        "#" => Accidental::Sharp,
        "b" => Accidental::Flat,
        _ => return Err(wrong()),
    };
    Ok(NoteName::new(letter, accidental))
}

/// `C major` → a key. Only major keys exist in V1.
///
/// # Errors
///
/// When the note or the mode is not understood.
pub fn parse_key(tonic_text: &str, mode_text: &str) -> Result<Key, String> {
    let tonic: NoteName = parse_note_name(tonic_text)?;
    match mode_text {
        "major" => Ok(Key::new(tonic, Mode::Major)),
        other => Err(format!(
            "\"{other}\" is not a mode V1 plays. Write e.g. key C major; minor keys and modes \
             are not designed yet"
        )),
    }
}

/// What follows a colon on a voice track, read for the kind of voice.
///
/// # Errors
///
/// When the text is not a chord, note or notes, or does not suit the voice.
pub fn parse_voice_contents(text: &str, voice: VoiceKind) -> Result<ElementContents, String> {
    if text.starts_with('"') {
        return Err(String::from(
            "a quoted name is a cue; cues belong on a cue control track",
        ));
    }
    if text.starts_with('[') {
        return Ok(ElementContents::Notes(parse_notes(text)?));
    }
    if text.starts_with(|character: char| "ABCDEFG".contains(character)) {
        return Ok(ElementContents::Notes(vec![parse_pitch(text)?]));
    }
    if let Some(bracket) = text.find('[') {
        if voice == VoiceKind::Mono {
            return Err(String::from(
                "a mono voice plays one note at a time; write a single note such as C2",
            ));
        }
        return Ok(ElementContents::ChordWithNotes {
            chord: parse_chord(&text[..bracket])?,
            notes: parse_notes(&text[bracket..])?,
        });
    }
    Ok(ElementContents::Chord(parse_chord(text)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitches_are_read_with_accidentals_and_octaves() {
        assert_eq!(parse_pitch("C2").unwrap().to_string(), "C2");
        assert_eq!(parse_pitch("F#2").unwrap().to_string(), "F#2");
        assert_eq!(parse_pitch("Bb1").unwrap().to_string(), "Bb1");
        assert!(parse_pitch("H2").unwrap_err().contains("not a note"));
        assert!(parse_pitch("C").unwrap_err().contains("not a note"));
    }

    #[test]
    fn chords_are_read_by_case_and_suffix() {
        let ii7: RomanChord = parse_chord("ii7").unwrap();
        assert_eq!(ii7.kind(), ChordKind::MinorSeventh);
        assert_eq!(ii7.to_string(), "ii7");
        assert_eq!(
            parse_chord("V7").unwrap().kind(),
            ChordKind::DominantSeventh
        );
        assert_eq!(parse_chord("bVII7").unwrap().to_string(), "bVII7");
        assert_eq!(parse_chord("viidim").unwrap().kind(), ChordKind::Diminished);
        assert_eq!(
            parse_chord("iim7b5").unwrap().kind(),
            ChordKind::HalfDiminishedSeventh
        );
        assert_eq!(parse_chord("I").unwrap().kind(), ChordKind::Major);
        assert!(parse_chord("IV9b").unwrap_err().contains("not a chord"));
        assert!(parse_chord("ii+").unwrap_err().contains("lower case"));
    }

    #[test]
    fn contents_follow_the_kind_of_voice() {
        assert!(matches!(
            parse_voice_contents("ii7", VoiceKind::Poly).unwrap(),
            ElementContents::Chord(_)
        ));
        assert!(matches!(
            parse_voice_contents("[D3 F3 A3]", VoiceKind::Poly).unwrap(),
            ElementContents::Notes(notes) if notes.len() == 3
        ));
        assert!(matches!(
            parse_voice_contents("ii7[F3 A3 C4]", VoiceKind::Poly).unwrap(),
            ElementContents::ChordWithNotes { .. }
        ));
        assert!(matches!(
            parse_voice_contents("C2", VoiceKind::Mono).unwrap(),
            ElementContents::Notes(notes) if notes.len() == 1
        ));
        assert!(parse_voice_contents("ii7[F3]", VoiceKind::Mono).is_err());
    }

    #[test]
    fn keys_need_a_note_and_major() {
        assert_eq!(parse_key("C", "major").unwrap().to_string(), "C major");
        assert_eq!(parse_key("Bb", "major").unwrap().to_string(), "Bb major");
        assert!(
            parse_key("C", "minor")
                .unwrap_err()
                .contains("not designed")
        );
        assert!(
            parse_key("X", "major")
                .unwrap_err()
                .contains("not a key note")
        );
    }
}
