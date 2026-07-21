use std::{fmt::Display, fs, io, iter::Peekable, str::Chars};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    pub filepath: String,
    pub content: String,
    line_indices: Vec<usize>,
}

impl SourceMap {
    pub fn from_filepath(filepath: &str) -> Result<Self, io::Error> {
        let content = fs::read_to_string(filepath)?;
        Ok(SourceMap {
            filepath: filepath.to_string(),
            line_indices: get_line_indices(&content),
            content: content,
        })
    }
    pub fn from_content(content: &str) -> Self {
        SourceMap {
            filepath: String::new(),
            content: content.to_string(),
            line_indices: get_line_indices(&content),
        }
    }
    pub fn get_span(&self, span: &Span) -> &str {
        self.content
            .get(span.start..(usize::min(self.content.len(), span.end)))
            .unwrap_or("")
    }
    pub fn get<T: HasSpan>(&self, token: &T) -> &str {
        self.get_span(token.span())
    }
    pub fn get_line(&self, lineno: usize) -> &str {
        if let Some((start, end)) = self.get_line_bounds(lineno)
            && let Some(line) = self.content.get(start..end)
        {
            line
        } else {
            ""
        }
    }
    pub fn end(&self) -> Span {
        let lines = self.content.lines();
        let (count, last) = lines.fold((0, ""), |(c, _), x| (c + 1, x));
        Span {
            start: self.content.len(),
            end: self.content.len(),
            line: count,
            col: last.len(),
        }
    }
    pub fn highlight_in_line(&self, span: &Span) -> String {
        let Some((line_start, line_end)) = self.get_line_bounds(span.line) else {
            return String::new();
        };
        if span.start < line_start || span.end > line_end + 1 {
            return String::new();
        }
        let mut line = self.content[line_start..line_end].to_string();
        line.push('\n');
        line.push_str(
            " ".repeat((&self.content[line_start..span.start]).len())
                .as_str(),
        );
        line.push_str(
            "~".repeat((&self.content[span.start..span.end]).len())
                .as_str(),
        );
        line
    }
    pub fn reader(&self) -> SourceReader<'_> {
        SourceReader::new(&self.content)
    }
    fn get_line_bounds(&self, lineno: usize) -> Option<(usize, usize)> {
        if lineno < 1 || lineno > self.line_indices.len() {
            return None;
        }
        let start = self.line_indices.get(lineno - 1).cloned()?;
        let end = self
            .line_indices
            .get(lineno)
            .cloned()
            .map(|v| v - 1)
            .unwrap_or(self.content.len());
        Some((start, end))
    }
}

fn get_line_indices(content: &str) -> Vec<usize> {
    let mut lines = vec![0];
    lines.extend(
        content
            .char_indices()
            .filter(|v| v.1 == '\n')
            .map(|v| v.0 + 1)
            .collect::<Vec<_>>(),
    );
    lines
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "linha: {}, coluna: {}", self.line, self.col)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_lines() {
        let source_map = SourceMap::from_content("teste1\n2\n333 ttt");
        assert_eq!(source_map.get_line(1), "teste1");
        assert_eq!(source_map.get_line(2), "2");
        assert_eq!(source_map.get_line(3), "333 ttt");

        assert_eq!(
            source_map.highlight_in_line(&Span {
                start: 7,
                end: 8,
                line: 2,
                col: 1
            }),
            "2\n~"
        );
        assert_eq!(
            source_map.highlight_in_line(&Span {
                start: 8,
                end: 9,
                line: 2,
                col: 1
            }),
            "2\n ~"
        );
        assert_eq!(
            source_map.highlight_in_line(&Span {
                start: 13,
                end: 16,
                line: 3,
                col: 5
            }),
            "333 ttt\n    ~~~"
        );
    }
}
