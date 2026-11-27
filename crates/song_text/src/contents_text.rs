//! The words a line element can carry after its colon, and the words of
//! a key: chord names (`ii7`, `bVII7`), intervals (`P5`, `m3`, `-P4`) on a
//! pitched track, note names (`C2`, `F#2`, `Bb1`) on a MIDI note track,
//! notes in brackets (`[M2 P4 M6]`, `[D3 F3 A3]`) and cue names (`"Start
//! SQ-1"`) (Song Notation, sections 2 and 4).

use vocabulary::interval::{Interval, IntervalDirection, IntervalQuality};
use vocabulary::key::{Key, Mode};
use vocabulary::pitch::{Accidental, NoteLetter, NoteName, Pitch};
use vocabulary::roman_chord::{ChordKind, Inversion, InversionError, RomanChord, ScaleDegree};
use vocabulary::song::{ElementContents, VoiceKind};

/// How a line's notes are written, from its track's `notes` (Song
/// Notation, section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteReading {
    /// `notes midi_note`: note names, `C2`.
    NoteNames,
    /// `notes pitched`: intervals from `tonic`, the section's tonic (as the
    /// entry plays it) in the line's `tonic_octave`. `None` when the line has
    /// no `tonic_octave` or the section no key; a note is then refused with
    /// `missing`, which says what to write.
    Intervals {
        tonic: Option<Pitch>,
        missing: &'static str,
    },
}

/// `P5`, `m3`, `A4`, `M9`, `-P4` → an interval.
///
/// # Errors
///
/// A message saying what is wrong and how to write it.
pub fn parse_interval(text: &str) -> Result<Interval, String> {
    let wrong = || {
        format!(
            "\"{text}\" is not an interval. Write P, M, m, A or d, then the number, with - \
             before it below the tonic, e.g. P1, m3, P5, M9 or -P4"
        )
    };
    let (direction, rest): (IntervalDirection, &str) = match text.strip_prefix('-') {
        Some(after) => (IntervalDirection::Down, after),
        None => (IntervalDirection::Up, text),
    };
    let mut characters = rest.chars();
    let quality: IntervalQuality = characters
        .next()
        .and_then(IntervalQuality::from_letter)
        .ok_or_else(wrong)?;
    let number: u8 = characters.as_str().parse::<u8>().map_err(|_| wrong())?;
    Interval::new(quality, number, direction).map_err(|error| format!("{text}: {error}"))
}

/// A note as the line's track writes it: a note name or an interval.
fn parse_note(text: &str, reading: NoteReading) -> Result<Pitch, String> {
    match reading {
        NoteReading::NoteNames => {
            if parse_interval(text).is_ok() && !text.starts_with('A') {
                return Err(format!(
                    "\"{text}\" is an interval, and this track has notes midi_note. Write a \
                     note name, e.g. C4"
                ));
            }
            parse_pitch(text)
        }
        NoteReading::Intervals { tonic, missing } => {
            let interval: Interval = parse_interval(text).map_err(|why| {
                if parse_pitch(text).is_ok() {
                    format!(
                        "\"{text}\" is a note name, and this track has notes pitched. Write \
                         it as an interval from the section's tonic, e.g. P1, m3 or P5"
                    )
                } else {
                    why
                }
            })?;
            let tonic: Pitch = tonic.ok_or_else(|| String::from(missing))?;
            let pitch: Pitch = interval
                .from_tonic(tonic)
                .map_err(|error| error.to_string())?;
            pitch
                .note_number()
                .map_err(|error| format!("{interval} from {tonic} is {pitch}: {error}"))?;
            Ok(pitch)
        }
    }
}

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

/// `[M2 P4 M6]` or `[D3 F3 A3]` → pitches, as the track writes notes.
///
/// # Errors
///
/// When the brackets are wrong, empty, or a note is not a note.
pub fn parse_notes(text: &str, reading: NoteReading) -> Result<Vec<Pitch>, String> {
    let example: &str = match reading {
        NoteReading::NoteNames => "[D3 F3 A3 C4]",
        NoteReading::Intervals { .. } => "[M2 P4 M6 m7]",
    };
    let inner: &str = text
        .strip_prefix('[')
        .and_then(|rest: &str| rest.strip_suffix(']'))
        .ok_or_else(|| format!("\"{text}\" is not a list of notes. Write e.g. {example}"))?;
    let pitches: Vec<Pitch> = inner
        .split_whitespace()
        .map(|note: &str| parse_note(note, reading))
        .collect::<Result<Vec<Pitch>, String>>()?;
    if pitches.is_empty() {
        return Err(format!("no notes between [ and ]. Write e.g. {example}"));
    }
    Ok(pitches)
}

/// `[1stInversion]` → 1, `[2ndInversion]` → 2, `[3rdInversion]` → 3,
/// `[4thInversion]` → 4. `None` when the brackets hold notes, not an
/// inversion.
fn inversion_number(text: &str) -> Option<Result<u8, String>> {
    let inner: &str = text.strip_prefix('[')?.strip_suffix(']')?.trim();
    let digits: &str = inner.strip_suffix("Inversion")?;
    let number_text: &str = digits
        .strip_suffix("st")
        .or_else(|| digits.strip_suffix("nd"))
        .or_else(|| digits.strip_suffix("rd"))
        .or_else(|| digits.strip_suffix("th"))?;
    Some(number_text.parse::<u8>().map_err(|_| {
        format!(
            "\"{inner}\" is not an inversion. Write e.g. [1stInversion], [2ndInversion], \
             [3rdInversion]"
        )
    }))
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
                "\"{text}\" is not a chord. After a lower case numeral write nothing, 6, 7, 9, \
                 M7 or maj7, ° or dim, °7 or dim7, ø7 or m7b5"
            )
        } else {
            format!(
                "\"{text}\" is not a chord. After an upper case numeral write nothing, 5, 6, 7, \
                 9, 13, M7 or maj7, M9 or maj9, add9, sus2, sus4, 7sus4, 7b9, 7#9, + or +7 \
                 (7#5)"
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
            "6" => Some(ChordKind::MinorSixth),
            "9" => Some(ChordKind::MinorNinth),
            "M7" | "maj7" => Some(ChordKind::MinorMajorSeventh),
            _ => None,
        }
    } else {
        match suffix {
            "" => Some(ChordKind::Major),
            "7" => Some(ChordKind::DominantSeventh),
            "M7" | "maj7" => Some(ChordKind::MajorSeventh),
            "+" => Some(ChordKind::Augmented),
            "7b9" => Some(ChordKind::DominantSeventhFlatNinth),
            "sus2" => Some(ChordKind::SuspendedSecond),
            "sus4" => Some(ChordKind::SuspendedFourth),
            "6" => Some(ChordKind::MajorSixth),
            "9" => Some(ChordKind::DominantNinth),
            "M9" | "maj9" => Some(ChordKind::MajorNinth),
            "add9" => Some(ChordKind::AddedNinth),
            "7sus4" => Some(ChordKind::DominantSeventhSuspendedFourth),
            "7#9" => Some(ChordKind::DominantSeventhSharpNinth),
            "+7" | "7#5" | "aug7" => Some(ChordKind::AugmentedSeventh),
            "5" => Some(ChordKind::PowerChord),
            "13" => Some(ChordKind::DominantThirteenth),
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

/// `C major` or `C minor` → a key.
///
/// # Errors
///
/// When the note or the mode is not understood.
pub fn parse_key(tonic_text: &str, mode_text: &str) -> Result<Key, String> {
    let tonic: NoteName = parse_note_name(tonic_text)?;
    match mode_text {
        "major" => Ok(Key::new(tonic, Mode::Major)),
        "minor" => Ok(Key::new(tonic, Mode::Minor)),
        other => Err(format!(
            "\"{other}\" is not a mode V1 plays. Write key C major or key C minor; a mode such \
             as dorian is an annotation, not part of the key"
        )),
    }
}

/// What follows a colon on a voice track, read for the kind of voice.
///
/// # Errors
///
/// When the text is not a chord, note or notes, or does not suit the voice.
pub fn parse_voice_contents(
    text: &str,
    voice: VoiceKind,
    reading: NoteReading,
) -> Result<ElementContents, String> {
    if text.starts_with('"') {
        return Err(String::from(
            "a quoted name is a cue; cues belong on a cue control track",
        ));
    }
    if text.starts_with('[') {
        return Ok(ElementContents::Notes(parse_notes(text, reading)?));
    }
    // A note name starts with its letter, an interval with its quality or a
    // minus sign; a chord numeral with I, V, b or #.
    if text.starts_with(|character: char| "ABCDEFGPMmd-".contains(character)) {
        return Ok(ElementContents::Notes(vec![parse_note(text, reading)?]));
    }
    if let Some(bracket) = text.find('[') {
        if voice == VoiceKind::Mono {
            return Err(String::from(
                "a mono voice plays one note at a time; write a single note such as C2",
            ));
        }
        let chord: RomanChord = parse_chord(&text[..bracket])?;
        if let Some(number) = inversion_number(&text[bracket..]) {
            let inversion: Inversion = Inversion::new(number?, chord.kind())
                .map_err(|error: InversionError| format!("{}: {error}", &text[..bracket]))?;
            return Ok(ElementContents::InvertedChord { chord, inversion });
        }
        return Ok(ElementContents::ChordWithNotes {
            chord,
            notes: parse_notes(&text[bracket..], reading)?,
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
        let kind_of = |text: &str| parse_chord(text).unwrap().kind();
        assert_eq!(kind_of("Isus2"), ChordKind::SuspendedSecond);
        assert_eq!(kind_of("Vsus4"), ChordKind::SuspendedFourth);
        assert_eq!(kind_of("IV6"), ChordKind::MajorSixth);
        assert_eq!(kind_of("vi6"), ChordKind::MinorSixth);
        assert_eq!(kind_of("V9"), ChordKind::DominantNinth);
        assert_eq!(kind_of("IM9"), ChordKind::MajorNinth);
        assert_eq!(kind_of("Imaj9"), ChordKind::MajorNinth);
        assert_eq!(kind_of("ii9"), ChordKind::MinorNinth);
        assert_eq!(kind_of("Iadd9"), ChordKind::AddedNinth);
        assert_eq!(kind_of("V7sus4"), ChordKind::DominantSeventhSuspendedFourth);
        assert_eq!(kind_of("V7#9"), ChordKind::DominantSeventhSharpNinth);
        assert_eq!(kind_of("V+7"), ChordKind::AugmentedSeventh);
        assert_eq!(kind_of("V7#5"), ChordKind::AugmentedSeventh);
        assert_eq!(kind_of("iM7"), ChordKind::MinorMajorSeventh);
        assert_eq!(kind_of("I5"), ChordKind::PowerChord);
        assert_eq!(kind_of("V13"), ChordKind::DominantThirteenth);
        // A suspended chord has no third, so it has no lower case form.
        assert!(parse_chord("vsus4").is_err());
        assert!(parse_chord("IV9b").unwrap_err().contains("not a chord"));
        assert!(parse_chord("ii+").unwrap_err().contains("lower case"));
    }

    fn in_c(octave: i8) -> NoteReading {
        NoteReading::Intervals {
            tonic: Some(Pitch::new(
                NoteName::new(NoteLetter::C, Accidental::Natural),
                octave,
            )),
            missing: "no tonic",
        }
    }

    fn notes_of(contents: ElementContents) -> Vec<String> {
        match contents {
            ElementContents::Notes(notes) | ElementContents::ChordWithNotes { notes, .. } => {
                notes.iter().map(ToString::to_string).collect()
            }
            other => panic!("not notes: {other:?}"),
        }
    }

    #[test]
    fn contents_follow_the_kind_of_voice() {
        assert!(matches!(
            parse_voice_contents("ii7", VoiceKind::Poly, in_c(3)).unwrap(),
            ElementContents::Chord(_)
        ));
        assert_eq!(
            notes_of(parse_voice_contents("[M2 P4 M6]", VoiceKind::Poly, in_c(3)).unwrap()),
            ["D3", "F3", "A3"]
        );
        assert_eq!(
            notes_of(parse_voice_contents("ii7[P4 M6 P8]", VoiceKind::Poly, in_c(3)).unwrap()),
            ["F3", "A3", "C4"]
        );
        assert_eq!(
            notes_of(parse_voice_contents("C2", VoiceKind::Mono, NoteReading::NoteNames).unwrap()),
            ["C2"]
        );
        assert!(parse_voice_contents("ii7[P4]", VoiceKind::Mono, in_c(3)).is_err());
    }

    #[test]
    fn a_track_takes_only_its_own_notation() {
        // A4 is the augmented fourth on a pitched track, the note on a MIDI note track.
        assert_eq!(
            notes_of(parse_voice_contents("A4", VoiceKind::Mono, in_c(2)).unwrap()),
            ["F#2"]
        );
        assert_eq!(
            notes_of(parse_voice_contents("A4", VoiceKind::Mono, NoteReading::NoteNames).unwrap()),
            ["A4"]
        );
        let name_on_pitched: String =
            parse_voice_contents("C2", VoiceKind::Mono, in_c(2)).unwrap_err();
        assert!(
            name_on_pitched.contains("is a note name"),
            "{name_on_pitched}"
        );
        let interval_on_midi: String =
            parse_voice_contents("P5", VoiceKind::Mono, NoteReading::NoteNames).unwrap_err();
        assert!(
            interval_on_midi.contains("is an interval"),
            "{interval_on_midi}"
        );
        let without_tonic: String = parse_voice_contents(
            "P5",
            VoiceKind::Mono,
            NoteReading::Intervals {
                tonic: None,
                missing: "write tonic_octave",
            },
        )
        .unwrap_err();
        assert_eq!(without_tonic, "write tonic_octave");
    }

    #[test]
    fn intervals_are_read_with_quality_number_and_direction() {
        assert_eq!(parse_interval("P5").unwrap().to_string(), "P5");
        assert_eq!(parse_interval("-P4").unwrap().to_string(), "-P4");
        assert_eq!(parse_interval("M9").unwrap().to_string(), "M9");
        assert!(parse_interval("M5").unwrap_err().contains("perfect"));
        assert!(
            parse_interval("X3")
                .unwrap_err()
                .contains("not an interval")
        );
    }

    #[test]
    fn keys_need_a_note_and_major_or_minor() {
        assert_eq!(parse_key("C", "major").unwrap().to_string(), "C major");
        assert_eq!(parse_key("Bb", "major").unwrap().to_string(), "Bb major");
        assert!(
            parse_key("C", "dorian")
                .unwrap_err()
                .contains("not a mode V1 plays")
        );
        assert_eq!(parse_key("C", "minor").unwrap().to_string(), "C minor");
        assert!(
            parse_key("X", "major")
                .unwrap_err()
                .contains("not a key note")
        );
    }
}
