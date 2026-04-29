use crate::parsers::mac::errors::*;
use regex::Regex;
use std::sync::LazyLock;

pub static IGNORE_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\s*//.*$|\s*#.*$|\s+)?$").unwrap());
pub static COMMA_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*,\s*").unwrap());

pub static DATA_SECTION_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\.data\s*").unwrap());
pub static TEXT_SECTION_R: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\.text\s*").unwrap());

pub static DATA_DEFINITION_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?<name>[\w_][\w_\d-]*):\s*(?<type>\.\w+)\s+(?<content>.*)\s*;?$").unwrap()
});
pub static TEXT_DEFINITION_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?<name>[\w_][\w_\d-]*):)?\s*(?<content>[^:/#;]+)?;?(?://.*$|#.*)?$")
        .unwrap()
});

pub fn read_ints<'a>(content: &'a str) -> Result<Vec<u16>, ParsingErrorType<'a>> {
    let mut m: Vec<u16> = Vec::new();
    let mut values = COMMA_R.split(content).peekable();
    while let Some(value) = values.next() {
        if values.peek().is_none() && value.is_empty() {
            break;
        }
        if value.starts_with("#") || value.starts_with("//") {
            break;
        }
        let pieces = value.split_whitespace();
        let mut s = String::new();
        for piece in pieces {
            if piece.starts_with("#") || piece.starts_with("//") {
                break;
            }
            s.push_str(piece);
        }
        if let Ok(n) = s.parse::<i16>() {
            m.push(n as u16);
        } else {
            return Err(ParsingErrorType::InvalidNumber(value));
        }
    }
    Ok(m)
}

pub fn read_str<'a>(content: &'a str) -> Result<Vec<u16>, ParsingErrorType<'a>> {
    let mut m = Vec::new();
    if content.len() < 2 {
        return Err(ParsingErrorType::InvalidString(content));
    }
    let mut chars = content.char_indices();
    let (_, starting_quote) = chars.next().unwrap_or((0, '"'));
    let mut len = 0;
    while let Some((byte_size, c)) = chars.next() {
        len += byte_size;
        if c == starting_quote {
            break;
        }
        if !c.is_ascii() {
            return Err(ParsingErrorType::InvalidChar(
                &content[len - byte_size..len],
            ));
        }
        if c == '\\' {
            if let Some((byte_size2, c2)) = chars.next() {
                len += byte_size2;
                if !c2.is_ascii() {
                    return Err(ParsingErrorType::InvalidChar(
                        &content[len - byte_size2..len],
                    ));
                }
                match c2 {
                    'n' => m.push('\n' as u16),
                    't' => m.push('\t' as u16),
                    'r' => m.push('\r' as u16),
                    '"' => m.push('"' as u16),
                    '\'' => m.push('\'' as u16),
                    _ => {
                        return Err(ParsingErrorType::InvalidChar(
                            &content[len - byte_size - byte_size..len],
                        ));
                    }
                }
            }
        } else {
            m.push(c as u16);
        }
    }
    Ok(m)
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_content() {
//         let mut con = Content::new("         \nAAAAA:  .data 25,25,25");
//         con.consume(&*WHITESPACE_R);
//         assert_eq!(con.text, "AAAAA:  .data 25,25,25");
//         con.consume(&*IDENTIFIER_R);
//         assert_eq!(con.text, ":  .data 25,25,25");
//     }
// }
