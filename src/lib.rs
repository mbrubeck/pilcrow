extern crate euclid;
extern crate indexmap;
extern crate libc;
extern crate pulldown_cmark;
extern crate rayon;

#[macro_use]
extern crate lazy_static;

// XXX pub mod ffi;
pub mod markdown;

#[cfg(any(target_os = "macos", target_os="ios"))]
mod apple;
#[cfg(any(target_os = "macos", target_os="ios"))]
pub use apple::*;

#[cfg(not(any(target_os = "macos", target_os="ios")))]
mod stub;
#[cfg(not(any(target_os = "macos", target_os="ios")))]
pub use stub::*;

use euclid::{SideOffsets2D, Size2D};
use std::cmp;
use std::ops::Range;
use std::sync::RwLock;

pub trait LayoutCallbacks: Send + Sync {
    fn get_image_size(&self, image_id: u32) -> Option<Size2D<u32>>;
}

lazy_static! {
    static ref LAYOUT_CALLBACKS: RwLock<Option<Box<LayoutCallbacks>>> = {
        RwLock::new(None)
    };
}

pub struct Document {
    paragraphs: Vec<Paragraph>,
    style: DocumentStyle,
}

#[derive(Clone, Copy, Debug)]
pub struct TypographicBounds {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
}

pub enum Format {
    Font(Font),
    Color(Color),
    Link(u32, String),
    Image(u32),
}

impl Format {
    #[inline]
    pub fn from_font(font: Font) -> Format {
        Format::Font(font)
    }

    #[inline]
    pub fn from_color(color: Color) -> Format {
        Format::Color(color)
    }

    #[inline]
    pub fn from_link(link_id: u32, url: String) -> Format {
        Format::Link(link_id, url)
    }

    #[inline]
    pub fn from_image(image_id: u32) -> Format {
        Format::Image(image_id)
    }

    pub fn font(&self) -> Option<Font> {
        if let Format::Font(ref font) = *self {
            Some((*font).clone())
        } else {
            None
        }
    }

    pub fn color(&self) -> Option<Color> {
        if let Format::Color(ref color) = *self {
            Some((*color).clone())
        } else {
            None
        }
    }

    #[inline]
    pub fn link(&self) -> Option<(u32, &str)> {
        if let Format::Link(link_id, ref url) = *self {
            Some((link_id, &**url))
        } else {
            None
        }
    }

    #[inline]
    pub fn image(&self) -> Option<u32> {
        if let Format::Image(image_id) = *self {
            Some(image_id)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r,
            g,
            b,
            a,
        }
    }

    #[inline]
    pub fn r_f32(&self) -> f32 {
        (self.r as f32) / 255.0
    }

    #[inline]
    pub fn g_f32(&self) -> f32 {
        (self.g as f32) / 255.0
    }

    #[inline]
    pub fn b_f32(&self) -> f32 {
        (self.b as f32) / 255.0
    }

    #[inline]
    pub fn a_f32(&self) -> f32 {
        (self.a as f32) / 255.0
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Image {
    pub id: u32,
    pub alt_text: String,
}

#[derive(Clone, PartialEq)]
pub struct ParagraphStyle {
    pub content: ParagraphContent,
    pub margin: SideOffsets2D<f32>,
}

impl ParagraphStyle {
    #[inline]
    pub fn new(content: ParagraphContent) -> ParagraphStyle {
        ParagraphStyle {
            content,
            margin: SideOffsets2D::zero(),
        }
    }
}

impl Default for ParagraphStyle {
    #[inline]
    fn default() -> ParagraphStyle {
        ParagraphStyle {
            content: ParagraphContent::Text,
            margin: SideOffsets2D::zero(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ParagraphContent {
    Text,
    Rule,
}

impl Document {
    #[inline]
    pub fn new() -> Document {
        Document {
            paragraphs: vec![],
            style: DocumentStyle::default(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.paragraphs.clear()
    }

    #[inline]
    pub fn append_paragraph(&mut self, paragraph: Paragraph) {
        self.paragraphs.push(paragraph)
    }

    #[inline]
    pub fn append_document(&mut self, other_document: Document) {
        self.paragraphs.extend(other_document.paragraphs.into_iter())
    }

    #[inline]
    pub fn paragraphs(&self) -> &[Paragraph] {
        &self.paragraphs
    }

    #[inline]
    pub fn paragraphs_mut(&mut self) -> &mut [Paragraph] {
        &mut self.paragraphs
    }

    #[inline]
    pub fn style_mut(&mut self) -> &mut DocumentStyle {
        &mut self.style
    }

    #[inline]
    pub fn entire_range(&self) -> Range<TextLocation> {
        let start = TextLocation::new(0, 0);
        let end = match self.paragraphs.last() {
            None => start,
            Some(last_paragraph) => {
                TextLocation::new(self.paragraphs.len() - 1, last_paragraph.char_len())
            }
        };
        start..end
    }

    pub fn copy_string_in_range(&self, range: Range<TextLocation>) -> String {
        let mut buffer = String::new();
        let first_paragraph_index = range.start.paragraph_index;
        let last_paragraph_index = cmp::min(range.end.paragraph_index + 1, self.paragraphs.len());
        let paragraph_count = last_paragraph_index - first_paragraph_index;
        let paragraph_range = first_paragraph_index..last_paragraph_index;
        for (paragraph_index, paragraph) in self.paragraphs[paragraph_range].iter().enumerate() {
            let char_start = if paragraph_index == 0 {
                range.start.character_index
            } else {
                0
            };
            let char_end = if paragraph_index == paragraph_count - 1 {
                range.end.character_index
            } else {
                paragraph.char_len()
            };
            if paragraph_index != 0 {
                buffer.push('\n')
            }
            paragraph.copy_string_in_range(&mut buffer, char_start..char_end)
        }
        buffer
    }

    #[inline]
    pub fn copy_string(&self) -> String {
        self.copy_string_in_range(self.entire_range())
    }
}

#[derive(Clone, PartialEq)]
pub struct DocumentStyle {
    pub margin: SideOffsets2D<f32>,
}

impl Default for DocumentStyle {
    #[inline]
    fn default() -> DocumentStyle {
        DocumentStyle {
            margin: SideOffsets2D::zero(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
#[repr(C)]
pub struct TextLocation {
    pub paragraph_index: usize,
    pub character_index: usize,
}

impl TextLocation {
    #[inline]
    pub fn new(paragraph_index: usize, character_index: usize) -> TextLocation {
        TextLocation {
            paragraph_index,
            character_index,
        }
    }

    #[inline]
    pub fn beginning() -> TextLocation {
        TextLocation::new(0, 0)
    }
}
