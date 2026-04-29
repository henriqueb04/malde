use regex::{Regex, Captures};
use std::sync::LazyLock;

pub static IDENTIFIER_R: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([\d\w_-]+)").unwrap()
});
pub static IGNORE_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:\s*//.*$|\s*#.*$|\s+)$").unwrap());
pub static COMMA_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*,\s*").unwrap());
pub static DATA_SECTION_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\.data\s*").unwrap());
pub static TEXT_SECTION_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\.text\s*").unwrap());

pub static DATA_DEFINITION_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(?<name>[\w_][\w_\d-]*):\s*(?<type>\.\w+)\s+(?<value1>-?\d+)\s*(?<valuesn>(?:,\s*-?\d+\s*)+)?\s*,?\s*;?\s*(?://.*|#.*)?$").unwrap());
pub static TEXT_DEFINITION_R: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(?:(?<name>[\w_][\w_\d-]*):)?\s*(?<content>[^:/#;]+)?;?(?://.*$|#.*)?$").unwrap());

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
