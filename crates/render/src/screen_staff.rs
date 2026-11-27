//! The staff view, zoomed all the way in: the play strip of EFS-1.5 drawn
//! from the song. Bars run in the order they play, repeats written out
//! (EFS-1.6); each voice track has its rhythm line (slash noteheads) and
//! its pitch staff with clef, key signature and the written notes, and the
//! lead's chord symbols stand above. While playing, the strip moves on by
//! half its length when the playhead passes the middle, so at least half
//! of it is always ahead (EFS-1.7, the "half shift"). While editing, the
//! selected entry starts at the left edge, and a longer one turns whole
//! pages from there: the composer works on one section, left to right.
//!
//! Drawn plainly for this first draft: flags rather than beams, ties as
//! short lines, no collision handling for seconds in a chord.

use composer::key_signature::KeySignature;
use composer::session::{Mode, Session};
use composer::timeline::{
    BarView, ElementView, EntryView, Timeline, TimelinePlace, TrackBarView, TrackView,
};
use note_spelling::note_value::{BaseValue, NoteValue};
use vocabulary::key::Key;
use vocabulary::pitch::{Accidental, NoteLetter, Pitch};
use vocabulary::rhythm_pattern::AccentLevel;
use vocabulary::song::Clef;
use vocabulary::time_signature::TimeSignature;

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::{PlayView, count_as_f32, key_label, key_label_of};
use crate::screen_theme as colors;
use crate::strip_layout::ScreenRect;

const G_CLEF: char = '\u{E050}';
const F_CLEF: char = '\u{E062}';
const PERCUSSION_CLEF: char = '\u{E069}';
const NOTEHEAD_WHOLE: char = '\u{E0A2}';
const NOTEHEAD_HALF: char = '\u{E0A3}';
const NOTEHEAD_BLACK: char = '\u{E0A4}';
const SLASH_BLACK: char = '\u{E100}';
const SLASH_WHOLE: char = '\u{E102}';
const SLASH_HALF: char = '\u{E103}';
const REST_WHOLE: char = '\u{E4E3}';
const REST_HALF: char = '\u{E4E4}';
const REST_QUARTER: char = '\u{E4E5}';
const REST_EIGHTH: char = '\u{E4E6}';
const REST_SIXTEENTH: char = '\u{E4E7}';
const FLAG_EIGHTH_UP: char = '\u{E240}';
const FLAG_EIGHTH_DOWN: char = '\u{E241}';
const FLAG_SIXTEENTH_UP: char = '\u{E242}';
const FLAG_SIXTEENTH_DOWN: char = '\u{E243}';
const FLAT: char = '\u{E260}';
const NATURAL: char = '\u{E261}';
const SHARP: char = '\u{E262}';
const DOT: char = '\u{E1E7}';

/// A pitch staff's space, in screen units.
const STAFF_SPACE_UNITS: f32 = 0.95;
/// Notehead widths in staff spaces (Bravura: noteheadBlack 1.18).
const NOTEHEAD_SPACES: f32 = 1.18;
const STEM_SPACES: f32 = 3.5;
/// Steps from a staff's bottom line: the middle line is 4, the top 8.
const MIDDLE_LINE_STEP: i32 = 4;
const TOP_LINE_STEP: i32 = 8;

/// Least width of a bar, in units: fewer bars fit across a narrow window.
const BAR_MIN_UNITS: f32 = 17.0;
/// Each step of the session's staff bar width makes bars this much wider.
const BAR_WIDTH_STEP_FACTOR: f32 = 1.25;

/// Before the first bar, in staff spaces: the clef (4), up to seven sharps
/// or flats (0.9 each, 6.3), the time signature (2.6) and a gap.
const HEADER_SPACES: f32 = 13.8;
const KEY_SIGNATURE_X_SPACES: f32 = 4.0;
const ACCIDENTAL_STEP_SPACES: f32 = 0.9;

/// Down a track's area, in staff spaces. A sampled track: its sample
/// source row, its rhythm line, the room above the staff for ledger lines
/// and stems, the staff of derived notes. A coupled track: the room for
/// its name and form above the staff, and the staff. Then the room below;
/// a grand staff's second staff below the first.
const SOURCE_ROW_SPACES: f32 = 4.2;
/// From the source row down to the rhythm line: room for the stems, flags
/// and accents above the slashes.
const RHYTHM_LINE_SPACES: f32 = 5.0;
const STAFF_TOP_BELOW_RHYTHM_SPACES: f32 = 6.0;
const COUPLED_STAFF_TOP_SPACES: f32 = 7.0;
const STAFF_HEIGHT_SPACES: f32 = 4.0;
const BELOW_STAFF_SPACES: f32 = 4.5;
const SECOND_STAFF_SPACES: f32 = 11.0;

const TIME_SIGNATURE_DIGIT_ZERO: u32 = 0xE080;

/// One bar of the strip: which entry, which pass, which bar of the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StripBar {
    entry: usize,
    pass_number: u32,
    bar: usize,
}

/// Every bar in the order it plays.
fn play_order(timeline: &Timeline) -> Vec<StripBar> {
    let mut bars: Vec<StripBar> = Vec::new();
    for (entry_index, entry) in timeline.entries.iter().enumerate() {
        if entry.muted {
            continue;
        }
        for pass_number in 1..=entry.repeat_count {
            for bar in 0..entry.bar_count() {
                bars.push(StripBar {
                    entry: entry_index,
                    pass_number,
                    bar,
                });
            }
        }
    }
    bars
}

/// Draws the view.
///
/// # Errors
///
/// When the atlas lacks a glyph.
// A layout reads top to bottom, as the screen does; split up it would
// scatter one picture over several functions.
#[allow(clippy::too_many_lines)]
pub fn draw(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    session: &Session,
    play: &PlayView,
    place: Option<TimelinePlace>,
) -> Result<(), RenderError> {
    let timeline: &Timeline = session.timeline();
    let order: Vec<StripBar> = play_order(timeline);
    if order.is_empty() {
        canvas.text(
            "Nothing plays: every entry is muted.",
            area.left_x_pixels + unit,
            area.top_y_pixels + 3.0 * unit,
            1.4 * unit,
            Align::Left,
            colors::text_soft(),
        )?;
        return Ok(());
    }
    let space: f32 = (STAFF_SPACE_UNITS * unit).round().max(4.0);
    let header_width: f32 = HEADER_SPACES * space;
    let music_left: f32 = area.left_x_pixels + header_width;
    let music_width: f32 = area.right_x_pixels - music_left - unit;
    // An even number of bars, so the half shift moves whole bars.
    let bar_least_width: f32 =
        BAR_MIN_UNITS * unit * BAR_WIDTH_STEP_FACTOR.powi(i32::from(session.staff_bar_width_steps));
    let fit: usize = crate::screen::whole_count(music_width / bar_least_width).max(2);
    let bars_across: usize = (fit / 2 * 2).clamp(2, 16);
    let half: usize = bars_across / 2;
    let current: usize = match (play.playing, place) {
        (true, Some(place)) => order
            .iter()
            .position(|bar| {
                bar.entry == place.entry_index
                    && bar.pass_number == place.pass_number
                    && bar.bar == place.bar_index
            })
            .unwrap_or(0),
        _ => order
            .iter()
            .position(|bar| {
                bar.entry == session.selection.entry
                    && bar.pass_number == session.selection.pass_number
                    && bar.bar == session.selection.first_bar
            })
            .unwrap_or(0),
    };
    // Playing: the half shift. Editing: the selected entry starts at the
    // left edge, so as much of the section as fits is in view and editing
    // runs left to right through it; a section longer than the strip turns
    // whole pages from its start when the selection leaves the view.
    let first: usize = if play.playing {
        current / half * half
    } else {
        let section_start: usize = order
            .iter()
            .position(|bar: &StripBar| bar.entry == session.selection.entry)
            .unwrap_or(current)
            .min(current);
        section_start + (current - section_start) / bars_across * bars_across
    };
    let shown: Vec<StripBar> = order
        .iter()
        .skip(first)
        .take(bars_across)
        .copied()
        .collect();
    let bar_width: f32 = music_width / count_as_f32(bars_across);

    // Each track is rendered in the form it is composed in, bar by bar; a
    // track with a sampled bar in view needs the source and rhythm rows.
    let forms: Vec<FormsInView> = (0..timeline.tracks.len())
        .map(|track_index: usize| forms_in_view(timeline, &shown, track_index))
        .collect();
    // Tracks that fit, the selected and the lead first; the rest are named
    // as hidden, never dropped silently (EFS-1.12).
    let track_height = |track_index: usize| -> f32 {
        let second: f32 = if timeline.tracks[track_index].clef == Clef::Grand {
            SECOND_STAFF_SPACES
        } else {
            0.0
        };
        let above_staff: f32 = if forms[track_index].sampled {
            SOURCE_ROW_SPACES + RHYTHM_LINE_SPACES + STAFF_TOP_BELOW_RHYTHM_SPACES
        } else {
            COUPLED_STAFF_TOP_SPACES
        };
        (above_staff + STAFF_HEIGHT_SPACES + BELOW_STAFF_SPACES + second) * space
    };
    let chord_row: f32 = 3.0 * unit;
    // In the Section Editor, the section's key and the keys its entries
    // transpose it to, above the staves.
    let keys: Option<SectionKeys> = if session.mode == Mode::SectionEditor {
        section_keys(timeline, session.selection.entry)
    } else {
        None
    };
    let keys_height: f32 = keys
        .as_ref()
        .map_or(0.0, |keys: &SectionKeys| keys.height(unit));
    let mut budget: f32 =
        area.bottom_y_pixels - area.top_y_pixels - chord_row - 2.0 * unit - keys_height;
    let selected_track: usize = session.selection.track;
    let mut priority: Vec<usize> = (0..timeline.tracks.len()).collect();
    priority.sort_by_key(|index| {
        (
            *index != selected_track,
            !timeline.tracks[*index].lead,
            *index,
        )
    });
    let mut shown_tracks: Vec<usize> = Vec::new();
    let mut hidden: Vec<&str> = Vec::new();
    for index in priority {
        let height: f32 = track_height(index);
        if height <= budget {
            budget -= height;
            shown_tracks.push(index);
        } else {
            hidden.push(&timeline.tracks[index].name);
        }
    }
    shown_tracks.sort_unstable();
    if !hidden.is_empty() {
        let note: String = format!("Hidden for room: {}", hidden.join(", "));
        canvas.text(
            &note,
            area.right_x_pixels - unit,
            area.bottom_y_pixels - 0.6 * unit,
            1.0 * unit,
            Align::Right,
            colors::problem(),
        )?;
    }

    if let Some(keys) = &keys {
        draw_section_keys(canvas, area, unit, keys)?;
    }
    let top: f32 = area.top_y_pixels + 0.5 * unit + keys_height;
    // Every bar in view that plays the selected section is tinted, the
    // other times through and other entries of it too: an edit changes
    // them all, so the tint shows what belongs together.
    let selected_section: Option<&str> = timeline
        .entries
        .get(session.selection.entry)
        .map(|entry: &EntryView| entry.section_name.as_str());
    let section_bar_lefts: Vec<f32> = shown
        .iter()
        .enumerate()
        .filter(|(_, strip_bar): &(usize, &StripBar)| {
            timeline
                .entries
                .get(strip_bar.entry)
                .map(|entry: &EntryView| entry.section_name.as_str())
                == selected_section
        })
        .map(|(column, _): (usize, &StripBar)| music_left + count_as_f32(column) * bar_width)
        .collect();
    let tint_section_bars =
        |canvas: &mut Canvas<'_>, tint_top: f32, tint_height: f32, color: [f32; 4]| {
            for bar_left in &section_bar_lefts {
                canvas.fill(rect(*bar_left, tint_top, bar_width, tint_height), color);
            }
        };
    tint_section_bars(
        canvas,
        top,
        area.bottom_y_pixels - top,
        colors::section_fill(),
    );
    // Bar numbers, section names at their starts, and the lead's chords.
    for (column, strip_bar) in shown.iter().enumerate() {
        let Some(entry) = timeline.entries.get(strip_bar.entry) else {
            continue;
        };
        let Some(bar) = entry.bars.get(strip_bar.bar) else {
            continue;
        };
        let bar_left: f32 = music_left + count_as_f32(column) * bar_width;
        if strip_bar.bar == 0 {
            let pass: String = if entry.repeat_count > 1 {
                format!("  {} of {}", strip_bar.pass_number, entry.repeat_count)
            } else {
                String::new()
            };
            let label: String = format!("{}{pass}  {}", entry.section_name, key_label(entry));
            // The name has the room of its own bars in view, so it never
            // runs into the next section's name or past the strip.
            let section_bars_in_view: usize = shown[column..]
                .iter()
                .take_while(|other: &&StripBar| {
                    other.entry == strip_bar.entry && other.pass_number == strip_bar.pass_number
                })
                .count();
            canvas.text_fitted(
                &label,
                bar_left + 0.3 * unit,
                top + 0.9 * unit,
                1.0 * unit,
                count_as_f32(section_bars_in_view) * bar_width - 0.6 * unit,
                colors::text_soft(),
            )?;
        }
        for chord in &bar.chords {
            let x: f32 = element_x(
                bar_left,
                bar_width,
                space,
                chord.offset_ticks,
                bar.length_ticks,
            );
            canvas.text(
                &chord.symbol,
                x,
                top + chord_row,
                1.5 * unit,
                Align::Left,
                colors::text(),
            )?;
        }
        canvas.hit(
            rect(bar_left, top, bar_width, chord_row),
            HitTarget::Bar {
                entry: strip_bar.entry,
                bar: strip_bar.bar,
            },
        );
    }
    let mut track_top: f32 = top + chord_row + 0.5 * unit;
    for track_index in shown_tracks {
        let track: &TrackView = &timeline.tracks[track_index];
        let form: FormsInView = forms[track_index];
        let height: f32 = track_height(track_index);
        let selected: bool = (session.mode == Mode::SongHarmony && track.lead)
            || (session.mode != Mode::SongHarmony && track_index == selected_track);
        if selected {
            canvas.fill(
                rect(
                    area.left_x_pixels,
                    track_top,
                    area.right_x_pixels - area.left_x_pixels,
                    height - space,
                ),
                colors::panel_raised(),
            );
            // The selected track's background hides the tint below it.
            tint_section_bars(
                canvas,
                track_top,
                height - space,
                colors::section_fill_raised(),
            );
        }
        canvas.text(
            &track.name,
            area.left_x_pixels + 0.5 * unit,
            track_top + 1.3 * unit,
            1.1 * unit,
            Align::Left,
            if selected {
                colors::text()
            } else {
                colors::text_soft()
            },
        )?;
        canvas.hit(
            rect(area.left_x_pixels, track_top, header_width, height),
            HitTarget::Track(track_index),
        );
        // The composed form, and a control to change it.
        let form_label: &str = match (form.coupled, form.sampled) {
            (true, true) => "coupled + sampled",
            (false, true) => "sampled",
            (true, false) => "coupled",
            (false, false) => "rests",
        };
        let form_area: ScreenRect = rect(
            area.left_x_pixels + 0.3 * unit,
            track_top + 1.7 * unit,
            header_width - 1.0 * unit,
            1.3 * unit,
        );
        canvas.frame(form_area, 1.0, colors::border());
        canvas.text_fitted(
            form_label,
            form_area.left_x_pixels + 0.3 * unit,
            form_area.bottom_y_pixels - 0.35 * unit,
            0.85 * unit,
            form_area.right_x_pixels - form_area.left_x_pixels - 0.6 * unit,
            colors::text_soft(),
        )?;
        canvas.hit(form_area, HitTarget::LineForm(track_index));
        let (source_y, rhythm_y, staff_top_y): (Option<f32>, Option<f32>, f32) = if form.sampled {
            let source_y: f32 = track_top + SOURCE_ROW_SPACES * space;
            let rhythm_y: f32 = source_y + RHYTHM_LINE_SPACES * space;
            (
                Some(source_y),
                Some(rhythm_y),
                rhythm_y + STAFF_TOP_BELOW_RHYTHM_SPACES * space,
            )
        } else {
            (None, None, track_top + COUPLED_STAFF_TOP_SPACES * space)
        };
        let first_bottom_y: f32 = staff_top_y + STAFF_HEIGHT_SPACES * space;
        let layout: TrackLayout = TrackLayout {
            top_y: rhythm_y.map_or(staff_top_y - 2.0 * space, |y: f32| y - 2.5 * space),
            source_y,
            rhythm_y,
            staves: staves_of(track.clef, first_bottom_y, space),
            space,
            music_left,
            bar_width,
            header_left: area.left_x_pixels,
        };
        draw_track(
            canvas,
            &layout,
            timeline,
            &shown,
            track_index,
            session,
            play,
            place,
        )?;
        track_top += height;
    }
    Ok(())
}

/// Which composed forms a track has in the bars in view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FormsInView {
    coupled: bool,
    sampled: bool,
}

fn forms_in_view(timeline: &Timeline, shown: &[StripBar], track_index: usize) -> FormsInView {
    let mut forms: FormsInView = FormsInView {
        coupled: false,
        sampled: false,
    };
    for strip_bar in shown {
        if let Some(track_bar) = timeline
            .entries
            .get(strip_bar.entry)
            .and_then(|entry: &EntryView| entry.bars.get(strip_bar.bar))
            .and_then(|bar: &BarView| bar.tracks.get(track_index))
            .filter(|track_bar: &&TrackBarView| track_bar.has_line)
        {
            if track_bar.sampled {
                forms.sampled = true;
            } else {
                forms.coupled = true;
            }
        }
    }
    forms
}

/// Where a track's lines go.
struct TrackLayout {
    /// The top of the track's music: where a selected note's column starts.
    top_y: f32,
    /// The sample source row's baseline, when a sampled bar is in view.
    source_y: Option<f32>,
    /// The rhythm line, when a sampled bar is in view.
    rhythm_y: Option<f32>,
    staves: Vec<Staff>,
    space: f32,
    music_left: f32,
    bar_width: f32,
    header_left: f32,
}

/// One five-line staff of a track.
#[derive(Debug, Clone, Copy)]
struct Staff {
    clef: Clef,
    /// The bottom line's height.
    bottom_y: f32,
    space: f32,
}

impl Staff {
    fn step_y(&self, step: i32) -> f32 {
        // Steps are half spaces; small counts, exact in f32.
        #[allow(clippy::cast_precision_loss)]
        let steps: f32 = step as f32;
        self.bottom_y - steps * self.space / 2.0
    }

    /// Steps of a pitch above the bottom line: diatonic places from C0,
    /// less the bottom line's.
    fn step_of(&self, pitch: Pitch) -> i32 {
        let place: i32 = i32::from(pitch.octave()) * 7 + letter_index(pitch.name().letter());
        let bottom_line: i32 = match self.clef {
            // E4, G2, F3 (alto: C4 on the middle line), D3 (tenor).
            Clef::Treble | Clef::Percussion | Clef::Grand => 4 * 7 + 2,
            Clef::Bass => 2 * 7 + 4,
            Clef::Alto => 3 * 7 + 3,
            Clef::Tenor => 3 * 7 + 1,
        };
        place - bottom_line
    }
}

/// The selected section's own key and the keys song composition entries
/// play it in, each with the first entry that plays it so, for the Section
/// Editor's header (Editor Draft, "The section's key in F3").
struct SectionKeys {
    own: Key,
    /// The first entry that plays the section in its own key, if any does.
    own_entry: Option<usize>,
    /// Each other key an entry plays it in, once, with the first such entry.
    transpositions: Vec<(Key, usize)>,
    /// The key the selected entry plays it in: the one drawn.
    drawn: Option<Key>,
}

/// Height of one line of the key header.
const KEY_LINE_UNITS: f32 = 2.0;

impl SectionKeys {
    fn height(&self, unit: f32) -> f32 {
        // One line for the section key, one per transposition, and a gap.
        (count_as_f32(1 + self.transpositions.len()) * KEY_LINE_UNITS + 0.6) * unit
    }
}

fn section_keys(timeline: &Timeline, selected_entry: usize) -> Option<SectionKeys> {
    let selected: &EntryView = timeline.entries.get(selected_entry)?;
    let own: Key = selected.section_key?;
    let mut own_entry: Option<usize> = None;
    let mut transpositions: Vec<(Key, usize)> = Vec::new();
    for (index, entry) in timeline.entries.iter().enumerate() {
        if entry.muted || entry.section_name != selected.section_name {
            continue;
        }
        let Some(key) = entry.key else {
            continue;
        };
        if key == own {
            own_entry = own_entry.or(Some(index));
        } else if !transpositions
            .iter()
            .any(|(listed, _): &(Key, usize)| *listed == key)
        {
            transpositions.push((key, index));
        }
    }
    Some(SectionKeys {
        own,
        own_entry,
        transpositions,
        drawn: selected.key,
    })
}

/// `Section Key: C major`, prominent, and below it each transposition; the
/// one drawn is marked, and a click on a line draws the section in that key
/// (it selects the first entry that plays it so).
fn draw_section_keys(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    keys: &SectionKeys,
) -> Result<(), RenderError> {
    let left: f32 = area.left_x_pixels + 0.5 * unit;
    let mut line_top: f32 = area.top_y_pixels + 0.3 * unit;
    let mut lines: Vec<(String, f32, Option<usize>, bool)> = vec![(
        match keys.own_entry {
            Some(_) => format!("Section Key: {}", key_label_of(keys.own)),
            None => format!(
                "Section Key: {}   (every entry transposes it)",
                key_label_of(keys.own)
            ),
        },
        1.35,
        keys.own_entry,
        keys.drawn == Some(keys.own),
    )];
    for (key, entry) in &keys.transpositions {
        lines.push((
            format!("Transposition: {}", key_label_of(*key)),
            1.15,
            Some(*entry),
            keys.drawn == Some(*key),
        ));
    }
    for (text, em_units, entry, drawn) in lines {
        let em: f32 = em_units * unit;
        let width: f32 = canvas.text_width(&text, em) + 1.6 * unit;
        let line: ScreenRect = rect(left, line_top, width, (KEY_LINE_UNITS - 0.2) * unit);
        if drawn {
            canvas.fill(line, colors::selection_fill());
            canvas.frame(line, 1.0, colors::selection_edge());
        }
        canvas.text(
            &text,
            left + 0.8 * unit,
            line_top + 1.35 * unit,
            em,
            Align::Left,
            if drawn {
                colors::text()
            } else {
                colors::text_soft()
            },
        )?;
        if let Some(entry) = entry {
            canvas.hit(line, HitTarget::Bar { entry, bar: 0 });
        }
        line_top += KEY_LINE_UNITS * unit;
    }
    Ok(())
}

fn letter_index(letter: NoteLetter) -> i32 {
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

/// A grand staff is a treble staff over a bass staff; any other clef is
/// one staff.
fn staves_of(clef: Clef, first_bottom_y: f32, space: f32) -> Vec<Staff> {
    match clef {
        Clef::Grand => vec![
            Staff {
                clef: Clef::Treble,
                bottom_y: first_bottom_y,
                space,
            },
            Staff {
                clef: Clef::Bass,
                bottom_y: first_bottom_y + SECOND_STAFF_SPACES * space,
                space,
            },
        ],
        other => vec![Staff {
            clef: other,
            bottom_y: first_bottom_y,
            space,
        }],
    }
}

fn element_x(
    bar_left: f32,
    bar_width: f32,
    space: f32,
    offset_ticks: u64,
    bar_length_ticks: u64,
) -> f32 {
    // Song lengths in ticks are well within f32 for a proportion.
    #[allow(clippy::cast_precision_loss)]
    let fraction: f32 = offset_ticks as f32 / bar_length_ticks.max(1) as f32;
    // Room after the barline for a key signature or accidental.
    bar_left + 2.2 * space + fraction * (bar_width - 3.4 * space)
}

// Each argument is a separate, named quantity of the layout. A layout
// reads top to bottom, as the screen does; split up it would scatter one
// picture over several functions.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn draw_track(
    canvas: &mut Canvas<'_>,
    layout: &TrackLayout,
    timeline: &Timeline,
    shown: &[StripBar],
    track_index: usize,
    session: &Session,
    play: &PlayView,
    place: Option<TimelinePlace>,
) -> Result<(), RenderError> {
    let space: f32 = layout.space;
    let right: f32 = layout.music_left + layout.bar_width * count_as_f32(shown.len());
    let first_entry: Option<&EntryView> = shown
        .first()
        .and_then(|bar| timeline.entries.get(bar.entry));
    let first_key: Option<Key> = first_entry.and_then(|entry| entry.key);
    let first_meter: Option<TimeSignature> = first_entry.and_then(|entry| entry.meter);
    // The sample source row and the rhythm line, for sampled bars: named
    // in the header (the source's origin and play mode), the rhythm line
    // with its percussion clef. Their lines are drawn bar by bar, only
    // where the track is sampled.
    if let Some(source_y) = layout.source_y {
        canvas.text(
            "source: own, static",
            layout.header_left + 0.5 * space,
            source_y,
            0.8 * space,
            Align::Left,
            colors::text_faint(),
        )?;
    }
    if let Some(rhythm_y) = layout.rhythm_y {
        canvas.music(
            PERCUSSION_CLEF,
            layout.header_left + 1.0 * space,
            rhythm_y,
            3.0 * space,
            colors::note(),
        )?;
        if let Some(staff) = layout.staves.last() {
            // The staff shows the notes the rhythm takes from the source,
            // not notes composed on it.
            canvas.text_fitted(
                "notes from source + rhythm",
                layout.header_left + 0.5 * space,
                staff.step_y(-5),
                0.8 * space,
                (HEADER_SPACES - 1.0) * space,
                colors::text_faint(),
            )?;
        }
    }
    for staff in &layout.staves {
        for line in 0..5 {
            let y: f32 = staff.step_y(line * 2);
            canvas.line(
                (layout.header_left + space, y),
                (right, y),
                1.0,
                colors::staff_line(),
            );
        }
        let (clef_glyph, clef_step): (char, i32) = match staff.clef {
            Clef::Bass => (F_CLEF, 6),
            Clef::Percussion => (PERCUSSION_CLEF, 4),
            _ => (G_CLEF, 2),
        };
        canvas.music(
            clef_glyph,
            layout.header_left + 1.0 * space,
            staff.step_y(clef_step),
            4.0 * space,
            colors::note(),
        )?;
        let mut time_x: f32 = layout.header_left + KEY_SIGNATURE_X_SPACES * space;
        if let Some(key) = first_key {
            draw_key_signature(canvas, *staff, key, time_x)?;
            let count: usize = usize::from(KeySignature::of(key).fifths().unsigned_abs());
            time_x += count_as_f32(count) * ACCIDENTAL_STEP_SPACES * space;
        }
        if let Some(meter) = first_meter {
            draw_time_signature(canvas, *staff, meter, time_x + 0.6 * space)?;
        }
    }
    let mut previous_key: Option<Key> = first_key;
    // A repeated entry shows its bars once per pass, but the selection is
    // a bar of the section, so it is marked in one pass only: the pass
    // playing, else the one it was selected in.
    let selected_pass: u32 = match (play.playing, place) {
        (true, Some(place)) if place.entry_index == session.selection.entry => place.pass_number,
        _ => session.selection.pass_number,
    };
    for (column, strip_bar) in shown.iter().enumerate() {
        let Some(entry) = timeline.entries.get(strip_bar.entry) else {
            continue;
        };
        let Some(bar) = entry.bars.get(strip_bar.bar) else {
            continue;
        };
        let bar_left: f32 = layout.music_left + count_as_f32(column) * layout.bar_width;
        let section_start: bool = strip_bar.bar == 0;
        let key_changes: bool = entry.key.is_some() && entry.key != previous_key && column > 0;
        draw_barlines(canvas, layout, bar_left, section_start);
        if key_changes && let Some(key) = entry.key {
            for staff in &layout.staves {
                draw_key_signature(canvas, *staff, key, bar_left + 0.4 * space)?;
            }
        }
        previous_key = entry.key.or(previous_key);
        let Some(track_bar) = bar.tracks.get(track_index) else {
            continue;
        };
        let selected_bar: bool = strip_bar.pass_number == selected_pass
            && session
                .selection
                .contains_bar(strip_bar.entry, strip_bar.bar);
        // A range of bars (Shift+Left/Right) is shaded on the selected
        // track, lighter than the selected note, so both can be seen. In
        // Track Harmony, edited bar by bar, a single bar is shaded too.
        if selected_bar
            && (session.selection.bar_count() > 1 || session.mode == Mode::TrackHarmony)
            && track_index == session.selection.track
        {
            let top: f32 = layout.top_y;
            let bottom: f32 = layout
                .staves
                .last()
                .map_or(layout.top_y, |staff| staff.step_y(-5));
            canvas.fill(
                rect(bar_left, top, layout.bar_width, bottom - top),
                colors::with_alpha(colors::selection_fill(), 0.5),
            );
        }
        draw_bar_music(
            canvas,
            layout,
            entry,
            bar,
            track_bar,
            bar_left,
            strip_bar,
            track_index,
            session,
            selected_bar,
        )?;
        if play.playing
            && let Some(place) = place
            && place.entry_index == strip_bar.entry
            && place.pass_number == strip_bar.pass_number
            && place.bar_index == strip_bar.bar
        {
            let x: f32 = element_x(
                bar_left,
                layout.bar_width,
                space,
                place.offset_ticks,
                bar.length_ticks,
            );
            let bottom: f32 = layout
                .staves
                .last()
                .map_or(layout.top_y, |staff| staff.step_y(-4));
            canvas.line((x, layout.top_y), (x, bottom), 3.0, colors::playhead());
        }
    }
    let end_x: f32 = right;
    draw_barlines(canvas, layout, end_x, false);
    Ok(())
}

fn draw_barlines(canvas: &mut Canvas<'_>, layout: &TrackLayout, x: f32, double: bool) {
    let space: f32 = layout.space;
    if let Some(rhythm_y) = layout.rhythm_y {
        canvas.line(
            (x, rhythm_y - 1.2 * space),
            (x, rhythm_y + 1.2 * space),
            1.5,
            colors::staff_line(),
        );
    }
    for staff in &layout.staves {
        canvas.line(
            (x, staff.step_y(8)),
            (x, staff.step_y(0)),
            1.5,
            colors::staff_line(),
        );
        if double {
            canvas.line(
                (x + 0.4 * space, staff.step_y(8)),
                (x + 0.4 * space, staff.step_y(0)),
                1.5,
                colors::staff_line(),
            );
        }
    }
}

/// The meter's two numbers, centred on the second and fourth lines.
fn draw_time_signature(
    canvas: &mut Canvas<'_>,
    staff: Staff,
    meter: TimeSignature,
    x: f32,
) -> Result<(), RenderError> {
    let em: f32 = 4.0 * staff.space;
    for (number, step) in [
        (meter.pulses_per_bar(), 6),
        (meter.pulses_per_whole_note(), 2),
    ] {
        let mut digit_x: f32 = x;
        for digit in number.to_string().chars() {
            let glyph: char = digit
                .to_digit(10)
                .and_then(|value: u32| char::from_u32(TIME_SIGNATURE_DIGIT_ZERO + value))
                .unwrap_or('\u{E080}');
            canvas.music(glyph, digit_x, staff.step_y(step), em, colors::note())?;
            digit_x += canvas.music_advance(glyph, em);
        }
    }
    Ok(())
}

/// Treble-clef places of each sharp and flat in signature order, in steps
/// from the bottom line (F♯ on the top line, B♭ on the middle line); the
/// bass clef's are two steps lower.
const SHARP_STEPS: [i32; 7] = [8, 5, 9, 6, 3, 7, 4];
const FLAT_STEPS: [i32; 7] = [4, 7, 3, 6, 2, 5, 1];

/// The sharps or flats of a key, in their standard places on the staff.
fn draw_key_signature(
    canvas: &mut Canvas<'_>,
    staff: Staff,
    key: Key,
    x: f32,
) -> Result<(), RenderError> {
    let (accidental, letters) = KeySignature::of(key).drawn_accidentals();
    let clef_shift: i32 = if staff.clef == Clef::Bass { -2 } else { 0 };
    let (glyph, steps): (char, &[i32; 7]) = match accidental {
        Accidental::Flat => (FLAT, &FLAT_STEPS),
        Accidental::Sharp | Accidental::Natural => (SHARP, &SHARP_STEPS),
    };
    for (index, _) in letters.iter().enumerate() {
        let step: i32 = steps[index] + clef_shift;
        canvas.music(
            glyph,
            x + count_as_f32(index) * ACCIDENTAL_STEP_SPACES * staff.space,
            staff.step_y(step),
            4.0 * staff.space,
            colors::note(),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // each is a separate, named quantity of the layout
fn draw_bar_music(
    canvas: &mut Canvas<'_>,
    layout: &TrackLayout,
    entry: &EntryView,
    bar: &BarView,
    track_bar: &TrackBarView,
    bar_left: f32,
    strip_bar: &StripBar,
    track_index: usize,
    session: &Session,
    selected_bar: bool,
) -> Result<(), RenderError> {
    let space: f32 = layout.space;
    if !track_bar.has_line {
        // A track with no line in a section rests: a whole-bar rest.
        let x: f32 = bar_left + layout.bar_width / 2.0 - space;
        for staff in &layout.staves {
            canvas.music(
                REST_WHOLE,
                x,
                staff.step_y(6),
                4.0 * space,
                colors::text_faint(),
            )?;
        }
        return Ok(());
    }
    let signature: Option<KeySignature> = entry.key.map(KeySignature::of);
    let sampled_rows: Option<(f32, f32)> = layout
        .source_y
        .zip(layout.rhythm_y)
        .filter(|_| track_bar.sampled);
    if let Some((source_y, rhythm_y)) = sampled_rows {
        canvas.line(
            (bar_left, rhythm_y),
            (bar_left + layout.bar_width, rhythm_y),
            1.0,
            colors::staff_line(),
        );
        // The source as composed: each chord or note where its even share
        // of the bar starts.
        for part in &track_bar.source {
            let x: f32 = element_x(
                bar_left,
                layout.bar_width,
                space,
                part.offset_ticks,
                bar.length_ticks,
            );
            canvas.text(
                &part.label,
                x,
                source_y,
                1.1 * space,
                Align::Left,
                colors::text_soft(),
            )?;
        }
    }
    for element in &track_bar.elements {
        let x: f32 = element_x(
            bar_left,
            layout.bar_width,
            space,
            element.offset_ticks,
            bar.length_ticks,
        );
        let is_selected: bool = selected_bar
            && session.mode == Mode::SectionEditor
            && track_index == session.selection.track
            && element.line_element_index == session.selection.element;
        let top: f32 = layout.top_y;
        let bottom: f32 = layout
            .staves
            .last()
            .map_or(layout.top_y, |staff| staff.step_y(-5));
        let column: ScreenRect = rect(x - 0.6 * space, top, 2.6 * space, bottom - top);
        if is_selected {
            canvas.fill(column, colors::selection_fill());
            canvas.frame(column, 1.0, colors::selection_edge());
        }
        canvas.hit(
            column,
            HitTarget::Element {
                entry: strip_bar.entry,
                bar: strip_bar.bar,
                track: track_index,
                element: element.line_element_index,
                pass_number: strip_bar.pass_number,
            },
        );
        match sampled_rows {
            Some((_, rhythm_y)) => draw_slash(canvas, element, x, rhythm_y, space)?,
            // Coupled: the marks stand above the staff, over the note.
            None => {
                if let Some(staff) = layout.staves.first() {
                    draw_marks(canvas, element, x, staff.step_y(15), space)?;
                }
            }
        }
        for staff in &layout.staves {
            draw_notes(
                canvas,
                *staff,
                element,
                x,
                signature,
                layout.staves.len() > 1,
            )?;
        }
    }
    Ok(())
}

/// The element on the rhythm line: a slash notehead with its stem and
/// flag, or a rest.
fn draw_slash(
    canvas: &mut Canvas<'_>,
    element: &ElementView,
    x: f32,
    line_y: f32,
    space: f32,
) -> Result<(), RenderError> {
    let rhythm_space: f32 = 0.75 * space;
    let em: f32 = 4.0 * rhythm_space;
    // The rhythm line is composed, so it has the composed notes' colour.
    let color: [f32; 4] = colors::note();
    let base: Option<BaseValue> = element.value.map(NoteValue::base);
    if element.rest {
        draw_rest(canvas, base, x, line_y, em, colors::text_soft())?;
        return Ok(());
    }
    let glyph: char = match base {
        Some(BaseValue::Whole) => SLASH_WHOLE,
        Some(BaseValue::Half) => SLASH_HALF,
        _ => SLASH_BLACK,
    };
    canvas.music(glyph, x, line_y + rhythm_space, em, color)?;
    if base != Some(BaseValue::Whole) {
        let stem_x: f32 = x + if base == Some(BaseValue::Half) {
            3.12
        } else {
            1.46
        } * rhythm_space;
        let stem_top: f32 = line_y - 3.0 * rhythm_space;
        canvas.line(
            (stem_x, line_y - 0.4 * rhythm_space),
            (stem_x, stem_top),
            1.5,
            color,
        );
        draw_flag(canvas, base, stem_x, stem_top, em, true, color)?;
    }
    draw_dots(
        canvas,
        element.value,
        x + 2.0 * rhythm_space,
        line_y - 0.5 * rhythm_space,
        em,
        rhythm_space,
        color,
    )?;
    draw_marks(canvas, element, x, line_y - 3.6 * rhythm_space, space)?;
    Ok(())
}

fn draw_rest(
    canvas: &mut Canvas<'_>,
    base: Option<BaseValue>,
    x: f32,
    y: f32,
    em: f32,
    color: [f32; 4],
) -> Result<(), RenderError> {
    let glyph: char = match base {
        Some(BaseValue::Whole) => REST_WHOLE,
        Some(BaseValue::Half) => REST_HALF,
        Some(BaseValue::Eighth) => REST_EIGHTH,
        Some(BaseValue::Sixteenth | BaseValue::ThirtySecond) => REST_SIXTEENTH,
        Some(BaseValue::Quarter) | None => REST_QUARTER,
    };
    canvas.music(glyph, x, y, em, color)
}

fn draw_flag(
    canvas: &mut Canvas<'_>,
    base: Option<BaseValue>,
    stem_x: f32,
    stem_end_y: f32,
    em: f32,
    up: bool,
    color: [f32; 4],
) -> Result<(), RenderError> {
    let glyph: Option<char> = match (base, up) {
        (Some(BaseValue::Eighth), true) => Some(FLAG_EIGHTH_UP),
        (Some(BaseValue::Eighth), false) => Some(FLAG_EIGHTH_DOWN),
        (Some(BaseValue::Sixteenth | BaseValue::ThirtySecond), true) => Some(FLAG_SIXTEENTH_UP),
        (Some(BaseValue::Sixteenth | BaseValue::ThirtySecond), false) => Some(FLAG_SIXTEENTH_DOWN),
        _ => None,
    };
    if let Some(glyph) = glyph {
        canvas.music(glyph, stem_x - 0.5, stem_end_y, em, color)?;
    }
    Ok(())
}

fn draw_dots(
    canvas: &mut Canvas<'_>,
    value: Option<NoteValue>,
    x: f32,
    y: f32,
    em: f32,
    space: f32,
    color: [f32; 4],
) -> Result<(), RenderError> {
    let dots: u8 = value.map_or(0, NoteValue::dots);
    for dot in 0..dots {
        canvas.music(DOT, x + f32::from(dot) * 0.6 * space, y, em, color)?;
    }
    Ok(())
}

/// Accent marks and the triplet's 3, above the element.
fn draw_marks(
    canvas: &mut Canvas<'_>,
    element: &ElementView,
    x: f32,
    y: f32,
    space: f32,
) -> Result<(), RenderError> {
    let accent: &str = match element.marks.accent() {
        AccentLevel::Accent => ">",
        AccentLevel::Marcato => "^",
        AccentLevel::Normal | AccentLevel::Soft | AccentLevel::Softer => "",
    };
    let mut text: String = accent.to_string();
    if element.triplet {
        text.push_str(" 3");
    }
    if element.marks.staccato() {
        text.push_str(" .");
    }
    if !text.is_empty() {
        canvas.text(
            text.trim(),
            x,
            y,
            1.3 * space,
            Align::Left,
            colors::text_soft(),
        )?;
    }
    Ok(())
}

/// The element's notes on one staff: noteheads with ledger lines and the
/// accidentals the key signature does not give, one stem, a flag, dots.
/// On a grand staff the notes from middle C up go on the treble staff.
// A layout reads top to bottom, as the screen does; split up it would
// scatter one picture over several functions.
#[allow(clippy::too_many_lines)]
fn draw_notes(
    canvas: &mut Canvas<'_>,
    staff: Staff,
    element: &ElementView,
    x: f32,
    signature: Option<KeySignature>,
    grand: bool,
) -> Result<(), RenderError> {
    let space: f32 = staff.space;
    let em: f32 = 4.0 * space;
    let base: Option<BaseValue> = element.value.map(NoteValue::base);
    if element.rest {
        let rest_step: i32 = if base == Some(BaseValue::Whole) {
            6
        } else {
            MIDDLE_LINE_STEP
        };
        return draw_rest(
            canvas,
            base,
            x,
            staff.step_y(rest_step),
            em,
            colors::text_soft(),
        );
    }
    let middle_c: Pitch = Pitch::new(
        vocabulary::pitch::NoteName::new(NoteLetter::C, Accidental::Natural),
        4,
    );
    let on_this_staff = |pitch: &&Pitch| -> bool {
        if !grand {
            return true;
        }
        let at_or_above_middle_c: bool = Staff {
            clef: Clef::Treble,
            bottom_y: 0.0,
            space,
        }
        .step_of(**pitch)
            >= Staff {
                clef: Clef::Treble,
                bottom_y: 0.0,
                space,
            }
            .step_of(middle_c);
        (staff.clef == Clef::Treble) == at_or_above_middle_c
    };
    let pitches: Vec<&Pitch> = element.pitches.iter().filter(on_this_staff).collect();
    if pitches.is_empty() {
        return Ok(());
    }
    let color: [f32; 4] = if element.from_source {
        colors::derived_note()
    } else {
        colors::note()
    };
    let head: char = match base {
        Some(BaseValue::Whole) => NOTEHEAD_WHOLE,
        Some(BaseValue::Half) => NOTEHEAD_HALF,
        _ => NOTEHEAD_BLACK,
    };
    let steps: Vec<i32> = pitches.iter().map(|pitch| staff.step_of(**pitch)).collect();
    for (pitch, step) in pitches.iter().zip(&steps) {
        let y: f32 = staff.step_y(*step);
        // Ledger lines below and above the staff.
        let mut ledger: i32 = -2;
        while ledger >= *step {
            let ledger_y: f32 = staff.step_y(ledger);
            canvas.line(
                (x - 0.4 * space, ledger_y),
                (x + (NOTEHEAD_SPACES + 0.4) * space, ledger_y),
                1.5,
                colors::staff_line(),
            );
            ledger -= 2;
        }
        let mut ledger: i32 = TOP_LINE_STEP + 2;
        while ledger <= *step {
            let ledger_y: f32 = staff.step_y(ledger);
            canvas.line(
                (x - 0.4 * space, ledger_y),
                (x + (NOTEHEAD_SPACES + 0.4) * space, ledger_y),
                1.5,
                colors::staff_line(),
            );
            ledger += 2;
        }
        canvas.music(head, x, y, em, color)?;
        let in_key: Accidental = signature.map_or(Accidental::Natural, |signature| {
            signature.accidental_for(pitch.name().letter())
        });
        if pitch.name().accidental() != in_key {
            let glyph: char = match pitch.name().accidental() {
                Accidental::Flat => FLAT,
                Accidental::Natural => NATURAL,
                Accidental::Sharp => SHARP,
            };
            canvas.music(glyph, x - 1.15 * space, y, em * 0.85, color)?;
        }
        // A dot sits in a space: a note on a line puts it in the space above.
        let dot_y: f32 = if step.rem_euclid(2) == 0 {
            staff.step_y(step + 1)
        } else {
            y
        };
        draw_dots(
            canvas,
            element.value,
            x + (NOTEHEAD_SPACES + 0.35) * space,
            dot_y,
            em,
            space,
            color,
        )?;
    }
    if base != Some(BaseValue::Whole) {
        let lowest: i32 = steps.iter().copied().min().unwrap_or(MIDDLE_LINE_STEP);
        let highest: i32 = steps.iter().copied().max().unwrap_or(MIDDLE_LINE_STEP);
        // Stems go up from notes below the middle line, down from those above.
        let up: bool = lowest + highest < 2 * MIDDLE_LINE_STEP;
        let (stem_x, from_y, to_y): (f32, f32, f32) = if up {
            let x_right: f32 = x + NOTEHEAD_SPACES * space - 0.6;
            (
                x_right,
                staff.step_y(lowest),
                staff.step_y(highest) - STEM_SPACES * space,
            )
        } else {
            (
                x + 0.6,
                staff.step_y(highest),
                staff.step_y(lowest) + STEM_SPACES * space,
            )
        };
        canvas.line((stem_x, from_y), (stem_x, to_y), 1.5, color);
        draw_flag(canvas, base, stem_x, to_y, em, up, color)?;
    }
    if element.tied_to_next {
        let y: f32 = staff.step_y(steps.iter().copied().min().unwrap_or(0) - 2);
        canvas.line(
            (x + NOTEHEAD_SPACES * space, y),
            (x + 3.0 * space, y),
            1.5,
            colors::text_soft(),
        );
    }
    Ok(())
}
