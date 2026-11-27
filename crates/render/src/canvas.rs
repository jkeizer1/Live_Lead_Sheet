//! What one frame of the editor's screen collects: line records, glyph
//! records, and the regions a click can land on. The screen modules draw
//! filled boxes, frames, text and music glyphs through it, so each only
//! says where things go.

use crate::error::RenderError;
use crate::glyph_atlas::{GlyphAtlas, GlyphFont};
use crate::glyph_instance::GlyphInstance;
use crate::line_instance::LineInstance;
use crate::strip_layout::{ScreenRect, TEXT_CAP_HEIGHT_EMS};

use composer::arrange_editor::ArrangeTarget;
use composer::library::LibraryTab;
use composer::repair::RepairTarget;
use composer::session::Mode;
use composer::shell::ShellTarget;

/// What a click on a region asks for. The app turns it into a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    Mode(Mode),
    LibraryTab(LibraryTab),
    Entry(usize),
    Bar {
        entry: usize,
        bar: usize,
    },
    Track(usize),
    Element {
        entry: usize,
        bar: usize,
        track: usize,
        element: usize,
        /// Which time through a repeated entry, 1 the first.
        pass_number: u32,
    },
    LibraryRow(usize),
    /// A track's form control: coupled or sampled.
    LineForm(usize),
    /// The menu bar, a menu's items, or a dialog.
    Shell(ShellTarget),
    /// The repair screen's list and buttons.
    Repair(RepairTarget),
    /// A message on the screen: a click copies all of it to the clipboard.
    CopyMessage(MessageSource),
    /// A cell or a dialog's control in F1 (Arrange Song Sections).
    Arrange(ArrangeTarget),
}

/// Which message a click on a message copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSource {
    /// The message in the footer.
    Footer,
    /// The message in the open dialog.
    Dialog,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub rect: ScreenRect,
    pub target: HitTarget,
}

/// A finished frame.
pub struct Frame {
    pub lines: Vec<LineInstance>,
    pub glyphs: Vec<GlyphInstance>,
    /// Drawn after everything above, so a menu or dialog covers the text
    /// under it (the GPU draws all lines, then all glyphs, of a layer).
    pub overlay_lines: Vec<LineInstance>,
    pub overlay_glyphs: Vec<GlyphInstance>,
    /// Later regions lie on top of earlier ones.
    pub hits: Vec<Hit>,
}

impl Frame {
    /// The region on top at a point.
    #[must_use]
    pub fn hit_at(&self, x_pixels: f32, y_pixels: f32) -> Option<HitTarget> {
        self.hits
            .iter()
            .rev()
            .find(|hit: &&Hit| hit.rect.contains(x_pixels, y_pixels))
            .map(|hit: &Hit| hit.target)
    }
}

/// The frame being drawn.
pub struct Canvas<'a> {
    pub atlas: &'a GlyphAtlas,
    pub lines: Vec<LineInstance>,
    pub glyphs: Vec<GlyphInstance>,
    overlay_lines: Vec<LineInstance>,
    overlay_glyphs: Vec<GlyphInstance>,
    /// Whether drawing now goes to the overlay layer.
    in_overlay: bool,
    pub hits: Vec<Hit>,
}

#[must_use]
pub fn rect(
    left_x_pixels: f32,
    top_y_pixels: f32,
    width_pixels: f32,
    height_pixels: f32,
) -> ScreenRect {
    ScreenRect {
        left_x_pixels,
        top_y_pixels,
        right_x_pixels: left_x_pixels + width_pixels,
        bottom_y_pixels: top_y_pixels + height_pixels,
    }
}

/// Where text goes along a line: from a left edge, or centred on a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl<'a> Canvas<'a> {
    #[must_use]
    pub fn new(atlas: &'a GlyphAtlas) -> Canvas<'a> {
        Canvas {
            atlas,
            lines: Vec::new(),
            glyphs: Vec::new(),
            overlay_lines: Vec::new(),
            overlay_glyphs: Vec::new(),
            in_overlay: false,
            hits: Vec::new(),
        }
    }

    /// From here on, everything drawn lands on top of what was drawn so far,
    /// text included.
    pub fn begin_overlay(&mut self) {
        self.in_overlay = true;
    }

    fn line_layer(&mut self) -> &mut Vec<LineInstance> {
        if self.in_overlay {
            &mut self.overlay_lines
        } else {
            &mut self.lines
        }
    }

    fn glyph_layer(&mut self) -> &mut Vec<GlyphInstance> {
        if self.in_overlay {
            &mut self.overlay_glyphs
        } else {
            &mut self.glyphs
        }
    }

    #[must_use]
    pub fn finish(self) -> Frame {
        Frame {
            lines: self.lines,
            glyphs: self.glyphs,
            overlay_lines: self.overlay_lines,
            overlay_glyphs: self.overlay_glyphs,
            hits: self.hits,
        }
    }

    pub fn hit(&mut self, rect: ScreenRect, target: HitTarget) {
        self.hits.push(Hit { rect, target });
    }

    /// A filled box: a horizontal line as thick as the box is tall.
    pub fn fill(&mut self, area: ScreenRect, color_linear_rgba: [f32; 4]) {
        let height_pixels: f32 = area.bottom_y_pixels - area.top_y_pixels;
        if height_pixels <= 0.0 || area.right_x_pixels <= area.left_x_pixels {
            return;
        }
        self.line_layer().push(LineInstance {
            start_x_pixels: area.left_x_pixels,
            start_y_pixels: area.top_y_pixels + height_pixels / 2.0,
            end_x_pixels: area.right_x_pixels,
            end_y_pixels: area.top_y_pixels + height_pixels / 2.0,
            thickness_pixels: height_pixels,
            color_linear_rgba,
        });
    }

    /// A box's outline, `thickness_pixels` wide, drawn inside its edges.
    pub fn frame(&mut self, area: ScreenRect, thickness_pixels: f32, color_linear_rgba: [f32; 4]) {
        let ScreenRect {
            left_x_pixels: left,
            top_y_pixels: top,
            right_x_pixels: right,
            bottom_y_pixels: bottom,
        } = area;
        let t: f32 = thickness_pixels;
        self.fill(rect(left, top, right - left, t), color_linear_rgba);
        self.fill(rect(left, bottom - t, right - left, t), color_linear_rgba);
        self.fill(
            rect(left, top + t, t, bottom - top - 2.0 * t),
            color_linear_rgba,
        );
        self.fill(
            rect(right - t, top + t, t, bottom - top - 2.0 * t),
            color_linear_rgba,
        );
    }

    pub fn line(
        &mut self,
        start: (f32, f32),
        end: (f32, f32),
        thickness_pixels: f32,
        color_linear_rgba: [f32; 4],
    ) {
        self.line_layer().push(LineInstance {
            start_x_pixels: start.0,
            start_y_pixels: start.1,
            end_x_pixels: end.0,
            end_y_pixels: end.1,
            thickness_pixels,
            color_linear_rgba,
        });
    }

    /// The text with any character the atlas lacks shown as one it has:
    /// a dash for dashes and dots, a question mark otherwise. A message
    /// must never stop a frame from being drawn.
    #[must_use]
    pub fn drawable(&self, text: &str) -> String {
        text.chars()
            .map(|character: char| {
                if self.atlas.glyph(GlyphFont::DejaVuSans, character).is_ok() {
                    character
                } else {
                    match character {
                        '–' | '—' | '·' | '•' | '−' => '-',
                        '×' => 'x',
                        '…' => '.',
                        '‘' | '’' => '\'',
                        '“' | '”' => '"',
                        '\n' | '\t' => ' ',
                        _ => '?',
                    }
                }
            })
            .collect()
    }

    /// How wide text is at `em_pixels`.
    #[must_use]
    pub fn text_width(&self, text: &str, em_pixels: f32) -> f32 {
        let drawable: String = self.drawable(text);
        self.atlas
            .text_width_pixels(GlyphFont::DejaVuSans, &drawable, em_pixels)
            .unwrap_or(0.0)
    }

    /// Text on a baseline, aligned at `x_pixels`. Returns its width.
    ///
    /// # Errors
    ///
    /// Never for text from `drawable`; kept so an atlas fault is reported.
    pub fn text(
        &mut self,
        text: &str,
        x_pixels: f32,
        baseline_y_pixels: f32,
        em_pixels: f32,
        align: Align,
        color_linear_rgba: [f32; 4],
    ) -> Result<f32, RenderError> {
        let drawable: String = self.drawable(text);
        let width_pixels: f32 =
            self.atlas
                .text_width_pixels(GlyphFont::DejaVuSans, &drawable, em_pixels)?;
        let left_x_pixels: f32 = match align {
            Align::Left => x_pixels,
            Align::Center => x_pixels - width_pixels / 2.0,
            Align::Right => x_pixels - width_pixels,
        };
        let atlas: &GlyphAtlas = self.atlas;
        atlas.text(
            GlyphFont::DejaVuSans,
            &drawable,
            left_x_pixels,
            baseline_y_pixels,
            em_pixels,
            color_linear_rgba,
            self.glyph_layer(),
        )?;
        Ok(width_pixels)
    }

    /// Text cut with "..." to fit `max_width_pixels`.
    ///
    /// # Errors
    ///
    /// As `text`.
    pub fn text_fitted(
        &mut self,
        text: &str,
        x_pixels: f32,
        baseline_y_pixels: f32,
        em_pixels: f32,
        max_width_pixels: f32,
        color_linear_rgba: [f32; 4],
    ) -> Result<(), RenderError> {
        let full: String = self.drawable(text);
        let mut shown: String = full.clone();
        let mut kept: Vec<char> = full.chars().collect();
        while self.text_width(&shown, em_pixels) > max_width_pixels {
            if kept.pop().is_none() {
                return Ok(());
            }
            shown = format!("{}..", kept.iter().collect::<String>().trim_end());
        }
        self.text(
            &shown,
            x_pixels,
            baseline_y_pixels,
            em_pixels,
            Align::Left,
            color_linear_rgba,
        )?;
        Ok(())
    }

    /// Text centred in a box, both ways.
    ///
    /// # Errors
    ///
    /// As `text`.
    pub fn text_in(
        &mut self,
        text: &str,
        area: ScreenRect,
        em_pixels: f32,
        color_linear_rgba: [f32; 4],
    ) -> Result<(), RenderError> {
        let center_x: f32 = f32::midpoint(area.left_x_pixels, area.right_x_pixels);
        let center_y: f32 = f32::midpoint(area.top_y_pixels, area.bottom_y_pixels);
        self.text(
            text,
            center_x,
            center_y + TEXT_CAP_HEIGHT_EMS * em_pixels / 2.0,
            em_pixels,
            Align::Center,
            color_linear_rgba,
        )?;
        Ok(())
    }

    /// One music glyph (SMuFL) with its origin at the point.
    ///
    /// # Errors
    ///
    /// When the atlas lacks the glyph.
    pub fn music(
        &mut self,
        codepoint: char,
        x_pixels: f32,
        y_pixels: f32,
        em_pixels: f32,
        color_linear_rgba: [f32; 4],
    ) -> Result<(), RenderError> {
        if let Some(glyph) = self.atlas.place(
            GlyphFont::Bravura,
            codepoint,
            x_pixels,
            y_pixels,
            em_pixels,
            color_linear_rgba,
        )? {
            self.glyph_layer().push(glyph);
        }
        Ok(())
    }

    /// How far a music glyph advances.
    #[must_use]
    pub fn music_advance(&self, codepoint: char, em_pixels: f32) -> f32 {
        self.atlas
            .glyph(GlyphFont::Bravura, codepoint)
            .map_or(0.0, |glyph| glyph.advance_ems * em_pixels)
    }
}
