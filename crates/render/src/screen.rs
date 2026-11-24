//! The editor's screen, one frame at a time, from the session it is handed
//! (Concept Brief, "Authoring view"; EFS-5). Top to bottom:
//!
//! - the header: the song, the five modes, what an edit changes now, and
//!   the counter while playing;
//! - the form strip: every entry of the composition in order, as wide as
//!   it lasts, on a lane coloured by its key, with the playhead (EFS-1.2);
//! - the main view the mode renders (form, harmony, tracks, staff), with
//!   the library browser beside it;
//! - the message line and the keys the current mode offers.
//!
//! Every size is in units of 1/90 of the window's height, so the screen
//! scales with the window (EFS-1.12).

use composer::repair::Repair;
use composer::session::{MessageKind, Mode, Pane, Session, Zoom};
use composer::shell::Shell;
use composer::timeline::{EntryView, Timeline, TimelinePlace};

use crate::canvas::{Align, Canvas, Frame, HitTarget, MessageSource, rect};
use crate::error::RenderError;
use crate::glyph_atlas::GlyphAtlas;
use crate::screen_theme as colors;
use crate::strip_layout::ScreenRect;
use crate::{
    screen_form, screen_harmony, screen_library, screen_repair, screen_shell, screen_staff,
    screen_tracks,
};

/// What the player is doing, as the app knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayView {
    pub playing: bool,
    pub looping: bool,
    /// Ticks from the start of the song being played.
    pub position_ticks: Option<u64>,
    /// About the MIDI output, in a few words: "3 ports open", or why
    /// playing is off.
    pub status: String,
}

/// What fills the screen below the menu bar.
pub enum Content<'a> {
    /// No song is open.
    Empty,
    /// A song or its studio file could not be read: the problems to repair.
    Repair(&'a Repair),
    Song(&'a Session),
}

/// Window height ÷ 90: the screen's unit of size.
const UNITS_PER_HEIGHT: f32 = 90.0;
/// The menu bar above the header.
pub const MENU_BAR_UNITS: f32 = 2.6;
const HEADER_UNITS: f32 = 3.4;
const FORM_STRIP_UNITS: f32 = 6.6;
const FOOTER_UNITS: f32 = 6.6;
const LIBRARY_FRACTION_OF_WIDTH: f32 = 0.24;
const MARGIN_UNITS: f32 = 0.8;

/// Where each part of the screen goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenLayout {
    pub unit_pixels: f32,
    pub menu_bar: ScreenRect,
    pub header: ScreenRect,
    pub form_strip: ScreenRect,
    pub main: ScreenRect,
    pub library: ScreenRect,
    pub footer: ScreenRect,
}

impl ScreenLayout {
    #[must_use]
    pub fn new(width_pixels: f32, height_pixels: f32) -> ScreenLayout {
        let unit_pixels: f32 = (height_pixels / UNITS_PER_HEIGHT).round().max(6.0);
        let margin: f32 = MARGIN_UNITS * unit_pixels;
        let menu_bar_height: f32 = MENU_BAR_UNITS * unit_pixels;
        let header_height: f32 = HEADER_UNITS * unit_pixels;
        let strip_height: f32 = FORM_STRIP_UNITS * unit_pixels;
        let footer_height: f32 = FOOTER_UNITS * unit_pixels;
        let library_width: f32 = (width_pixels * LIBRARY_FRACTION_OF_WIDTH).max(22.0 * unit_pixels);
        let header_top: f32 = menu_bar_height;
        let body_top: f32 = header_top + header_height + strip_height;
        let body_height: f32 = (height_pixels - body_top - footer_height).max(unit_pixels);
        ScreenLayout {
            unit_pixels,
            menu_bar: rect(0.0, 0.0, width_pixels, menu_bar_height),
            header: rect(0.0, header_top, width_pixels, header_height),
            form_strip: rect(
                margin,
                header_top + header_height,
                width_pixels - 2.0 * margin,
                strip_height,
            ),
            main: rect(
                margin,
                body_top,
                width_pixels - library_width - 2.0 * margin,
                body_height,
            ),
            library: rect(
                width_pixels - library_width,
                body_top,
                library_width - margin,
                body_height,
            ),
            footer: rect(
                0.0,
                height_pixels - footer_height,
                width_pixels,
                footer_height,
            ),
        }
    }
}

/// Draws the whole screen: the song when one is open, else the empty
/// screen, then the menu bar, then any menu or dialog on top.
///
/// # Errors
///
/// When the glyph atlas lacks a music glyph the layout asks for.
pub fn draw_screen(
    content: &Content<'_>,
    shell: &Shell,
    play: &PlayView,
    atlas: &GlyphAtlas,
    width_pixels: f32,
    height_pixels: f32,
) -> Result<Frame, RenderError> {
    let layout: ScreenLayout = ScreenLayout::new(width_pixels, height_pixels);
    let mut canvas: Canvas = Canvas::new(atlas);
    match content {
        Content::Song(session) => draw_song_screen(&mut canvas, &layout, session, play)?,
        Content::Repair(repair) => screen_repair::draw(&mut canvas, &layout, repair, shell)?,
        Content::Empty => screen_shell::draw_empty(&mut canvas, &layout, shell)?,
    }
    screen_shell::draw_menu_bar(&mut canvas, &layout, shell)?;
    canvas.begin_overlay();
    let session: Option<&Session> = match content {
        Content::Song(session) => Some(session),
        Content::Repair(_) | Content::Empty => None,
    };
    screen_shell::draw_overlays(&mut canvas, &layout, shell, session)?;
    Ok(canvas.finish())
}

fn draw_song_screen(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    session: &Session,
    play: &PlayView,
) -> Result<(), RenderError> {
    let timeline: &Timeline = session.timeline();
    let place: Option<TimelinePlace> = play
        .position_ticks
        .and_then(|ticks: u64| timeline.place_at(ticks));
    draw_header(canvas, layout, session, play, place)?;
    draw_form_strip(canvas, layout, session, play)?;
    canvas.fill(layout.main, colors::panel());
    match session.zoom {
        Zoom::Form => {
            screen_form::draw(canvas, layout.main, layout.unit_pixels, session, place)?;
        }
        Zoom::Harmony => {
            screen_harmony::draw(canvas, layout.main, layout.unit_pixels, session, place)?;
        }
        Zoom::Tracks => {
            screen_tracks::draw(canvas, layout.main, layout.unit_pixels, session, place)?;
        }
        Zoom::Staff => screen_staff::draw(
            canvas,
            layout.main,
            layout.unit_pixels,
            session,
            play,
            place,
        )?,
    }
    if session.pane == Pane::Main {
        canvas.frame(layout.main, 2.0, colors::focus_edge());
    }
    if let Some(preview) = session.preview() {
        let label: String = format!("PREVIEW  {}", preview.label);
        let em: f32 = 1.3 * layout.unit_pixels;
        let width: f32 = canvas.text_width(&label, em) + 2.0 * layout.unit_pixels;
        let badge: ScreenRect = rect(
            layout.main.right_x_pixels - width - layout.unit_pixels,
            layout.main.top_y_pixels + 0.5 * layout.unit_pixels,
            width,
            2.0 * layout.unit_pixels,
        );
        canvas.fill(badge, colors::panel_raised());
        canvas.frame(badge, 2.0, colors::preview());
        canvas.text_in(&label, badge, em, colors::preview())?;
    }
    screen_library::draw(canvas, layout.library, layout.unit_pixels, session)?;
    draw_footer(canvas, layout, session)?;
    Ok(())
}

fn draw_header(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    session: &Session,
    play: &PlayView,
    place: Option<TimelinePlace>,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    canvas.fill(layout.header, colors::panel());
    let baseline: f32 = layout.header.top_y_pixels + 2.25 * unit;
    let mut x: f32 = MARGIN_UNITS * unit;
    let unsaved: &str = if session.document().has_unsaved_edits() {
        " *"
    } else {
        ""
    };
    let title: String = format!("{}{unsaved}", session.timeline().song_name);
    x += canvas.text(&title, x, baseline, 1.6 * unit, Align::Left, colors::text())? + 2.0 * unit;
    // The five modes, in the order of the song's hierarchy, each named for
    // what it is for, with its key.
    let button_em: f32 = 1.2 * unit;
    for mode in Mode::ALL {
        let label: String = format!("{} {}", mode.key_name(), mode.title());
        x += button(
            canvas,
            x,
            layout.header,
            unit,
            &label,
            button_em,
            session.mode == mode,
            HitTarget::Mode(mode),
        )?;
    }
    // The counter and the player's state, on the right.
    let right: f32 = layout.header.right_x_pixels - MARGIN_UNITS * unit;
    let counter: String = match place {
        Some(place) => counter_text(session.timeline(), place),
        None => String::new(),
    };
    let state: String = match (play.playing, play.looping) {
        (true, true) => format!("LOOPING  {counter}"),
        (true, false) => format!("PLAYING  {counter}"),
        (false, _) => String::from("Stopped"),
    };
    let state_color: [f32; 4] = if play.playing {
        colors::playhead()
    } else {
        colors::text_soft()
    };
    let status_width: f32 = canvas.text(
        &play.status,
        right,
        baseline,
        1.1 * unit,
        Align::Right,
        colors::text_faint(),
    )?;
    let state_width: f32 = canvas.text(
        &state,
        right - status_width - 2.0 * unit,
        baseline,
        1.5 * unit,
        Align::Right,
        state_color,
    )?;
    // What an edit changes now, between the modes and the state.
    let editing_left: f32 = x + 1.5 * unit;
    let editing_right: f32 = right - status_width - state_width - 4.0 * unit;
    canvas.text_fitted(
        &session.editing_text(),
        editing_left,
        baseline,
        1.2 * unit,
        editing_right - editing_left,
        colors::text(),
    )?;
    Ok(())
}

/// A header button; returns how much room it took.
#[allow(clippy::too_many_arguments)] // each is a separate, named quantity
fn button(
    canvas: &mut Canvas<'_>,
    x: f32,
    header: ScreenRect,
    unit: f32,
    label: &str,
    em: f32,
    chosen: bool,
    target: HitTarget,
) -> Result<f32, RenderError> {
    let width: f32 = canvas.text_width(label, em) + 1.6 * unit;
    let area: ScreenRect = rect(x, header.top_y_pixels + 0.6 * unit, width, 2.2 * unit);
    if chosen {
        canvas.fill(area, colors::selection_fill());
        canvas.frame(area, 1.0, colors::selection_edge());
    } else {
        canvas.frame(area, 1.0, colors::border());
    }
    let color: [f32; 4] = if chosen {
        colors::text()
    } else {
        colors::text_soft()
    };
    canvas.text_in(label, area, em, color)?;
    canvas.hit(area, target);
    Ok(width + 0.5 * unit)
}

/// `Verse  pass 2 of 2   Bar 3   Beat 2` (EFS-1.3).
#[must_use]
pub fn counter_text(timeline: &Timeline, place: TimelinePlace) -> String {
    let Some(entry) = timeline.entries.get(place.entry_index) else {
        return String::new();
    };
    let beat: u64 = entry.bars.get(place.bar_index).map_or(1, |bar| {
        // ticks into the bar ÷ ticks/beat → whole beats; beat 1 is the first.
        place.offset_ticks / bar.meter.beat_length().ticks().max(1) + 1
    });
    let section: String = if entry.count_in {
        String::from("Count-in")
    } else if entry.repeat_count > 1 {
        format!(
            "{}  {} of {}",
            entry.section_name, place.pass_number, entry.repeat_count
        )
    } else {
        entry.section_name.clone()
    };
    format!("{section}   Bar {}   Beat {beat}", place.bar_index + 1)
}

/// The entry's key as the screen shows it: `D major`, `F♯ minor`.
#[must_use]
pub fn key_label(entry: &EntryView) -> String {
    entry
        .key
        .map_or_else(|| String::from("no key"), key_label_of)
}

/// A key as the screen shows it.
#[must_use]
pub fn key_label_of(key: vocabulary::key::Key) -> String {
    format!(
        "{} {}",
        composer::chord_symbol::note_name_text(key.tonic()),
        key.mode().text()
    )
}

/// The key's colour on the key-centre lane.
#[must_use]
pub fn key_color(entry: &EntryView) -> [f32; 4] {
    entry.key.map_or_else(colors::panel_raised, |key| {
        let signature = composer::key_signature::KeySignature::of(key);
        colors::key_hue(
            signature.fifths(),
            key.mode() == vocabulary::key::Mode::Minor,
        )
    })
}

/// The tempo in beats per minute, with its decimals when it has any.
#[must_use]
pub fn tempo_label(entry: &EntryView) -> String {
    let whole: u64 = entry.milli_beats_per_minute / 1000;
    let thousandths: u64 = entry.milli_beats_per_minute % 1000;
    if thousandths == 0 {
        format!("{whole}")
    } else {
        format!(
            "{whole}.{}",
            format!("{thousandths:03}").trim_end_matches('0')
        )
    }
}

#[must_use]
pub fn meter_label(entry: &EntryView) -> String {
    entry.meter.map_or_else(String::new, |meter| {
        format!(
            "{}/{}",
            meter.pulses_per_bar(),
            meter.pulses_per_whole_note()
        )
    })
}

/// Where each entry's block lies on the form strip: as wide as the entry
/// lasts, a muted entry a narrow fixed width. Returns (left, width) per
/// entry.
fn strip_blocks(timeline: &Timeline, strip: ScreenRect, unit: f32) -> Vec<(f32, f32)> {
    let muted_width: f32 = 1.4 * unit;
    let gap: f32 = 0.25 * unit;
    let muted_count: f32 =
        count_as_f32(timeline.entries.iter().filter(|entry| entry.muted).count());
    let entry_count: f32 = count_as_f32(timeline.entries.len());
    let width_for_music: f32 = (strip.right_x_pixels - strip.left_x_pixels)
        - muted_count * muted_width
        - entry_count * gap;
    // Song lengths in ticks are far below 2^24 × 5040, well within f32's
    // reach for a proportion.
    #[allow(clippy::cast_precision_loss)]
    let total_ticks: f32 = (timeline.total_ticks.max(1)) as f32;
    let mut x: f32 = strip.left_x_pixels;
    timeline
        .entries
        .iter()
        .map(|entry: &EntryView| {
            #[allow(clippy::cast_precision_loss)]
            let width: f32 = if entry.muted {
                muted_width
            } else {
                (entry.length_ticks() as f32 / total_ticks * width_for_music).max(unit)
            };
            let block: (f32, f32) = (x, width);
            x += width + gap;
            block
        })
        .collect()
}

fn draw_form_strip(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    session: &Session,
    play: &PlayView,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    let timeline: &Timeline = session.timeline();
    let blocks_top: f32 = layout.form_strip.top_y_pixels + 0.6 * unit;
    let blocks: Vec<(f32, f32)> = strip_blocks(timeline, layout.form_strip, unit);
    for (index, (entry, (x, width))) in timeline.entries.iter().zip(&blocks).enumerate() {
        let selected: bool = index == session.selection.entry;
        draw_strip_block(canvas, entry, *x, *width, blocks_top, unit, selected)?;
        canvas.hit(
            rect(
                *x,
                blocks_top,
                *width,
                STRIP_BLOCK_UNITS * unit + 1.9 * unit,
            ),
            HitTarget::Entry(index),
        );
    }
    if let Some(position_ticks) = play.position_ticks {
        // The playhead across the strip, where the song is.
        for (entry, (x, width)) in timeline.entries.iter().zip(&blocks) {
            let end: u64 = entry.start_ticks + entry.length_ticks();
            if entry.muted || position_ticks < entry.start_ticks || position_ticks >= end {
                continue;
            }
            #[allow(clippy::cast_precision_loss)]
            let fraction: f32 =
                (position_ticks - entry.start_ticks) as f32 / entry.length_ticks().max(1) as f32;
            let playhead_x: f32 = x + fraction * width;
            canvas.line(
                (playhead_x, blocks_top - 0.3 * unit),
                (
                    playhead_x,
                    blocks_top + STRIP_BLOCK_UNITS * unit + 2.1 * unit,
                ),
                3.0,
                colors::playhead(),
            );
        }
    }
    Ok(())
}

/// Height of an entry's block on the form strip, above its key lane.
const STRIP_BLOCK_UNITS: f32 = 3.4;

/// One entry on the form strip: its block, and its key on the lane below.
fn draw_strip_block(
    canvas: &mut Canvas<'_>,
    entry: &EntryView,
    x: f32,
    width: f32,
    blocks_top: f32,
    unit: f32,
    selected: bool,
) -> Result<(), RenderError> {
    let block: ScreenRect = rect(x, blocks_top, width, STRIP_BLOCK_UNITS * unit);
    let (fill, edge): ([f32; 4], [f32; 4]) = if selected {
        (colors::selection_fill(), colors::selection_edge())
    } else {
        (colors::panel_raised(), colors::border())
    };
    canvas.fill(block, fill);
    canvas.frame(block, 1.0, edge);
    if !entry.muted && width > 3.0 * unit {
        canvas.text_fitted(
            &entry.section_name,
            x + 0.5 * unit,
            blocks_top + 1.5 * unit,
            1.1 * unit,
            width - unit,
            colors::text(),
        )?;
        let repeats: String = if entry.repeat_count > 1 {
            format!("x{}", entry.repeat_count)
        } else {
            String::new()
        };
        let detail: String = format!("{repeats}  {} bars", entry.bar_count());
        canvas.text_fitted(
            detail.trim(),
            x + 0.5 * unit,
            blocks_top + 2.9 * unit,
            0.95 * unit,
            width - unit,
            colors::text_soft(),
        )?;
    }
    let lane_top: f32 = blocks_top + STRIP_BLOCK_UNITS * unit + 0.3 * unit;
    let lane: ScreenRect = rect(x, lane_top, width, 1.6 * unit);
    canvas.fill(
        lane,
        if entry.muted {
            colors::panel()
        } else {
            key_color(entry)
        },
    );
    if !entry.muted && width > 2.5 * unit {
        canvas.text_fitted(
            &key_label(entry),
            x + 0.4 * unit,
            lane_top + 1.2 * unit,
            0.95 * unit,
            width - 0.6 * unit,
            colors::text(),
        )?;
    }
    Ok(())
}

/// How many whole things of a size fit: the floor of a non-negative
/// layout quotient, at least 1.
#[must_use]
pub fn whole_count(quotient: f32) -> usize {
    // Layout quotients are small positive numbers (rows or bars that fit
    // on a screen), so the floor fits a usize; a negative or NaN one
    // becomes 0, then 1.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count: usize = quotient.floor().max(1.0) as usize;
    count
}

/// A count of things as f32, for layout. Counts on screen are small.
#[must_use]
pub fn count_as_f32(count: usize) -> f32 {
    // Counts of entries, bars and rows are far below 2^24.
    #[allow(clippy::cast_precision_loss)]
    let value: f32 = count as f32;
    value
}

fn draw_footer(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    session: &Session,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    canvas.fill(layout.footer, colors::panel());
    let left: f32 = MARGIN_UNITS * unit;
    let width: f32 = layout.footer.right_x_pixels - 2.0 * left;
    if let Some(message) = session.message() {
        let color: [f32; 4] = match message.kind {
            MessageKind::Done => colors::done(),
            MessageKind::Problem => colors::problem(),
        };
        canvas.hit(
            rect(
                0.0,
                layout.footer.top_y_pixels,
                layout.footer.right_x_pixels,
                3.4 * unit,
            ),
            HitTarget::CopyMessage(MessageSource::Footer),
        );
        screen_shell::draw_wrapped(
            canvas,
            &message.text,
            screen_shell::WrappedText {
                left_pixels: left,
                first_baseline_pixels: layout.footer.top_y_pixels + 1.6 * unit,
                line_step_pixels: 1.5 * unit,
                em_pixels: 1.2 * unit,
                max_width_pixels: width,
                max_lines: 2,
            },
            color,
        )?;
    }
    let (first, second): (String, String) = help_lines(session);
    canvas.text_fitted(
        &first,
        left,
        layout.footer.top_y_pixels + 4.9 * unit,
        1.05 * unit,
        width,
        colors::text_soft(),
    )?;
    canvas.text_fitted(
        &second,
        left,
        layout.footer.top_y_pixels + 6.2 * unit,
        1.05 * unit,
        width,
        colors::text_faint(),
    )?;
    Ok(())
}

/// The keys that work now: for the pane and mode first, then those that
/// always work.
#[must_use]
pub fn help_lines(session: &Session) -> (String, String) {
    let specific: &str = if session.pane == Pane::Library {
        "Library:  Up/Down browse (shows a preview)   Left/Right Progressions or Rhythms   Enter apply   Esc drop   Space hear the preview   N new section from the progression   Tab back"
    } else {
        match session.mode {
            Mode::ArrangeSections => {
                "F1 Arrange Sections:  Left/Right or Up/Down entry   [ ] fewer/more repeats   M mute   Alt+Left/Right move entry   D duplicate   Delete remove   Shift+Up/Down entry key by semitone   Ctrl+Up/Down entry key by fifth   T/Shift+T entry tempo   N new section"
            }
            Mode::ComposeSections => {
                "F2 Compose Sections:  Left/Right bar   Up/Down line   Shift+Up/Down or K/Shift+K section key by semitone   Ctrl+Up/Down or F/Shift+F section key by fifth   T/Shift+T section tempo   C coupled/sampled"
            }
            Mode::EditLines => {
                "F3 Edit Lines:  Left/Right note   Shift+Left/Right bars   Up/Down line   Ctrl+- / Ctrl+= more/fewer bars   Ctrl+Up/Down scale step   Ctrl+Shift+Up/Down chord tone   Shift+Up/Down semitone   Alt+Up/Down octave   A accent   R rest/note   B bass from roots"
            }
            Mode::SongHarmony => {
                "F4 Song Harmony:  Left/Right bar   Up/Down entry   Shift+Left/Right bar range   Ctrl+Up/Down chords by scale step   Shift+Up/Down chords by semitone   Tab progressions"
            }
            Mode::TrackHarmony => {
                "F5 Track Harmony:  Left/Right bar   Shift+Left/Right bar range   Up/Down track   Ctrl+Up/Down chords by scale step   Shift+Up/Down by semitone   Alt+Up/Down by octave   Tab progressions"
            }
        }
    };
    let always: &str = "F1-F5 mode   Tab library   Space play/stop from the selected entry   Shift+Space from the start   L loop entry   Ctrl+Z/Ctrl+Y undo/redo   Ctrl+S save   Ctrl+Q quit";
    (specific.to_string(), always.to_string())
}
