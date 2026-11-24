//! Draws one frame of the editor's screen on the CPU, in colour, without a
//! window or GPU, to check the layout of each view.
//!
//! `cargo run --release -p render --example screen_preview -- [WIDTH HEIGHT] [STEP ...]`
//!
//! Loads `songs_canonical/composer/composer_demo.song` and the two
//! libraries in `library/`, carries out the steps, and writes
//! `scratch/screen_preview.ppm` (default 2560 × 1440). Steps, in order:
//!
//! | Step | Does |
//! |---|---|
//! | `song=PATH` | first step only: opens that song instead |
//! | `mode=N` | picks mode N, as F1 to F5 do (1 Arrange Sections, 2 Compose Sections, 3 Edit Lines, 4 Song Harmony, 5 Track Harmony) |
//! | `entry=N` | selects entry N (1 the first) |
//! | `bars=N` | widens the selection to N bars |
//! | `track=N` | selects voice track N (1 the first) |
//! | `library` | switches to the library browser (shows a preview) |
//! | `down=N` | moves the library cursor (or selection) down N rows |
//! | `rhythms` | the rhythm tab |
//! | `enter` | keeps the preview |
//! | `right=N` | moves the selection right N times (the next note on the staff) |
//! | `tone=N` | moves the selected notes N chord tones (negative: down) |
//! | `convert` | changes the selected line between coupled and sampled (twice: ask, then do) |
//! | `barwidth=N` | the staff's bars N steps wider (negative: narrower, more across) |
//! | `playing=TICKS` | draws the playhead at TICKS from the song's start |

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use composer::library::{Libraries, LibraryTab};
use composer::session::{Command, Mode, Session};
use composer::song_document::SongDocument;
use render::canvas::Frame;
use render::glyph_atlas::GlyphAtlas;
use render::glyph_instance::GlyphInstance;
use render::line_instance::LineInstance;
use render::screen::{PlayView, draw_screen};

const SONG: &str = include_str!("../../../songs_canonical/composer/composer_demo.song");
const PROGRESSIONS: &str = include_str!("../../../library/Progressions.library");
const RHYTHMS: &str = include_str!("../../../library/Rhythms.library");

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn number(argument: &str, prefix: &str) -> Result<usize, String> {
    argument[prefix.len()..]
        .parse()
        .map_err(|_| format!("{argument:?}: give a whole number after {prefix}"))
}

// The steps are one list, read top to bottom.
#[allow(clippy::too_many_lines)]
fn run() -> Result<(), String> {
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    let (width_pixels, height_pixels): (u32, u32) = if arguments.len() >= 2
        && arguments[0].parse::<u32>().is_ok()
        && arguments[1].parse::<u32>().is_ok()
    {
        let width: u32 = arguments
            .remove(0)
            .parse()
            .map_err(|_| String::from("width"))?;
        let height: u32 = arguments
            .remove(0)
            .parse()
            .map_err(|_| String::from("height"))?;
        (width, height)
    } else {
        (2560, 1440)
    };
    let song_path: Option<String> = arguments
        .first()
        .and_then(|first| first.strip_prefix("song="))
        .map(str::to_string);
    let (song_name, song_text): (String, String) = match song_path {
        Some(path) => {
            let text: String =
                std::fs::read_to_string(&path).map_err(|e| format!("Reading {path}: {e}"))?;
            arguments.remove(0);
            (path, text)
        }
        None => (String::from("composer_demo.song"), SONG.to_string()),
    };
    let document: SongDocument =
        SongDocument::new(&song_name, song_text).map_err(|errors| errors.join("\n"))?;
    let libraries: Libraries = Libraries::read(
        "Progressions.library",
        PROGRESSIONS,
        "Rhythms.library",
        RHYTHMS,
    )
    .map_err(|errors| errors.join("\n"))?;
    let mut session: Session = Session::new(document, libraries);
    let mut play: PlayView = PlayView {
        playing: false,
        looping: false,
        position_ticks: None,
        status: String::from("preview: no MIDI output"),
    };
    for argument in &arguments {
        let command: Option<Command> = match argument.as_str() {
            other if other.starts_with("mode=") => {
                let key: usize = number(other, "mode=")?;
                let mode: Mode = *Mode::ALL
                    .get(key.saturating_sub(1))
                    .ok_or_else(|| format!("{other:?}: modes are 1 to 5"))?;
                Some(Command::SetMode(mode))
            }
            "library" => Some(Command::SwitchPane),
            "rhythms" => Some(Command::SetTab(LibraryTab::Rhythms)),
            "enter" => Some(Command::Commit),
            other if other.starts_with("entry=") => Some(Command::SelectEntry(
                number(other, "entry=")?.saturating_sub(1),
            )),
            other if other.starts_with("track=") => Some(Command::SelectTrack(
                number(other, "track=")?.saturating_sub(1),
            )),
            other if other.starts_with("bars=") => {
                for _ in 1..number(other, "bars=")? {
                    session.handle(Command::ExtendRight);
                }
                None
            }
            other if other.starts_with("down=") => {
                for _ in 0..number(other, "down=")? {
                    session.handle(Command::Down);
                }
                None
            }
            other if other.starts_with("right=") => {
                for _ in 0..number(other, "right=")? {
                    session.handle(Command::Right);
                }
                None
            }
            "convert" => Some(Command::ConvertLineForm),
            other if other.starts_with("barwidth=") => {
                let steps: i8 = other["barwidth=".len()..]
                    .parse()
                    .map_err(|e| format!("{other:?}: {e}"))?;
                Some(Command::StaffBarWidth(steps))
            }
            other if other.starts_with("tone=") => {
                let steps: i16 = other["tone=".len()..]
                    .parse()
                    .map_err(|e| format!("{other:?}: {e}"))?;
                Some(Command::ChordTone(steps))
            }
            other if other.starts_with("playing=") => {
                play.playing = true;
                play.position_ticks =
                    Some(u64::try_from(number(other, "playing=")?).map_err(|e| e.to_string())?);
                None
            }
            other => {
                return Err(format!(
                    "{other:?} is not a step; see the doc comment at the top of the file"
                ));
            }
        };
        if let Some(command) = command {
            session.handle(command);
        }
    }
    let atlas: GlyphAtlas = GlyphAtlas::embedded().map_err(|e| e.to_string())?;
    // Preview sizes are far below 2^24, so the conversion to f32 is exact.
    #[allow(clippy::cast_precision_loss)]
    let frame: Frame = draw_screen(
        &session,
        &play,
        &atlas,
        width_pixels as f32,
        height_pixels as f32,
    )
    .map_err(|e| e.to_string())?;
    let mut image: Image = Image::new(width_pixels, height_pixels)?;
    for line in &frame.lines {
        image.draw_line(line);
    }
    for glyph in &frame.glyphs {
        image.draw_glyph(glyph, &atlas);
    }
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scratch/screen_preview.ppm");
    image.write_ppm(&path)?;
    println!(
        "{} lines, {} glyphs, {} click regions: wrote {}",
        frame.lines.len(),
        frame.glyphs.len(),
        frame.hits.len(),
        path.display()
    );
    if let Some(message) = session.message() {
        println!("Message: {}", message.text);
    }
    Ok(())
}

/// Linear-light colour per pixel, row by row from the top, starting at the
/// theme's background.
struct Image {
    width_pixels: u32,
    height_pixels: u32,
    pixels: Vec<[f32; 3]>,
}

impl Image {
    fn new(width_pixels: u32, height_pixels: u32) -> Result<Image, String> {
        let count: usize = usize::try_from(u64::from(width_pixels) * u64::from(height_pixels))
            .map_err(|e| e.to_string())?;
        let background: [f32; 3] = [0.0091, 0.0103, 0.0130];
        Ok(Image {
            width_pixels,
            height_pixels,
            pixels: vec![background; count],
        })
    }

    fn blend(&mut self, column: i64, row: i64, color: [f32; 4], amount: f32) {
        let (Ok(column), Ok(row)): (Result<u32, _>, Result<u32, _>) =
            (u32::try_from(column), u32::try_from(row))
        else {
            return;
        };
        if column >= self.width_pixels || row >= self.height_pixels {
            return;
        }
        let Ok(index): Result<usize, _> =
            usize::try_from(u64::from(row) * u64::from(self.width_pixels) + u64::from(column))
        else {
            return;
        };
        let alpha: f32 = amount * color[3];
        let old: [f32; 3] = self.pixels[index];
        self.pixels[index] = [
            old[0] + (color[0] - old[0]) * alpha,
            old[1] + (color[1] - old[1]) * alpha,
            old[2] + (color[2] - old[2]) * alpha,
        ];
    }

    /// As the line shader: coverage by distance across the line, ends cut
    /// square.
    fn draw_line(&mut self, line: &LineInstance) {
        let half: f32 = line.thickness_pixels / 2.0;
        let (start_x, start_y): (f32, f32) = (line.start_x_pixels, line.start_y_pixels);
        let (along_x, along_y): (f32, f32) =
            (line.end_x_pixels - start_x, line.end_y_pixels - start_y);
        let length: f32 = along_x.hypot(along_y);
        if length <= 0.0 {
            return;
        }
        let (direction_x, direction_y): (f32, f32) = (along_x / length, along_y / length);
        let margin: f32 = half + 1.0;
        for row in pixel_range(
            start_y.min(line.end_y_pixels) - margin,
            start_y.max(line.end_y_pixels) + margin,
        ) {
            for column in pixel_range(
                start_x.min(line.end_x_pixels) - margin,
                start_x.max(line.end_x_pixels) + margin,
            ) {
                #[allow(clippy::cast_precision_loss)]
                let (x, y): (f32, f32) =
                    (column as f32 + 0.5 - start_x, row as f32 + 0.5 - start_y);
                let along: f32 = x * direction_x + y * direction_y;
                if !(0.0..=length).contains(&along) {
                    continue;
                }
                let across: f32 = (x * direction_y - y * direction_x).abs();
                let coverage: f32 = (half - across + 0.5).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    self.blend(column, row, line.color_linear_rgba, coverage);
                }
            }
        }
    }

    /// As the glyph shader: the field's distance, in screen pixels, gives
    /// the coverage.
    fn draw_glyph(&mut self, glyph: &GlyphInstance, atlas: &GlyphAtlas) {
        let x0: f32 = glyph.left_x_pixels;
        let y0: f32 = glyph.top_y_pixels;
        for row in pixel_range(y0, y0 + glyph.height_pixels) {
            for column in pixel_range(x0, x0 + glyph.width_pixels) {
                #[allow(clippy::cast_precision_loss)]
                let (across, down): (f32, f32) = (
                    ((column as f32 + 0.5) - x0) / glyph.width_pixels,
                    ((row as f32 + 0.5) - y0) / glyph.height_pixels,
                );
                let u: f32 = glyph.atlas_left + (glyph.atlas_right - glyph.atlas_left) * across;
                let v: f32 = glyph.atlas_top + (glyph.atlas_bottom - glyph.atlas_top) * down;
                let value: f32 = sample(atlas, u, v);
                let distance_pixels: f32 = (value - 0.5) * glyph.distance_scale_pixels;
                let coverage: f32 = (distance_pixels + 0.5).clamp(0.0, 1.0);
                self.blend(column, row, glyph.color_linear_rgba, coverage);
            }
        }
    }

    fn write_ppm(&self, path: &Path) -> Result<(), String> {
        let mut bytes: Vec<u8> =
            format!("P6\n{} {}\n255\n", self.width_pixels, self.height_pixels).into_bytes();
        for pixel in &self.pixels {
            for channel in pixel {
                // 0 ≤ sRGB × 255 ≤ 255.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                bytes.push((to_srgb(*channel).clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        std::fs::write(path, bytes).map_err(|e| format!("Writing {}: {e}", path.display()))
    }
}

fn pixel_range(from: f32, to: f32) -> std::ops::Range<i64> {
    // Window coordinates are far inside i64's range.
    #[allow(clippy::cast_possible_truncation)]
    let range: std::ops::Range<i64> = ((from - 0.5).ceil() as i64)..((to - 0.5).ceil() as i64);
    range
}

fn sample(atlas: &GlyphAtlas, u: f32, v: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let (width, height): (f32, f32) = (atlas.width_pixels as f32, atlas.height_pixels as f32);
    let x: f32 = (u * width - 0.5).clamp(0.0, width - 1.0);
    let y: f32 = (v * height - 0.5).clamp(0.0, height - 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (left, top): (usize, usize) = (x.floor() as usize, y.floor() as usize);
    let width_whole: usize = atlas.width_pixels as usize;
    let height_whole: usize = atlas.height_pixels as usize;
    let right: usize = (left + 1).min(width_whole - 1);
    let bottom: usize = (top + 1).min(height_whole - 1);
    #[allow(clippy::cast_precision_loss)]
    let (across, down): (f32, f32) = (x - left as f32, y - top as f32);
    let at = |column: usize, row: usize| -> f32 {
        f32::from(atlas.image[row * width_whole + column]) / 255.0
    };
    let upper: f32 = at(left, top) * (1.0 - across) + at(right, top) * across;
    let lower: f32 = at(left, bottom) * (1.0 - across) + at(right, bottom) * across;
    upper * (1.0 - down) + lower * down
}

fn to_srgb(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}
