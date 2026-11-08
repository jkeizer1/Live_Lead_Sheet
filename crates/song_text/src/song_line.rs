//! One line of a song section: `line "Keys"  | q:I q q q | qr h:ii7 q:V7 |`
//! becomes a vocabulary `Line` for its track (Song Notation, sections 3, 4
//! and 5). A line may go on over several text lines: a text line starting
//! with `|` adds its bars to the line above.

use vocabulary::rhythm_pattern::{ElementKind, RhythmElement, RhythmPatternError};
use vocabulary::score_length::ScoreLength;
use vocabulary::song::{
    ElementContents, Line, LineElement, SongError, Track, TrackKind, VoiceKind,
};
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_voice_contents;
use crate::length_words::describe_place;
use crate::rhythm_bar::{BarElement, parse_bars};

/// One text line holding bars of a song line: the `line "Keys" | … |` line
/// itself, or a line that continues it, `| … |`.
#[derive(Debug, Clone, Copy)]
pub struct BarsText<'a> {
    pub line_number: usize,
    pub text: &'a str,
}

/// A mistake in a song line, at the file line where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineError {
    pub line_number: usize,
    pub message: String,
}

/// Reads the bars of one line, from all its text lines in order, and checks
/// them against the track. Bars are numbered along the whole line.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead, with the file
/// line of the bar it is in; the caller adds the file name. A mistake about
/// the whole line is placed at `line_number`, where the line starts.
pub fn build_line(
    track: &Track,
    line_number: usize,
    parts: &[BarsText],
    time_signature: TimeSignature,
) -> Result<Line, LineError> {
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let mut bars: Vec<Vec<BarElement>> = Vec::new();
    // The file line each bar is written on, by bar index.
    let mut bar_line_numbers: Vec<usize> = Vec::new();
    for part in parts {
        let part_bars: Vec<Vec<BarElement>> =
            parse_bars(part.text, bars.len(), bar_length_ticks, true).map_err(|message| {
                LineError {
                    line_number: part.line_number,
                    message,
                }
            })?;
        bar_line_numbers.extend(std::iter::repeat_n(part.line_number, part_bars.len()));
        bars.extend(part_bars);
    }
    if bars.is_empty() {
        return Err(LineError {
            line_number,
            message: String::from(
                "this line has no bars. Write them after the name or on the lines below, \
                 e.g. | q:I q q q |",
            ),
        });
    }
    // The file line of the bar holding `start_ticks`. A place past the last
    // bar (an error about the line's end) is on the last bar's line; only a
    // line number for a message, so no value is invented.
    let line_at = |start_ticks: u64| -> usize {
        let bar_index: usize =
            usize::try_from(start_ticks / bar_length_ticks).unwrap_or(usize::MAX);
        bar_line_numbers
            .get(bar_index)
            .or(bar_line_numbers.last())
            .copied()
            .unwrap_or(line_number)
    };
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
            let at = |message: String| LineError {
                line_number: line_at(start_ticks),
                message,
            };
            let place: String = describe_place(start_ticks, time_signature);
            let contents: Option<ElementContents> = contents_of(track, placed, &mut carried)
                .map_err(|why| at(format!("{place}: {why}")))?;
            let element: RhythmElement = RhythmElement::new(
                ScoreLength::from_ticks(start_ticks),
                ScoreLength::from_ticks(placed.length_ticks),
                placed.kind,
                placed.marks,
                placed.tied_to_next,
            )
            .map_err(|error| at(format!("{place}: {error}")))?;
            elements.push(
                LineElement::new(element, contents)
                    .map_err(|error| at(describe_song_error(&error, time_signature)))?,
            );
        }
    }
    let bar_count: u32 = u32::try_from(bars.len()).map_err(|_| LineError {
        line_number,
        message: String::from("this line has too many bars. Split the section"),
    })?;
    Line::new(track, time_signature, bar_count, elements).map_err(|error| LineError {
        line_number: error_start_ticks(&error).map_or(line_number, line_at),
        message: describe_song_error(&error, time_signature),
    })
}

/// Where in the line a whole-line error points, if it names a place.
fn error_start_ticks(error: &SongError) -> Option<u64> {
    match error {
        SongError::ElementWithoutContents { start_ticks, .. }
        | SongError::ContentsNotAllowed { start_ticks, .. }
        | SongError::ContentsOnRest { start_ticks }
        | SongError::Rhythm(RhythmPatternError::TieWithoutNote { start_ticks, .. }) => {
            Some(*start_ticks)
        }
        _ => None,
    }
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
/// beats, rather than ticks.
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

    /// The bars all on the `line` line itself, line 1 of the file.
    fn one(text: &str) -> Vec<BarsText<'_>> {
        vec![BarsText {
            line_number: 1,
            text,
        }]
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
        let line: Line = build_line(
            &bass(),
            1,
            &one("| q:C2 q q q | qr h:D2 q:G2 |"),
            four_four(),
        )
        .unwrap();
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
        let error: String = build_line(&keys(), 1, &one("| q q q q |"), four_four())
            .unwrap_err()
            .message;
        assert!(error.starts_with("bar 1 beat 1: the first note of a line"));
    }

    #[test]
    fn a_chord_on_a_mono_voice_is_explained_at_its_place() {
        let error: String = build_line(&bass(), 1, &one("| q:C2 q q q:ii7 |"), four_four())
            .unwrap_err()
            .message;
        assert!(error.starts_with("bar 1 beat 4:"));
        assert!(error.contains("one note"));
    }

    #[test]
    fn a_short_bar_is_explained_in_note_values() {
        let error: String = build_line(&keys(), 1, &one("| q:I q q |"), four_four())
            .unwrap_err()
            .message;
        assert!(error.contains("bar 1 is short by a quarter note"));
    }

    #[test]
    fn a_line_goes_on_over_several_text_lines_numbering_bars_along_it() {
        let parts: [BarsText; 2] = [
            BarsText {
                line_number: 20,
                text: "| q:C2 q q q | h:G2 h |",
            },
            BarsText {
                line_number: 21,
                text: "| w:A2 | w |",
            },
        ];
        let line: Line = build_line(&bass(), 20, &parts, four_four()).unwrap();
        assert_eq!(line.bar_count(), 4);
        // The last whole note carries A2 from the bar before, across the parts.
        assert!(line.elements().last().unwrap().contents().is_some());
    }

    #[test]
    fn a_mistake_is_placed_at_the_text_line_of_its_bar() {
        // Bar 3 is short: on line 21, named bar 3 of the whole line.
        let short: [BarsText; 2] = [
            BarsText {
                line_number: 20,
                text: "| q:C2 q q q | w |",
            },
            BarsText {
                line_number: 21,
                text: "| q q q |",
            },
        ];
        let error: LineError = build_line(&bass(), 20, &short, four_four()).unwrap_err();
        assert_eq!(error.line_number, 21);
        assert!(
            error.message.starts_with("bar 3 is short"),
            "{}",
            error.message
        );
        // A chord on the bass in bar 3: on line 21 too.
        let chord: [BarsText; 2] = [
            BarsText {
                line_number: 20,
                text: "| q:C2 q q q | w |",
            },
            BarsText {
                line_number: 21,
                text: "| q q:ii7 h |",
            },
        ];
        let error: LineError = build_line(&bass(), 20, &chord, four_four()).unwrap_err();
        assert_eq!(error.line_number, 21);
        assert!(
            error.message.starts_with("bar 3 beat 2:"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_tie_across_the_join_of_two_text_lines_is_one_note() {
        let parts: [BarsText; 2] = [
            BarsText {
                line_number: 5,
                text: "| h:C2 h~ |",
            },
            BarsText {
                line_number: 6,
                text: "| h h |",
            },
        ];
        let line: Line = build_line(&bass(), 5, &parts, four_four()).unwrap();
        assert_eq!(line.bar_count(), 2);
    }

    #[test]
    fn a_line_with_no_bars_says_where_to_write_them() {
        let error: LineError = build_line(&bass(), 7, &[], four_four()).unwrap_err();
        assert_eq!(error.line_number, 7);
        assert!(error.message.starts_with("this line has no bars"));
    }
}
