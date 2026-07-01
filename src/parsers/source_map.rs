use std::{
    iter::Peekable,
    str::{CharIndices, Chars},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap<'a> {
    pub filename: &'a str,
    pub content: &'a str,
}

impl<'a> SourceMap<'a> {
    pub fn get_span(&self, span: &Span) -> &'a str {
        &self.content[span.start..span.end]
    }
    pub fn get<T: HasSpan>(&self, token: &T) -> &'a str {
        self.get_span(token.span())
    }
    pub fn end(&self) -> Span {
        let lines = self.content.lines();
        let (count, last) = lines.fold((0, ""), |(c, _), x| (c+1, x));
        Span {
            start: self.content.len(),
            end: self.content.len(),
            line: count,
            col: last.len(),
        }
    }
    pub fn reader(&self) -> SourceReader<'a> {
        SourceReader::new(self.content)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

pub struct SourceReader<'a> {
    chars: Peekable<Chars<'a>>,
    offset: usize,
    line: usize,
    col: usize,
}

impl<'a> SourceReader<'a> {
    pub fn new(content: &'a str) -> Self {
        SourceReader {
            chars: content.chars().peekable(),
            offset: 0,
            line: 1,
            col: 1,
        }
    }
    pub fn next(&mut self) -> Option<(usize, char)> {
        let c = self.chars.next()?;
        let len = c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += len;
        }
        self.offset += len;
        Some((len, c))
    }
    pub fn peek(&mut self) -> Option<(usize, &char)> {
        let c = self.chars.peek()?;
        Some((c.len_utf8(), c))
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
    pub fn line(&self) -> usize {
        self.line
    }
    pub fn col(&self) -> usize {
        self.col
    }
}

pub trait HasSpan {
    fn span(&self) -> &Span;
}
