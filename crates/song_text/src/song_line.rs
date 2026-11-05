//! One line of a song section: `line "Keys"  | q:I q q q | qr h:ii7 q:V7 |`
//! becomes a vocabulary `Line` for its track (Song Notation, sections 3, 4
//! and 5).

use vocabulary::rhythm_pattern::{ElementKind, RhythmElement, RhythmPatternError};
use vocabulary::score_length::ScoreLength;
use vocabulary::song::{
    ElementContents, Line, LineElement, SongError, Track, TrackKind, VoiceKind,
};
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_voice_contents;
use crate::length_words::describe_place;
use crate::rhythm_bar::{BarElement, parse_bars};

/// Reads the bars of one line and checks them against the track.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead; the caller adds
/// the file and line number.
pub fn build_line(
    track: &Track,
    bars_text: &str,
    time_signature: TimeSignature,
) -> Result<Line, String> {
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let bars: Vec<Vec<BarElement>> = parse_bars(bars_text, 0, bar_length_ticks, true)?;
    let mut elements: Vec<LineElement> = Vec::new();
    // The contents of the last sounding element, carried to the elements
    // that write none (Song Notation, section 4). Voices only.
    let mut carried: Option<ElementContents> = None;
    for (bar_index, bar) in bars.iter().enumerate() {
        // bar index × ticks/bar → ticks since the start of the line; a line
        // is at most a section's bars, so this cannot overflow.
        let bar_start_ticks: u64 = bar_index as u64 * bar_length_ticks;
        for placed in bar {
            let start_ticks: u64 = bar_start_ticks + placed.start_ticks;
            let place: String = describe_place(start_ticks, time_signature);
            let contents: Option<ElementContents> = contents_of(track, placed, &mut carried)
                .map_err(|why| format!("{place}: {why}"))?;
            let element: RhythmElement = RhythmElement::new(
                ScoreLength::from_ticks(start_ticks),
                ScoreLength::from_ticks(placed.length_ticks),
                placed.kind,
                placed.marks,
                placed.tied_to_next,
            )
            .map_err(|error| format!("{place}: {error}"))?;
            elements.push(
                LineElement::new(element, contents)
                    .map_err(|error| describe_song_error(&error, time_signature))?,
            );
        }
    }
    let bar_count: u32 = u32::try_from(bars.len())
        .map_err(|_| String::from("this line has too many bars. Split the section"))?;
    Line::new(track, time_signature, bar_count, elements)
        .map_err(|error| describe_song_error(&error, time_signature))
}

/// What one element carries, by the kind of track; `carried` is updated.
fn contents_of(
    track: &Track,
    placed: &BarElement,
    carried: &mut Option<ElementContents>,
) -> Result<Option<ElementContents>, String> {
    if placed.kind == ElementKind::Rest {
        return match &placed.contents_text {
            None => Ok(None),
            Some(text) => Err(format!(
                "a rest carries nothing. Take :{text} off, or write a note instead of the rest"
            )),
        };
    }
    match (track.kind(), &placed.contents_text) {
        (TrackKind::Voice { voice, .. }, Some(text)) => {
            let contents: ElementContents = parse_voice_contents(text, *voice)?;
            *carried = Some(contents.clone());
            Ok(Some(contents))
        }
        (TrackKind::Voice { voice, .. }, None) => carried.clone().map(Some).ok_or_else(|| {
            match voice {
                VoiceKind::Poly => {
                    "the first note of a line must say what it plays: write a colon \
                    and a chord or notes, e.g. q:I or q:[C3 E3 G3]"
                }
                VoiceKind::Mono => {
                    "the first note of a line must say what it plays: write a colon \
                    and a note, e.g. q:C2"
                }
            }
            .to_string()
        }),
        (TrackKind::CueControl { .. }, Some(text)) => match text
            .strip_prefix('"')
            .and_then(|rest: &str| rest.strip_suffix('"'))
        {
            Some(name) => Ok(Some(ElementContents::Cue(name.to_string()))),
            None => Err(format!(
                "a cue is placed by its name in quotes. Write :\"{text}\" if that is the cue's name"
            )),
        },
        // A clock track with anything is refused later, for the whole line.
        (TrackKind::CueControl { .. }, None) | (TrackKind::ClockAndTransport, _) => Ok(None),
    }
}

/// A song error in the musician's words: where in the line, in bars and
/// counts, rather than ticks.
fn describe_song_error(error: &SongError, time_signature: TimeSignature) -> String {
    match error {
        SongError::ElementWithoutContents {
            track_name,
            start_ticks,
        } => format!(
            "{}: track \"{track_name}\" needs this note to say what it plays. Write a colon and \
             what it plays, e.g. q:C2",
            describe_place(*start_ticks, time_signature)
        ),
        SongError::ContentsNotAllowed {
            track_name,
            start_ticks,
            why,
        } => format!(
            "{}: track \"{track_name}\" cannot play that: {why}",
            describe_place(*start_ticks, time_signature)
        ),
        SongError::ContentsOnRest { start_ticks } => format!(
            "{}: a rest carries nothing. Take the part after the colon off",
            describe_place(*start_ticks, time_signature)
        ),
        SongError::Rhythm(RhythmPatternError::TieWithoutNote { start_ticks, .. }) => format!(
            "the tie at {} has no note after it to join. Remove the ~ or put a note after it",
            describe_place(*start_ticks, time_signature)
        ),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::song::Clef;

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    fn keys() -> Track {
        Track::new(
            "Keys",
            TrackKind::Voice {
                voice: VoiceKind::Poly,
                clef: Clef::Grand,
                lead: true,
                home_octave: None,
            },
        )
        .unwrap()
    }

    fn bass() -> Track {
        Track::new(
            "Bass",
            TrackKind::Voice {
                voice: VoiceKind::Mono,
                clef: Clef::Bass,
                lead: false,
                home_octave: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn an_element_with_nothing_carries_the_previous_contents() {
        let line: Line = build_line(&bass(), "| q:C2 q q q | qr h:D2 q:G2 |", four_four()).unwrap();
        assert_eq!(line.bar_count(), 2);
        // Six sounding notes carry the chord or note; the rest carries nothing.
        let carrying: usize = line
            .elements()
            .iter()
            .filter(|element| element.contents().is_some())
            .count();
        assert_eq!(carrying, 6);
    }

    #[test]
    fn the_first_note_of_a_line_must_say_what_it_plays() {
        let error: String = build_line(&keys(), "| q q q q |", four_four()).unwrap_err();
        assert!(error.starts_with("bar 1 count 1: the first note of a line"));
    }

    #[test]
    fn a_chord_on_a_mono_voice_is_explained_at_its_place() {
        let error: String = build_line(&bass(), "| q:C2 q q q:ii7 |", four_four()).unwrap_err();
        assert!(error.starts_with("bar 1 count 4:"));
        assert!(error.contains("one note"));
    }

    #[test]
    fn a_short_bar_is_explained_in_counts() {
        let error: String = build_line(&keys(), "| q:I q q |", four_four()).unwrap_err();
        assert!(error.contains("bar 1 is short by a quarter note"));
    }
}
