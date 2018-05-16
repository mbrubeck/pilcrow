// pilcrow/src/lib.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate cocoa;
extern crate core_foundation;
extern crate core_graphics;
extern crate core_text;

#[macro_use]
extern crate objc;

pub use self::format::{FontFaceId, FontId};
use {Color, Font, Format, Image, ParagraphTrait, ParagraphCursorTrait};

use TypographicBounds;

use self::{
    core_foundation::attributedstring::{CFAttributedString, CFMutableAttributedString},
    core_foundation::base::{CFIndex, CFRange, CFType, CFTypeRef, TCFType, kCFNotFound},
    core_foundation::dictionary::{CFDictionary, CFMutableDictionary},
    core_foundation::string::{CFString, CFStringRef},
    core_foundation::stringtokenizer::{CFStringTokenizer, kCFStringTokenizerUnitWord},
    core_graphics::base::CGFloat,
    core_graphics::font::CGGlyph,
    core_graphics::geometry::{CGPoint, CGRect, CGSize, CG_ZERO_POINT},
    core_graphics::path::CGPath,
    core_text::frame::CTFrame,
    core_text::framesetter::CTFramesetter,
    core_text::line::CTLine,
    core_text::run::CTRun,
};
use euclid::{Point2D, Rect, SideOffsets2D, Size2D, Vector2D};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::cmp::{self, Ordering};
use std::ops::Range;
use std::sync::{Mutex, MutexGuard, RwLock};

mod format;
use self::format;

pub type Glyph = CGGlyph;

pub struct Paragraph {
    attributed_string: Mutex<CFMutableAttributedString>,
    style: ParagraphStyle,
}

unsafe impl Sync for Paragraph {}

impl ParagraphTrait for Paragraph {
    #[inline]
    fn new(style: ParagraphStyle) -> Paragraph {
        let attributed_string = CFAttributedString::new(CFString::from(""), CFDictionary::new());
        let mutable_attributed_string =
            CFMutableAttributedString::from_attributed_string(attributed_string);
        Paragraph {
            attributed_string: Mutex::new(mutable_attributed_string),
            style,
        }
    }

    fn from_string(string: &str, style: ParagraphStyle) -> Paragraph {
        let attributed_string = CFAttributedString::new(CFString::from(string),
                                                        CFDictionary::new());
        let mutable_attributed_string =
            CFMutableAttributedString::from_attributed_string(attributed_string);
        Paragraph {
            attributed_string: Mutex::new(mutable_attributed_string),
            style,
        }
    }

    #[inline]
    fn copy_string_in_range(&self, buffer: &mut String, range: Range<usize>) {
        buffer.extend(self.attributed_string
                          .lock()
                          .unwrap()
                          .string()
                          .to_string()
                          .chars()
                          .skip(range.start)
                          .take(range.end - range.start))
    }

    #[inline]
    fn char_len(&self) -> usize {
        self.attributed_string.lock().unwrap().string().char_len() as usize
    }

    #[inline]
    fn edit_at(&mut self, position: usize) -> ParagraphCursor {
        let attributes = self.attributed_string
                             .lock()
                             .unwrap()
                             .attributes_at(position as CFIndex)
                             .0;
        let format_stack = format::attributes_to_formatting(&attributes);
        ParagraphCursor {
            attributed_string: self.attributed_string.lock().unwrap(),
            position: position,
            buffer: CFMutableAttributedString::new(),
            format_stack: format_stack,
        }
    }

    fn word_range_at_char_index(&self, index: usize) -> Range<usize> {
        let attributed_string = self.attributed_string.lock().unwrap();
        let string = attributed_string.string();
        let range = CFRange::init(0, string.char_len());
        let tokenizer = CFStringTokenizer::new(string, range, kCFStringTokenizerUnitWord);
        tokenizer.go_to_token_at_index(index as CFIndex);
        let range = tokenizer.get_current_token_range();
        (range.location as usize)..((range.location + range.length) as usize)
    }
}

pub struct ParagraphCursor<'a> {
    attributed_string: MutexGuard<'a, CFMutableAttributedString>,
    position: usize,
    buffer: CFMutableAttributedString,
    format_stack: Vec<Format>,
}

impl<'a> ParagraphCursorTrait for ParagraphCursor<'a> {
    fn commit(self) {
        let range = CFRange::init(self.position as CFIndex, 0);
        let buffer = self.buffer.as_attributed_string();
        self.attributed_string.replace_attributed_string(range, buffer)
    }

    fn push_string(&mut self, string: &str) {
        let mut attributes = CFMutableDictionary::new();
        for format in &self.format_stack {
            format.add_to_native_attributes(&mut attributes);
        }
        let attributes = attributes.as_dictionary();
        let attributed_string = CFAttributedString::new(CFString::from(string), attributes);
        let range = CFRange::init(self.buffer.string().char_len() as CFIndex, 0);
        self.buffer.replace_attributed_string(range, attributed_string)
    }

    fn push_format(&mut self, format: Format) {
        self.format_stack.push(format)
    }

    fn pop_format(&mut self) {
        self.format_stack.pop().expect("ParagraphCursor::pop_format(): Format stack empty!");
    }

    fn format_stack(&self) -> &[Format] {
        &self.format_stack
    }
}

pub struct Framesetter {
    framesetters: Vec<Mutex<ParagraphFramesetter>>,
    document_style: DocumentStyle,
}

impl Framesetter {
    pub fn new(document: &Document) -> Framesetter {
        Framesetter {
            framesetters: document.paragraphs().par_iter().map(|paragraph| {
                let attributed_string = paragraph.attributed_string.lock().unwrap();
                let attributed_string = attributed_string.as_attributed_string();
                let framesetter = CTFramesetter::from_attributed_string(attributed_string.clone());
                Mutex::new(ParagraphFramesetter {
                    framesetter: framesetter,
                    attributed_string: attributed_string,
                    style: paragraph.style.clone(),
                })
            }).collect(),
            document_style: document.style.clone(),
        }
    }

    pub fn layout_in_rect(&self, rect: &Rect<f32>, callbacks: Option<Box<LayoutCallbacks>>)
                          -> Section {
        eprintln!("document margins: {:?}", self.document_style.margin);

        *LAYOUT_CALLBACKS.write().unwrap() = callbacks;

        let rect = rect.inner_rect(self.document_style.margin);

        let mut frames: Vec<_> = self.framesetters.par_iter().map(|paragraph_framesetter| {
            let paragraph_framesetter = paragraph_framesetter.lock().unwrap();
            let range = CFRange::init(0, paragraph_framesetter.attributed_string
                                                              .string()
                                                              .char_len());

            let mut rect = rect;
            rect.size.width -= paragraph_framesetter.style.margin.horizontal();

            let origin = CGPoint::new(rect.origin.x as CGFloat, rect.origin.y as CGFloat);
            let size = CGSize::new(rect.size.width as CGFloat, rect.size.height as CGFloat);
            let path = CGPath::from_rect(CGRect::new(&origin, &size), None);
            Frame {
                frame: paragraph_framesetter.framesetter.create_frame(range, path, None),
                style: paragraph_framesetter.style.clone(),
                virtual_size: rect.size,
                origin: Point2D::zero(),
            }
        }).collect();

        // TODO(pcwalton): Vertical writing direction.
        let mut origin = rect.origin;
        for mut frame in &mut frames {
            origin.y += frame.style.margin.top;
            frame.origin = origin + Vector2D::new(frame.style.margin.left, 0.0);
            origin.y += frame.height();
            origin.y += frame.style.margin.bottom;
        }

        Section {
            frames,
        }
    }
}

struct ParagraphFramesetter {
    framesetter: CTFramesetter,
    attributed_string: CFAttributedString,
    style: ParagraphStyle,
}

pub struct Section {
    frames: Vec<Frame>,
}

impl Section {
    #[inline]
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn frame_index_at_point(&self, point: &Point2D<f32>) -> Option<usize> {
        self.frames.binary_search_by(|frame| {
            compare_bounds_and_point_vertically(&frame.bounds(), &point)
        }).ok()
    }
}

pub struct Frame {
    frame: CTFrame,
    style: ParagraphStyle,
    virtual_size: Size2D<f32>,
    origin: Point2D<f32>,
}

impl Frame {
    pub fn char_len(&self) -> usize {
        self.frame.get_string_range().length as usize
    }

    pub fn lines(&self) -> Vec<Line> {
        let lines = self.frame.lines();
        let mut line_origins = vec![CG_ZERO_POINT; lines.len() as usize];
        self.frame.get_line_origins(0, &mut line_origins);
        let virtual_height = self.virtual_size.height;
        let frame_origin = self.origin;
        lines.into_iter().zip(line_origins.into_iter()).map(|(line, line_origin)| {
            Line {
                line: (*line).clone(),
                origin: Point2D::new(frame_origin.x + line_origin.x as f32,
                                     frame_origin.y + virtual_height - line_origin.y as f32),
            }
        }).collect()
    }

    #[inline]
    pub fn bounds(&self) -> Rect<f32> {
        Rect::new(self.origin, Size2D::new(self.virtual_size.width, self.height()))
    }

    pub fn height(&self) -> f32 {
        let lines = self.frame.lines();
        let line_count = lines.len();
        if line_count == 0 {
            return 0.0
        }

        let last_line = lines.get(line_count - 1).unwrap();
        let mut line_origins = [CG_ZERO_POINT];
        self.frame.get_line_origins(line_count - 1, &mut line_origins);
        last_line.typographic_bounds().descent as f32 - line_origins[0].y as f32 +
            self.virtual_size.height
    }

    pub fn line_index_at_point(&self, point: &Point2D<f32>) -> Option<usize> {
        self.lines().binary_search_by(|line| {
            compare_bounds_and_point_vertically(&line.typographic_bounding_rect(), &point)
        }).ok()
    }

    #[inline]
    pub fn style(&self) -> &ParagraphStyle {
        &self.style
    }
}

pub struct Line {
    line: CTLine,
    pub origin: Point2D<f32>,
}

impl Line {
    pub fn runs(&self) -> Vec<Run> {
        self.line.glyph_runs().into_iter().map(|run| {
            Run {
                run: (*run).clone(),
            }
        }).collect()
    }

    #[inline]
    pub fn char_range(&self) -> Range<usize> {
        let range = self.line.string_range();
        (range.location as usize)..((range.location + range.length) as usize)
    }

    pub fn typographic_bounding_rect(&self) -> Rect<f32> {
        let typographic_bounds = self.typographic_bounds();
        Rect::new(Point2D::new(self.origin.x, self.origin.y - typographic_bounds.ascent),
                  Size2D::new(typographic_bounds.width,
                              typographic_bounds.ascent + typographic_bounds.descent))
    }

    #[inline]
    pub fn typographic_bounds(&self) -> TypographicBounds {
        let typographic_bounds = self.line.typographic_bounds();
        TypographicBounds {
            width: typographic_bounds.width as f32,
            ascent: typographic_bounds.ascent as f32,
            descent: typographic_bounds.descent as f32,
            leading: typographic_bounds.leading as f32,
        }
    }

    #[inline]
    pub fn char_index_for_position(&self, position: &Point2D<f32>) -> Option<usize> {
        let position = CGPoint::new(position.x as CGFloat, position.y as CGFloat);
        match self.line.get_string_index_for_position(position) {
            kCFNotFound => None,
            index => Some(index as usize),
        }
    }

    #[inline]
    pub fn inline_position_for_char_index(&self, index: usize) -> f32 {
        self.line.get_offset_for_string_index(index as CFIndex).0 as f32
    }
}

pub struct Run {
    run: CTRun,
}

impl Run {
    #[inline]
    pub fn glyph_count(&self) -> usize {
        self.run.glyph_count() as usize
    }

    pub fn glyphs(&self) -> Vec<Glyph> {
        let mut glyphs = vec![0; self.glyph_count()];
        self.run.get_glyphs(0, &mut glyphs);
        glyphs
    }

    pub fn positions(&self) -> Vec<Point2D<f32>> {
        let mut positions = vec![CG_ZERO_POINT; self.glyph_count()];
        self.run.get_positions(0, &mut positions);
        positions.into_iter().map(|p| Point2D::new(p.x as f32, p.y as f32)).collect()
    }

    #[inline]
    pub fn char_range(&self) -> Range<usize> {
        let range = self.run.get_string_range();
        (range.start as usize)..(range.end as usize)
    }

    pub fn formatting(&self) -> Vec<Format> {
        format::attributes_to_formatting(&self.run.attributes())
    }

    #[inline]
    pub fn typographic_bounds(&self) -> TypographicBounds {
        let typographic_bounds = self.run.typographic_bounds(0..(self.glyph_count() as CFIndex));
        TypographicBounds {
            width: typographic_bounds.width as f32,
            ascent: typographic_bounds.ascent as f32,
            descent: typographic_bounds.descent as f32,
            leading: typographic_bounds.leading as f32,
        }
    }
}

fn compare_bounds_and_point_vertically(bounds: &Rect<f32>, point: &Point2D<f32>) -> Ordering {
    match (bounds.origin.y <= point.y, point.y < bounds.max_y()) {
        (true, true) => Ordering::Equal,
        (false, _) => Ordering::Greater,
        (_, false) => Ordering::Less,
    }
}
