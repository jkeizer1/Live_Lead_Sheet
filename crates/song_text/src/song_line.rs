//! One line of a song section: `line "Keys"  | I:q / / / | r ii7:h V7:q |`
//! becomes a vocabulary `Line` for its track, in its role (Song Notation,
//! sections 3, 4 and 5). A line may go on over several text lines: a text line starting
//! with `|` adds its bars to the line above.

use vocabulary::rhythm_pattern::{ElementKind, RhythmElement, RhythmPatternError};
use vocabulary::score_length::ScoreLength;
use vocabulary::song::{
    ElementContents, Line, LineElement, LineRole, SongError, Track, TrackKind, VoiceKind,
};
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_voice_contents;
use crate::length_words::describe_place;
use crate::rhythm_bar::{BarElement, BarNotation, LengthCarry, parse_bars, parse_source_bars};

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
/// them against the track and the line's role. Bars are numbered along the
/// whole line, and a length left off carries along it.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead, with the file
/// line of the bar it is in; the caller adds the file name. A mistake about
/// the whole line is placed at `line_number`, where the line starts.
pub fn build_line(
    track: &Track,
    role: LineRole,
    line_number: usize,
    parts: &[BarsText],
    time_signature: TimeSignature,
) -> Result<Line, LineError> {
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let mut bars: Vec<Vec<BarElement>> = Vec::new();
    // The file line each bar is written on, by bar index.
    let mut bar_line_numbers: Vec<usize> = Vec::new();
    let mut carry: LengthCarry = LengthCarry::start_of_line();
    for part in parts {
        // A sample source writes contents and no lengths: its elements share
        // each bar evenly. Every other line writes both.
        let read: Result<Vec<Vec<BarElement>>, String> =
            if matches!(role, LineRole::SampleSource(_)) {
                parse_source_bars(part.text, bars.len(), bar_length_ticks)
            } else {
                parse_bars(
                    part.text,
                    bars.len(),
                    bar_length_ticks,
                    BarNotation::SongLine,
                    &mut carry,
                )
            };
        let part_bars: Vec<Vec<BarElement>> = read.map_err(|message| LineError {
            line_number: part.line_number,
            message,
        })?;
        bar_line_numbers.extend(std::iter::repeat_n(part.line_number, part_bars.len()));
        bars.extend(part_bars);
    }
    if bars.is_empty() {
        return Err(LineError {
            line_number,
            message: String::from(
                "this line has no bars. Write them after the name or on the lines below, \
                 e.g. | I:q / / / |",
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
    // The contents of the last sounding element, for a `/` (Song Notation,
    // section 4). Voices only.
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
    Line::new(track, role, time_signature, bar_count, elements).map_err(|error| LineError {
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
/// A sounding element with no contents is a `/`.
fn contents_of(
    track: &Track,
    placed: &BarElement,
    carried: &mut Option<ElementContents>,
) -> Result<Option<ElementContents>, String> {
    if placed.kind == ElementKind::Rest {
        // The bar parser gives a rest no contents: `r` is all it writes.
        return Ok(None);
    }
    if placed.from_source {
        // `:q` plays what the sample source holds when it starts. It is not
        // a `/`, so what a later `/` repeats is left as it was.
        return match track.kind() {
            TrackKind::Voice { .. } => Ok(None),
            _ => Err(String::from(
                "only a voice track has a sample_source. Name the cue instead, \
                 e.g. \"Start SQ-1\":w",
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
                    "a / plays the chord or notes before it, and none comes before it in \
                    this line. Write what it plays, e.g. I:q or [C3 E3 G3]:q"
                }
                VoiceKind::Mono => {
                    "a / plays the note before it, and none comes before it in this line. \
                    Write the note, e.g. C2:q"
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
                "a cue is placed by its name in quotes. Write \"{text}\":w if that is the \
                 cue's name"
            )),
        },
        (TrackKind::CueControl { .. }, None) => Err(String::from(
            "a / cannot stand on a cue track. Name each cue, e.g. \"Start SQ-1\":w",
        )),
        // A clock track with anything is refused later, for the whole line.
        (TrackKind::ClockAndTransport, _) => Ok(None),
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
            "{}: track \"{track_name}\" needs this note to say what it plays. Write it \
             before the length, e.g. C2:q",
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
        SongError::RestInSampleSource { start_ticks, .. } => format!(
            "{}: a sample_source line holds chords or notes, not rests. Put the rests on the \
             rhythm line below it, e.g. r:q",
            describe_place(*start_ticks, time_signature)
        ),
        SongError::ContentsOnRest { start_ticks } => format!(
            "{}: a rest plays nothing. Write r, or a note instead of the rest",
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

    fn first_layer() -> LineRole {
        LineRole::VoiceLayer(vocabulary::song::VoiceLayerNumber::FIRST)
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
                voice_layer_names: Vec::new(),
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
                voice_layer_names: Vec::new(),
            },
        )
        .unwrap()
    }

    #[test]
    fn a_slash_plays_the_previous_contents() {
        let line: Line = build_line(
            &bass(),
            first_layer(),
            1,
            &one("| C2:q / / / | r D2:h G2:q |"),
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
    fn a_slash_with_nothing_before_it_is_refused() {
        let error: String = build_line(
            &keys(),
            first_layer(),
            1,
            &one("| /:q / / / |"),
            four_four(),
        )
        .unwrap_err()
        .message;
        assert!(
            error.starts_with("bar 1 beat 1: a / plays the chord"),
            "{error}"
        );
    }

    #[test]
    fn a_chord_on_a_mono_voice_is_explained_at_its_place() {
        let error: String = build_line(
            &bass(),
            first_layer(),
            1,
            &one("| C2:q / / ii7 |"),
            four_four(),
        )
        .unwrap_err()
        .message;
        assert!(error.starts_with("bar 1 beat 4:"));
        assert!(error.contains("one note"));
    }

    #[test]
    fn a_short_bar_is_explained_in_note_values() {
        let error: String = build_line(&keys(), first_layer(), 1, &one("| I:q / / |"), four_four())
            .unwrap_err()
            .message;
        assert!(error.contains("bar 1 is short by a quarter note"));
    }

    #[test]
    fn a_line_goes_on_over_several_text_lines_numbering_bars_along_it() {
        let parts: [BarsText; 2] = [
            BarsText {
                line_number: 20,
                text: "| C2:q / / / | G2:h / |",
            },
            BarsText {
                line_number: 21,
                text: "| A2:w | / |",
            },
        ];
        let line: Line = build_line(&bass(), first_layer(), 20, &parts, four_four()).unwrap();
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
                text: "| C2:q / / / | /:w |",
            },
            BarsText {
                line_number: 21,
                text: "| /:q / / |",
            },
        ];
        let error: LineError =
            build_line(&bass(), first_layer(), 20, &short, four_four()).unwrap_err();
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
                text: "| C2:q / / / | /:w |",
            },
            BarsText {
                line_number: 21,
                text: "| /:q ii7 /:h |",
            },
        ];
        let error: LineError =
            build_line(&bass(), first_layer(), 20, &chord, four_four()).unwrap_err();
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
                text: "| C2:h /~ |",
            },
            BarsText {
                line_number: 6,
                text: "| / / |",
            },
        ];
        let line: Line = build_line(&bass(), first_layer(), 5, &parts, four_four()).unwrap();
        assert_eq!(line.bar_count(), 2);
    }

    #[test]
    fn a_line_with_no_bars_says_where_to_write_them() {
        let error: LineError = build_line(&bass(), first_layer(), 7, &[], four_four()).unwrap_err();
        assert_eq!(error.line_number, 7);
        assert!(error.message.starts_with("this line has no bars"));
    }

    #[test]
    fn a_chord_with_its_length_run_on_is_told_to_add_the_colon() {
        let error: String =
            build_line(&keys(), first_layer(), 1, &one("| Iq / / / |"), four_four())
                .unwrap_err()
                .message;
        assert!(
            error.contains(
                "\"Iq\" is not a chord. A chord needs a colon before its length: write I:q"
            ),
            "{error}"
        );
        let dotted: String = build_line(
            &keys(),
            first_layer(),
            1,
            &one("| ii7h. V7:q |"),
            four_four(),
        )
        .unwrap_err()
        .message;
        assert!(dotted.contains("write ii7:h."), "{dotted}");
    }

    #[test]
    fn the_length_carries_across_the_text_lines_of_a_line() {
        let parts: [BarsText; 2] = [
            BarsText {
                line_number: 3,
                text: "| C2:h G2 |",
            },
            BarsText {
                line_number: 4,
                text: "| A2 F2 |",
            },
        ];
        let line: Line = build_line(&bass(), first_layer(), 3, &parts, four_four()).unwrap();
        assert_eq!(line.elements()[3].element().length().ticks(), 10_080);
    }

    #[test]
    fn a_second_voice_layer_is_a_line_of_its_own() {
        let second: LineRole =
            LineRole::VoiceLayer(vocabulary::song::VoiceLayerNumber::new(2).unwrap());
        let line: Line = build_line(&keys(), second, 1, &one("| r:h A4 |"), four_four()).unwrap();
        assert_eq!(line.role(), second);
        let error: String = build_line(&bass(), second, 1, &one("| C2:w |"), four_four())
            .unwrap_err()
            .message;
        assert!(error.contains("only voice layer 1"), "{error}");
    }
}
