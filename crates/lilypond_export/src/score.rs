//! A whole song as one LilyPond score.
//!
//! Layout, in the song's track order:
//! - a voice track is a staff in its clef; a grand-staff track is a piano
//!   staff (treble and bass braced), and each of its voice layers goes on
//!   the upper or the lower staff by its median pitch, middle C and above
//!   going up;
//! - voice layers sharing a staff are LilyPond voices, stems up for the
//!   first, down for the second (`\voiceOne`, `\voiceTwo` and so on);
//! - a cue control track is a one-line rhythm staff with each cue's name
//!   above its note; the clock and transport track is not shown;
//! - each section starts with its name as a rehearsal mark (with the
//!   number of passes when more than two), and its key, meter and tempo
//!   where they change; `repeat n` is a LilyPond repeat;
//! - a track with no line in a section rests for its bars;
//! - the lead track's voice layer 1 shows its chords as Roman numerals
//!   above the staff where the chord changes.

use std::collections::HashMap;
use std::fmt::Write;

use note_spelling::note_value::NoteValue;
use note_spelling::spelled_line::{Sound, SpelledBar, SpelledElement, SpelledItem, spell_line};
use vocabulary::key::Key;
use vocabulary::pitch::Pitch;
use vocabulary::roman_chord::RomanChord;
use vocabulary::song::{Clef, LineRole, Section, Song, Track, TrackKind, VoiceLayerNumber};
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;
use vocabulary::time_signature::TimeSignature;

use crate::export_error::ExportError;
use crate::lilypond_words::{duration, key, marks, pitch, quoted};

/// The LilyPond version the output is written for (Linux Mint's package).
const LILYPOND_VERSION: &str = "2.24.0";
/// MIDI note of middle C: a grand-staff voice layer whose median pitch is
/// this or above goes on the upper staff.
const MIDDLE_C_NOTE: u8 = 60;
const MILLI_QUARTERS_PER_QUARTER_NOTE: u64 = 1000;

/// Which line of which track a voice of the score is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LineKey {
    track_index: usize,
    role: LineRole,
}

/// One staff and the voices on it.
struct StaffPlan {
    clef: Option<&'static str>,
    rhythmic: bool,
    voices: Vec<LineKey>,
}

/// One track: one staff, or two braced for a grand staff.
struct TrackPlan {
    name: String,
    piano: bool,
    staves: Vec<StaffPlan>,
}

/// Every line of the song, spelled: by section, then by track and role.
type SpelledSong = Vec<HashMap<LineKey, Vec<SpelledBar>>>;

/// The song as a LilyPond file.
///
/// # Errors
///
/// When a place in the song cannot be spelled yet (for example five notes
/// in the time of a quarter); the message names the section, track and bar.
pub fn song_to_lilypond(song: &Song) -> Result<String, ExportError> {
    let spelled: SpelledSong = spell_song(song)?;
    let plans: Vec<TrackPlan> = plan_tracks(song, &spelled);

    let mut text: String = String::new();
    let _ = writeln!(text, "\\version {}", quoted(LILYPOND_VERSION));
    let _ = writeln!(
        text,
        "% Written by the Live Lead Sheet from the song \"{}\". Engrave with: lilypond FILE.ly",
        song.name().replace('\n', " ")
    );
    let _ = writeln!(
        text,
        "\\header {{\n  title = {}\n  tagline = ##f\n}}\n",
        quoted(song.name())
    );
    text.push_str("\\score {\n  <<\n");
    let mut first_voice: bool = true;
    for plan in &plans {
        write_track(&mut text, song, &spelled, plan, &mut first_voice);
    }
    text.push_str("  >>\n  \\layout { }\n}\n");
    Ok(text)
}

fn spell_song(song: &Song) -> Result<SpelledSong, ExportError> {
    let mut spelled: SpelledSong = Vec::with_capacity(song.sections().len());
    for section in song.sections() {
        let mut lines: HashMap<LineKey, Vec<SpelledBar>> = HashMap::new();
        for line in section.lines() {
            if matches!(line.role(), LineRole::SampleSource(_)) {
                return Err(ExportError::SampleSourceNotDrawn {
                    track_name: line.track_name().to_string(),
                });
            }
            let Some(track_index) = song
                .tracks()
                .iter()
                .position(|track: &Track| track.name() == line.track_name())
            else {
                // A song refuses a line for a track it lacks, so this
                // cannot happen; nothing is drawn for such a line.
                continue;
            };
            let bars: Vec<SpelledBar> = spell_line(&song.tracks()[track_index], section, line)
                .map_err(ExportError::Spelling)?;
            lines.insert(
                LineKey {
                    track_index,
                    role: line.role(),
                },
                bars,
            );
        }
        spelled.push(lines);
    }
    Ok(spelled)
}

fn plan_tracks(song: &Song, spelled: &SpelledSong) -> Vec<TrackPlan> {
    let mut plans: Vec<TrackPlan> = Vec::new();
    for (track_index, track) in song.tracks().iter().enumerate() {
        let roles: Vec<LineRole> = roles_of(track_index, spelled);
        let keys: Vec<LineKey> = roles
            .iter()
            .map(|role: &LineRole| LineKey {
                track_index,
                role: *role,
            })
            .collect();
        match track.kind() {
            TrackKind::ClockAndTransport => {}
            TrackKind::CueControl { .. } => plans.push(TrackPlan {
                name: track.name().to_string(),
                piano: false,
                staves: vec![StaffPlan {
                    clef: None,
                    rhythmic: true,
                    voices: keys_or_first(keys, track_index, LineRole::CuePlacement),
                }],
            }),
            TrackKind::Voice {
                clef: Clef::Grand, ..
            } => {
                let (upper, lower): (Vec<LineKey>, Vec<LineKey>) =
                    keys.into_iter().partition(|line_key: &LineKey| {
                        median_note(line_key, spelled) >= MIDDLE_C_NOTE
                    });
                plans.push(TrackPlan {
                    name: track.name().to_string(),
                    piano: true,
                    staves: vec![
                        StaffPlan {
                            clef: Some("treble"),
                            rhythmic: false,
                            voices: upper,
                        },
                        StaffPlan {
                            clef: Some("bass"),
                            rhythmic: false,
                            voices: lower,
                        },
                    ],
                });
            }
            TrackKind::Voice { clef, .. } => plans.push(TrackPlan {
                name: track.name().to_string(),
                piano: false,
                staves: vec![StaffPlan {
                    clef: Some(clef_word(*clef)),
                    rhythmic: false,
                    voices: keys_or_first(
                        keys,
                        track_index,
                        LineRole::VoiceLayer(VoiceLayerNumber::FIRST),
                    ),
                }],
            }),
        }
    }
    plans
}

/// The roles a track has lines in, in any section: voice layers by number.
fn roles_of(track_index: usize, spelled: &SpelledSong) -> Vec<LineRole> {
    let mut roles: Vec<LineRole> = Vec::new();
    for lines in spelled {
        for line_key in lines.keys() {
            if line_key.track_index == track_index && !roles.contains(&line_key.role) {
                roles.push(line_key.role);
            }
        }
    }
    roles.sort_by_key(|role: &LineRole| match role {
        // A sample source never gets here: `spell_song` refuses it.
        LineRole::VoiceLayer(number) | LineRole::SampleSource(number) => number.value(),
        LineRole::CuePlacement => 0,
    });
    roles
}

/// A track with no line anywhere still gets a staff, resting.
fn keys_or_first(keys: Vec<LineKey>, track_index: usize, role: LineRole) -> Vec<LineKey> {
    if keys.is_empty() {
        vec![LineKey { track_index, role }]
    } else {
        keys
    }
}

/// The median MIDI note of a line's pitches across the song; a line with
/// no pitches counts as middle C, so it goes on the upper staff.
fn median_note(line_key: &LineKey, spelled: &SpelledSong) -> u8 {
    let mut notes: Vec<u8> = Vec::new();
    for lines in spelled {
        let Some(bars) = lines.get(line_key) else {
            continue;
        };
        for element in bars.iter().flat_map(elements_of) {
            if let Sound::Pitches(pitches) = &element.sound {
                notes.extend(
                    pitches
                        .iter()
                        .filter_map(|written: &Pitch| written.note_number().ok())
                        .map(vocabulary::midi_values::NoteNumber::value),
                );
            }
        }
    }
    notes.sort_unstable();
    notes.get(notes.len() / 2).copied().unwrap_or(MIDDLE_C_NOTE)
}

fn elements_of(bar: &SpelledBar) -> Vec<&SpelledElement> {
    bar.items
        .iter()
        .flat_map(|item: &SpelledItem| match item {
            SpelledItem::Single(element) => vec![element],
            SpelledItem::Triplet(elements) => elements.iter().collect(),
        })
        .collect()
}

fn clef_word(clef: Clef) -> &'static str {
    match clef {
        Clef::Treble | Clef::Grand => "treble",
        Clef::Bass => "bass",
        Clef::Alto => "alto",
        Clef::Tenor => "tenor",
        Clef::Percussion => "percussion",
    }
}

fn write_track(
    text: &mut String,
    song: &Song,
    spelled: &SpelledSong,
    plan: &TrackPlan,
    first_voice: &mut bool,
) {
    let name: String = quoted(&plan.name);
    if plan.piano {
        let _ = writeln!(
            text,
            "    \\new PianoStaff \\with {{ instrumentName = {name} }} <<"
        );
        for staff in &plan.staves {
            write_staff(text, song, spelled, staff, None, first_voice);
        }
        text.push_str("    >>\n");
    } else {
        for staff in &plan.staves {
            write_staff(text, song, spelled, staff, Some(&name), first_voice);
        }
    }
}

fn write_staff(
    text: &mut String,
    song: &Song,
    spelled: &SpelledSong,
    staff: &StaffPlan,
    instrument_name: Option<&str>,
    first_voice: &mut bool,
) {
    let context: &str = if staff.rhythmic {
        "RhythmicStaff"
    } else {
        "Staff"
    };
    let with: String = instrument_name
        .map(|name: &str| format!(" \\with {{ instrumentName = {name} }}"))
        .unwrap_or_default();
    let clef: String = staff
        .clef
        .map(|clef: &str| format!("\\clef {clef} "))
        .unwrap_or_default();
    let _ = write!(text, "      \\new {context}{with} {{ {clef}");
    let voice_words: [&str; 4] = ["\\voiceOne", "\\voiceTwo", "\\voiceThree", "\\voiceFour"];
    if staff.voices.is_empty() {
        // A staff of a grand staff with no voice layer of its own: it keeps
        // the key and meter, and stays empty.
        text.push_str(&voice_music(song, spelled, None, staff.rhythmic, false));
    } else if staff.voices.len() == 1 {
        text.push_str(&voice_music(
            song,
            spelled,
            Some(staff.voices[0]),
            staff.rhythmic,
            std::mem::take(first_voice),
        ));
    } else {
        text.push_str("<<\n");
        for (index, line_key) in staff.voices.iter().enumerate() {
            // LilyPond has four voice directions; a fifth layer reuses the
            // fourth's, which only affects stem directions on the page.
            let direction: &str = voice_words[index.min(voice_words.len() - 1)];
            let music: String = voice_music(
                song,
                spelled,
                Some(*line_key),
                staff.rhythmic,
                std::mem::take(first_voice),
            );
            let _ = writeln!(text, "        \\new Voice {{ {direction} {music} }}");
        }
        text.push_str("      >> ");
    }
    text.push_str("}\n");
}

/// The music of one voice across all sections. `line_key` is `None` for a
/// staff that only holds spacers. The score's first voice carries the
/// section marks and tempos.
fn voice_music(
    song: &Song,
    spelled: &SpelledSong,
    line_key: Option<LineKey>,
    rhythmic: bool,
    carries_marks: bool,
) -> String {
    let shows_chords: bool = line_key.is_some_and(|line_key: LineKey| {
        line_key.role == LineRole::VoiceLayer(VoiceLayerNumber::FIRST)
            && matches!(
                song.tracks()[line_key.track_index].kind(),
                TrackKind::Voice { lead: true, .. }
            )
    });
    let first_on_staff: bool = line_key.is_some_and(|line_key: LineKey| match line_key.role {
        LineRole::VoiceLayer(number) | LineRole::SampleSource(number) => {
            number == VoiceLayerNumber::FIRST
        }
        LineRole::CuePlacement => true,
    });
    let mut music: String = String::new();
    let mut previous: Option<&Section> = None;
    let mut slur_open: bool = false;
    for (section_index, section) in song.sections().iter().enumerate() {
        music.push_str(&section_start(section, previous, rhythmic, carries_marks));
        let body: String = match line_key.and_then(|key| spelled[section_index].get(&key)) {
            Some(bars) => bars_text(bars, shows_chords, rhythmic, &mut slur_open),
            None => rest_bars(section, first_on_staff && line_key.is_some()),
        };
        if section.repeat_count() > 1 {
            let _ = write!(
                music,
                "\\repeat volta {} {{ {body} }} ",
                section.repeat_count()
            );
        } else {
            music.push_str(&body);
            music.push(' ');
        }
        previous = Some(section);
    }
    music
}

/// The rehearsal mark, tempo, key and meter a section starts with: the mark
/// and tempo only in the voice that carries them; key and meter where they
/// change (a rhythm staff has no key).
fn section_start(
    section: &Section,
    previous: Option<&Section>,
    rhythmic: bool,
    carries_marks: bool,
) -> String {
    let mut words: String = String::new();
    if carries_marks {
        // Repeat signs say "play again"; more passes than two are written
        // in the mark, as "Gate test (×300)".
        let mark: String = if section.repeat_count() > 2 {
            format!("{} (×{})", section.name(), section.repeat_count())
        } else {
            section.name().to_string()
        };
        let _ = write!(words, "\\mark \\markup \\box {} ", quoted(&mark));
        let tempo_changes: bool =
            previous.is_none_or(|before: &Section| before.tempo() != section.tempo());
        if tempo_changes {
            let _ = write!(words, "{} ", tempo_words(section));
        }
    }
    let previous_key: Option<Key> = previous.and_then(Section::key);
    if !rhythmic
        && let Some(written) = section.key()
        && previous_key != Some(written)
    {
        let _ = write!(words, "{} ", key(written));
    }
    let meter: TimeSignature = section.time_signature();
    if previous.is_none_or(|before: &Section| before.time_signature() != meter) {
        let _ = write!(
            words,
            "\\time {}/{} ",
            meter.pulses_per_bar(),
            meter.pulses_per_whole_note()
        );
    }
    words
}

/// The section's tempo in its meter's beats, as written in the song: beats
/// per minute = (milli-quarters/min × 5040 ticks/q) ÷ (1000 mq/q × ticks/beat).
/// A carried tempo that is not a whole number of beats is shown in quarter
/// notes, and failing that as text.
fn tempo_words(section: &Section) -> String {
    let milli_quarters_per_minute: u64 = section.tempo().milli_quarters_per_minute();
    let beat_ticks: u64 = section.time_signature().beat_length().ticks();
    let numerator: u64 = milli_quarters_per_minute * TICKS_PER_QUARTER_NOTE;
    let denominator: u64 = MILLI_QUARTERS_PER_QUARTER_NOTE * beat_ticks;
    if numerator.is_multiple_of(denominator)
        && let Some(beat) = NoteValue::from_ticks(beat_ticks)
    {
        return format!("\\tempo {} = {}", duration(beat), numerator / denominator);
    }
    if milli_quarters_per_minute.is_multiple_of(MILLI_QUARTERS_PER_QUARTER_NOTE) {
        return format!(
            "\\tempo 4 = {}",
            milli_quarters_per_minute / MILLI_QUARTERS_PER_QUARTER_NOTE
        );
    }
    format!(
        "\\tempo {}",
        quoted(&format!(
            "{}.{:03} quarter notes per minute",
            milli_quarters_per_minute / MILLI_QUARTERS_PER_QUARTER_NOTE,
            milli_quarters_per_minute % MILLI_QUARTERS_PER_QUARTER_NOTE
        ))
    )
}

/// Bars of rest for a section with no line for this voice: a full-bar rest
/// that counts the bars (`R1*12/8` is four bars of 3/8) for a staff's
/// first voice, an invisible spacer for its other voices.
fn rest_bars(section: &Section, visible: bool) -> String {
    let meter: TimeSignature = section.time_signature();
    let pulses: u64 = u64::from(meter.pulses_per_bar()) * u64::from(section.bar_count());
    let rest: char = if visible { 'R' } else { 's' };
    format!("{rest}1*{pulses}/{} |", meter.pulses_per_whole_note())
}

fn bars_text(
    bars: &[SpelledBar],
    shows_chords: bool,
    rhythmic: bool,
    slur_open: &mut bool,
) -> String {
    let mut last_chord: Option<RomanChord> = None;
    let mut text: String = String::new();
    for bar in bars {
        for item in &bar.items {
            match item {
                SpelledItem::Single(element) => {
                    text.push_str(&element_text(
                        element,
                        shows_chords,
                        rhythmic,
                        &mut last_chord,
                        slur_open,
                    ));
                    text.push(' ');
                }
                SpelledItem::Triplet(elements) => {
                    text.push_str("\\tuplet 3/2 { ");
                    for element in elements {
                        text.push_str(&element_text(
                            element,
                            shows_chords,
                            rhythmic,
                            &mut last_chord,
                            slur_open,
                        ));
                        text.push(' ');
                    }
                    text.push_str("} ");
                }
            }
        }
        text.push_str("| ");
    }
    text
}

/// One note, chord or rest: what sounds, its value, marks, tie, slur, and
/// the text above it (a chord numeral or a cue's name).
fn element_text(
    element: &SpelledElement,
    shows_chords: bool,
    rhythmic: bool,
    last_chord: &mut Option<RomanChord>,
    slur_open: &mut bool,
) -> String {
    let head: String = match &element.sound {
        Sound::Rest => String::from("r"),
        // A rhythm staff has one line; its notes need a pitch to be written.
        Sound::Cue(_) => String::from("c"),
        Sound::Pitches(pitches) if rhythmic || pitches.len() == 1 => pitches
            .first()
            .map_or_else(|| String::from("c"), |written: &Pitch| pitch(*written)),
        Sound::Pitches(pitches) => {
            let notes: Vec<String> = pitches
                .iter()
                .map(|written: &Pitch| pitch(*written))
                .collect();
            format!("<{}>", notes.join(" "))
        }
    };
    let mut text: String = format!("{head}{}{}", duration(element.value), marks(element.marks));
    if element.tied_to_next {
        text.push('~');
    }
    let sounds: bool = element.sound != Sound::Rest;
    if *slur_open && !element.marks.legato() {
        text.push(')');
        *slur_open = false;
    } else if sounds && !*slur_open && element.marks.legato() {
        text.push('(');
        *slur_open = true;
    }
    if shows_chords
        && let Some(chord) = element.chord
        && *last_chord != Some(chord)
    {
        let _ = write!(text, "^\\markup {}", quoted(&chord.to_string()));
        *last_chord = Some(chord);
    }
    if let Sound::Cue(name) = &element.sound {
        let _ = write!(text, "^\\markup \\italic {}", quoted(name));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use song_text::song_file::parse_song;

    const HEAD: &str = "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
                        track \"Manual\"\n  type poly_voice\n  clef grand\n  lead\n\
                        track \"Bass\"\n  type mono_voice\n  clef bass\n\
                        track \"Cues\"\n  type cue_control\n  cue \"Go\" destination \"X\" start\n";

    fn lilypond_of(sections: &str) -> String {
        let song: Song = parse_song("test.song", &format!("{HEAD}{sections}")).unwrap();
        song_to_lilypond(&song).unwrap()
    }

    #[test]
    fn voice_layers_share_a_staff_with_their_own_stem_directions() {
        let text: String = lilypond_of(
            "section \"A\"\n  key C major  meter 4/4  tempo 60 beats/min\n\
             \x20 line \"Manual\"  voice_layer 1  | G4:h F4 |\n\
             \x20 line \"Manual\"  voice_layer 2  | Eb4:q D4 C4 D4 |\n\
             \x20 line \"Manual\"  voice_layer 3  | C3:w |\n",
        );
        assert!(text.contains("\\new Voice { \\voiceOne"), "{text}");
        assert!(text.contains("\\new Voice { \\voiceTwo"), "{text}");
        // Layer 3 lies below middle C, so it is alone on the lower staff.
        assert!(
            text.contains("\\clef bass \\key c \\major \\time 4/4 c1"),
            "{text}"
        );
        assert!(text.contains("g'2 f'2"), "{text}");
        assert!(text.contains("es'4 d'4 c'4 d'4"), "{text}");
    }

    #[test]
    fn a_track_with_no_line_rests_and_a_repeat_is_a_volta() {
        let text: String = lilypond_of(
            "section \"A\"  repeat 2\n  key C major  meter 3/8  tempo 60 beats/min\n\
             \x20 line \"Manual\"  | I:q. | / |\n",
        );
        // The bass rests two bars of 3/8.
        assert!(text.contains("R1*6/8"), "{text}");
        assert!(text.contains("\\repeat volta 2 {"), "{text}");
        // 3/8 counts eighth-note beats.
        assert!(text.contains("\\tempo 8 = 60"), "{text}");
        assert!(text.contains("<c' e' g'>4.^\\markup \"I\""), "{text}");
    }

    #[test]
    fn triplets_marks_ties_slurs_and_cues_are_written() {
        let text: String = lilypond_of(
            "section \"A\"\n  key C major  meter 4/4  tempo 60 beats/min\n\
             \x20 line \"Bass\"  | trp (C2:q D2 E2) F2:q_ G2~ | G2:w^ |\n\
             \x20 line \"Cues\"  | \"Go\":h r | r:w |\n",
        );
        assert!(text.contains("\\tuplet 3/2 { c,4 d,4 e,4 }"), "{text}");
        assert!(text.contains("f,4( g,4~)"), "{text}");
        assert!(text.contains("g,1\\fermata"), "{text}");
        assert!(text.contains("c2^\\markup \\italic \"Go\""), "{text}");
    }

    #[test]
    fn a_compound_meter_counts_dotted_quarter_beats() {
        let text: String = lilypond_of(
            "section \"A\"\n  key C major  meter 6/8  tempo 60 beats/min\n\
             \x20 line \"Bass\"  | C2:q. G2 |\n",
        );
        assert!(text.contains("\\tempo 4. = 60"), "{text}");
    }
}
