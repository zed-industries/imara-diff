use std::fmt::{Display, Write};
use std::ops::Range;

use crate::intern::{InternedInput, Interner, Token};
use crate::Sink;

/// A [`Sink`] that creates a textual diff
/// in the format typically output by git or gnu-diff if the `-u` option is used
pub struct UnifiedDiffBuilder<'a, W, T>
where
    W: Write,
    T: Display,
{
    before: &'a [Token],
    after: &'a [Token],
    interner: &'a Interner<T>,

    pos: Option<u32>,
    before_hunk_start: u32,
    after_hunk_start: u32,
    before_hunk_len: u32,
    after_hunk_len: u32,

    buffer: String,
    dst: W,
}

impl<'a, T> UnifiedDiffBuilder<'a, String, T>
where
    T: Display,
{
    /// Create a new `UnifiedDiffBuilder` for the given `input`,
    /// that will return a [`String`].
    pub fn new(input: &'a InternedInput<T>) -> Self {
        Self {
            before_hunk_start: 0,
            after_hunk_start: 0,
            before_hunk_len: 0,
            after_hunk_len: 0,
            buffer: String::with_capacity(8),
            dst: String::new(),
            interner: &input.interner,
            before: &input.before,
            after: &input.after,
            pos: None,
        }
    }
}

impl<'a, W, T> UnifiedDiffBuilder<'a, W, T>
where
    W: Write,
    T: Display,
{
    /// Create a new `UnifiedDiffBuilder` for the given `input`,
    /// that will writes it output to the provided implementation of [`Write`].
    pub fn with_writer(input: &'a InternedInput<T>, writer: W) -> Self {
        Self {
            before_hunk_start: 0,
            after_hunk_start: 0,
            before_hunk_len: 0,
            after_hunk_len: 0,
            buffer: String::with_capacity(8),
            dst: writer,
            interner: &input.interner,
            before: &input.before,
            after: &input.after,
            pos: None,
        }
    }

    fn print_tokens(&mut self, tokens: &[Token], prefix: char) {
        for &token in tokens {
            writeln!(&mut self.buffer, "{prefix}{}", self.interner[token]).unwrap();
        }
    }

    fn flush(&mut self) {
        if self.before_hunk_len == 0 && self.after_hunk_len == 0 {
            return;
        }
        let Some(pos) = self.pos.clone() else {
            return;
        };

        let end = (pos + 3).min(self.before.len() as u32);
        self.update_pos(pos, end, end);

        writeln!(
            &mut self.dst,
            "@@ -{},{} +{},{} @@",
            self.before_hunk_start + 1,
            self.before_hunk_len,
            self.after_hunk_start + 1,
            self.after_hunk_len,
        )
        .unwrap();
        write!(&mut self.dst, "{}", &self.buffer).unwrap();
        self.buffer.clear();
        self.before_hunk_len = 0;
        self.after_hunk_len = 0
    }

    fn update_pos(&mut self, pos: u32, print_to: u32, move_to: u32) {
        self.print_tokens(&self.before[pos as usize..print_to as usize], ' ');
        let len = print_to - pos;
        self.pos = Some(move_to);
        self.before_hunk_len += len;
        self.after_hunk_len += len;
    }
}

impl<W, T> Sink for UnifiedDiffBuilder<'_, W, T>
where
    W: Write,
    T: Display,
{
    type Out = W;

    fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
        if self.pos.is_none() || before.start.saturating_sub(self.pos.unwrap()) > 6 {
            self.flush();
            self.before_hunk_start = before.start.saturating_sub(3);
            self.after_hunk_start = after.start.saturating_sub(3);
            self.pos = Some(self.before_hunk_start);
        }
        self.update_pos(self.pos.unwrap(), before.start, before.end);
        self.before_hunk_len += before.end - before.start;
        self.after_hunk_len += after.end - after.start;
        self.print_tokens(
            &self.before[before.start as usize..before.end as usize],
            '-',
        );
        self.print_tokens(&self.after[after.start as usize..after.end as usize], '+');
    }

    fn finish(mut self) -> Self::Out {
        self.flush();
        self.dst
    }
}
