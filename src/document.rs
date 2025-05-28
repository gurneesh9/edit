// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Abstractions over reading/writing arbitrary text containers.

use std::ffi::OsString;
use std::mem;
use std::ops::Range;
use std::path::PathBuf;

use crate::arena::{ArenaString, scratch_arena};
use crate::helpers::ReplaceRange as _;
use crate::syntax::{SyntaxHighlighter, FileType};

/// A document with syntax highlighting capabilities
pub struct Document {
    content: String,
    file_type: FileType,
    syntax_highlighter: Option<SyntaxHighlighter>,
}

impl Document {
    pub fn from_string(content: String, filename: &str) -> Self {
        Self {
            content,
            file_type: SyntaxHighlighter::detect_file_type(filename),
            syntax_highlighter: Some(SyntaxHighlighter::new()),
        }
    }

    pub fn highlight_line<'a>(&'a mut self, line: &'a str, line_number: usize) -> Vec<(syntect::highlighting::Style, &'a str)> {
        if let Some(highlighter) = &mut self.syntax_highlighter {
            highlighter.highlight_line(line, self.file_type, line_number)
        } else {
            vec![(syntect::highlighting::Style::default(), line)]
        }
    }

    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        if let Some(highlighter) = &mut self.syntax_highlighter {
            highlighter.set_theme(theme_name)
        } else {
            false
        }
    }

    pub fn available_themes(&self) -> Vec<String> {
        if let Some(highlighter) = &self.syntax_highlighter {
            highlighter.available_themes()
        } else {
            vec![]
        }
    }
}

/// An abstraction over reading from text containers.
pub trait ReadableDocument {
    /// Read some bytes starting at (including) the given absolute offset.
    ///
    /// # Warning
    ///
    /// * Be lenient on inputs:
    ///   * The given offset may be out of bounds and you MUST clamp it.
    ///   * You should not assume that offsets are at grapheme cluster boundaries.
    /// * Be strict on outputs:
    ///   * You MUST NOT break grapheme clusters across chunks.
    ///   * You MUST NOT return an empty slice unless the offset is at or beyond the end.
    fn read_forward(&self, off: usize) -> &[u8];

    /// Read some bytes before (but not including) the given absolute offset.
    ///
    /// # Warning
    ///
    /// * Be lenient on inputs:
    ///   * The given offset may be out of bounds and you MUST clamp it.
    ///   * You should not assume that offsets are at grapheme cluster boundaries.
    /// * Be strict on outputs:
    ///   * You MUST NOT break grapheme clusters across chunks.
    ///   * You MUST NOT return an empty slice unless the offset is zero.
    fn read_backward(&self, off: usize) -> &[u8];
}

/// An abstraction over writing to text containers.
pub trait WriteableDocument: ReadableDocument {
    /// Replace the given range with the given bytes.
    ///
    /// # Warning
    ///
    /// * The given range may be out of bounds and you MUST clamp it.
    /// * The replacement may not be valid UTF8.
    fn replace(&mut self, range: Range<usize>, replacement: &[u8]);
}

impl ReadableDocument for Document {
    fn read_forward(&self, off: usize) -> &[u8] {
        let s = self.content.as_bytes();
        &s[off.min(s.len())..]
    }

    fn read_backward(&self, off: usize) -> &[u8] {
        let s = self.content.as_bytes();
        &s[..off.min(s.len())]
    }
}

impl WriteableDocument for Document {
    fn replace(&mut self, range: Range<usize>, replacement: &[u8]) {
        // `replacement` is not guaranteed to be valid UTF-8, so we need to sanitize it.
        let scratch = scratch_arena(None);
        let utf8 = ArenaString::from_utf8_lossy(&scratch, replacement);
        let src = match &utf8 {
            Ok(s) => s,
            Err(s) => s.as_str(),
        };

        // SAFETY: `range` is guaranteed to be on codepoint boundaries.
        unsafe { self.content.as_mut_vec() }.replace_range(range, src.as_bytes());
    }
}

impl ReadableDocument for &[u8] {
    fn read_forward(&self, off: usize) -> &[u8] {
        let s = *self;
        &s[off.min(s.len())..]
    }

    fn read_backward(&self, off: usize) -> &[u8] {
        let s = *self;
        &s[..off.min(s.len())]
    }
}

impl ReadableDocument for String {
    fn read_forward(&self, off: usize) -> &[u8] {
        let s = self.as_bytes();
        &s[off.min(s.len())..]
    }

    fn read_backward(&self, off: usize) -> &[u8] {
        let s = self.as_bytes();
        &s[..off.min(s.len())]
    }
}

impl WriteableDocument for String {
    fn replace(&mut self, range: Range<usize>, replacement: &[u8]) {
        // `replacement` is not guaranteed to be valid UTF-8, so we need to sanitize it.
        let scratch = scratch_arena(None);
        let utf8 = ArenaString::from_utf8_lossy(&scratch, replacement);
        let src = match &utf8 {
            Ok(s) => s,
            Err(s) => s.as_str(),
        };

        // SAFETY: `range` is guaranteed to be on codepoint boundaries.
        unsafe { self.as_mut_vec() }.replace_range(range, src.as_bytes());
    }
}

impl ReadableDocument for PathBuf {
    fn read_forward(&self, off: usize) -> &[u8] {
        let s = self.as_os_str().as_encoded_bytes();
        &s[off.min(s.len())..]
    }

    fn read_backward(&self, off: usize) -> &[u8] {
        let s = self.as_os_str().as_encoded_bytes();
        &s[..off.min(s.len())]
    }
}

impl WriteableDocument for PathBuf {
    fn replace(&mut self, range: Range<usize>, replacement: &[u8]) {
        let mut vec = mem::take(self).into_os_string().into_encoded_bytes();
        vec.replace_range(range, replacement);
        *self = unsafe { Self::from(OsString::from_encoded_bytes_unchecked(vec)) };
    }
}
