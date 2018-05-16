#![allow(unused)]

use std::ops::Range;

pub struct Paragraph;

use {Format, ParagraphStyle};

impl Paragraph {
    #[inline]
    pub fn new(style: ParagraphStyle) -> Paragraph {
        unimplemented!()
    }

    pub fn from_string(string: &str, style: ParagraphStyle) -> Paragraph {
        unimplemented!()
    }

    #[inline]
    pub fn copy_string_in_range(&self, buffer: &mut String, range: Range<usize>) {
        unimplemented!()
    }

    #[inline]
    pub fn char_len(&self) -> usize {
        unimplemented!()
    }

    #[inline]
    pub fn edit_at(&mut self, position: usize) -> ParagraphCursor {
        unimplemented!()
    }

    pub fn word_range_at_char_index(&self, index: usize) -> Range<usize> {
        unimplemented!()
    }
}

pub struct ParagraphCursor<'a>(&'a ());

impl<'a> ParagraphCursor<'a> {
    pub fn commit(self) {
        unimplemented!()
    }

    pub fn push_string(&mut self, string: &str) {
        unimplemented!()
    }

    pub fn push_format(&mut self, format: Format) {
        unimplemented!()
    }

    pub fn pop_format(&mut self) {
        unimplemented!()
    }

    pub fn format_stack(&self) -> &[Format] {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct Font;

impl Font {
    #[inline]
    pub fn from_native_font(native_font: NativeFont) -> Font {
        unimplemented!()
    }

    pub fn default_serif() -> Font {
        unimplemented!()
    }

    pub fn default_monospace() -> Font {
        unimplemented!()
    }

    #[inline]
    pub fn id(&self) -> FontId {
        unimplemented!()
    }

    #[inline]
    pub fn face_id(&self) -> FontFaceId {
        unimplemented!()
    }

    #[inline]
    pub fn size(&self) -> f32 {
        unimplemented!()
    }

    #[inline]
    pub fn native_font(&self) -> NativeFont {
        unimplemented!()
    }

    pub fn to_size(&self, new_size: f32) -> Font {
        unimplemented!()
    }

    pub fn to_bold(&self) -> Option<Font> {
        unimplemented!()
    }

    pub fn to_italic(&self) -> Option<Font> {
        unimplemented!()
    }
}

pub type NativeFont = ();

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FontId(usize);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FontFaceId(usize);
