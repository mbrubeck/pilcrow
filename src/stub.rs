#![allow(unused)]

use {FontTrait, ParagraphTrait, ParagraphCursorTrait};
use std::ops::Range;

pub struct Paragraph;

use {Format, ParagraphStyle};

impl ParagraphTrait for Paragraph {
    #[inline]
    fn new(style: ParagraphStyle) -> Paragraph {
        unimplemented!()
    }

    fn from_string(string: &str, style: ParagraphStyle) -> Paragraph {
        unimplemented!()
    }

    #[inline]
    fn copy_string_in_range(&self, buffer: &mut String, range: Range<usize>) {
        unimplemented!()
    }

    #[inline]
    fn char_len(&self) -> usize {
        unimplemented!()
    }

    #[inline]
    fn edit_at(&mut self, position: usize) -> ParagraphCursor {
        unimplemented!()
    }

    fn word_range_at_char_index(&self, index: usize) -> Range<usize> {
        unimplemented!()
    }
}

pub struct ParagraphCursor<'a>(&'a ());

impl<'a> ParagraphCursorTrait for ParagraphCursor<'a> {
    fn commit(self) {
        unimplemented!()
    }

    fn push_string(&mut self, string: &str) {
        unimplemented!()
    }

    fn push_format(&mut self, format: Format) {
        unimplemented!()
    }

    fn pop_format(&mut self) {
        unimplemented!()
    }

    fn format_stack(&self) -> &[Format] {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct Font;

impl FontTrait for Font {
    type FontId = FontId;
    type FontFaceId = FontFaceId;

    #[inline]
    fn from_native_font(native_font: NativeFont) -> Font {
        unimplemented!()
    }

    fn default_serif() -> Font {
        unimplemented!()
    }

    fn default_monospace() -> Font {
        unimplemented!()
    }

    #[inline]
    fn id(&self) -> FontId {
        unimplemented!()
    }

    #[inline]
    fn face_id(&self) -> FontFaceId {
        unimplemented!()
    }

    #[inline]
    fn size(&self) -> f32 {
        unimplemented!()
    }

    #[inline]
    fn native_font(&self) -> NativeFont {
        unimplemented!()
    }

    fn to_size(&self, new_size: f32) -> Font {
        unimplemented!()
    }

    fn to_bold(&self) -> Option<Font> {
        unimplemented!()
    }

    fn to_italic(&self) -> Option<Font> {
        unimplemented!()
    }
}

pub type NativeFont = ();

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FontId(usize);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FontFaceId(usize);
