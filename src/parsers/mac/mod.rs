mod regex;
mod errors;

use crate::parsers::mac::regex::*;
use crate::parsers::mac::errors::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ASMParser<'a> {
    keyword_table: HashMap<&'a str, &'a str>,
    data_symbols: HashMap<&'a str, usize>,
    instructions_symbols: HashMap<&'a str, usize>,
    pre_instructions: Vec<(&'a str, usize)>,
}

impl<'a> ASMParser<'a> {
    pub fn new() -> Self {
        ASMParser {
            keyword_table: HashMap::from([
                ("LODD", "0000"),
                ("STOD", "0001"),
                ("ADD", "0010"),
                ("SUBD", "0011"),
                ("JPOS", "0100"),
                ("JZER", "0101"),
                ("JUMP", "0110"),
                ("LOCO", "0111"),
                ("LODL", "1000"),
                ("STOL", "1001"),
                ("ADDL", "1010"),
                ("SUBL", "1011"),
                ("JNEG", "1100"),
                ("JNZE", "1101"),
                ("CALL", "1110"),
                ("PSHI", "1111000000000000"),
                ("POPI", "1111001000000000"),
                ("PUSH", "1111010000000000"),
                ("POP", "1111011000000000"),
                ("RETN", "1111100000000000"),
                ("SWAP", "1111101000000000"),
                ("INSP", "11111100"),
                ("DESP", "11111110"),
                ("HALT", "0000000000000000"),
            ]),
            data_symbols: HashMap::new(),
            instructions_symbols: HashMap::new(),
            pre_instructions: Vec::new(),
        }
    }

    pub fn set_keywords(&mut self, keywords: &[(&'a str, &'a str)]) {
        self.keyword_table.clear();
        for (k, v) in keywords {
            self.keyword_table.insert(k, v);
        }
    }

    pub fn parse_text(&mut self, text: &'a str) -> Result<Vec<u16>, (usize, ParsingErrorType<'a>)> {
        let mut is_data = false;
        let mut is_text = false;
        let mut mem: Vec<u16> = Vec::new();
        let mut data: Vec<u16> = Vec::new();
        // Primeira passagem
        let mut symbs_waiting: Vec<&str> = Vec::new();
        for (lineno, contents) in text.split('\n').enumerate() {
            let lineno = lineno + 1;
            if IGNORE_R.is_match(contents) {
                continue;
            }
            if DATA_SECTION_R.is_match(contents) {
                is_data = true;
                is_text = false;
                continue;
            }
            if TEXT_SECTION_R.is_match(contents) {
                is_data = false;
                is_text = true;
                continue;
            }
            if is_data {
                if let Some(cap) = DATA_DEFINITION_R.captures(contents) {
                    let Some(name) = cap.name("name").map(|v| v.as_str()) else {
                        return Err((lineno, ParsingErrorType::InvalidLine));
                    };
                    let Some(def_type) = cap.name("type").map(|v| v.as_str()) else {
                        return Err((lineno, ParsingErrorType::InvalidLine));
                    };
                    let Some(content) = cap.name("content").map(|v| v.as_str()) else {
                        return Err((lineno, ParsingErrorType::InvalidLine));
                    };
                    let initial_len = data.len();
                    match def_type {
                        ".word" | ".byte" => {
                            let m: Vec<u16> = match read_ints(content) {
                                Ok(m) => m,
                                Err(err) => return Err((lineno, err)),
                            };
                            if def_type == ".byte" {
                                for &n in m.iter() {
                                    if n > u8::MAX as u16 {
                                        return Err((lineno, ParsingErrorType::NumberTooBig(n as isize, 8)));
                                    }
                                }
                            }
                            data.extend(m);
                        },
                        ".ascii" | ".asciz" | ".asciiz" => {
                            let mut m: Vec<u16> = match read_str(content) {
                                Ok(m) => m,
                                Err(err) => return Err((lineno, err)),
                            };
                            if def_type != ".ascii" {
                                m.push(0);
                            }
                            data.extend(m);
                        },
                        _ => return Err((lineno, ParsingErrorType::UnsupportedDirective(def_type))),
                    }
                    if self.data_symbols.contains_key(name)
                        || self.instructions_symbols.contains_key(name)
                    {
                        return Err((lineno, ParsingErrorType::DuplicatedIdentifier(name)));
                    }
                    self.data_symbols.insert(name, initial_len);
                } else {
                    return Err((lineno, ParsingErrorType::InvalidLine));
                }
            } else if is_text && let Some(m) = TEXT_DEFINITION_R.captures(contents) {
                if let Some(name) = m.name("name").map(|v| v.as_str()) {
                    symbs_waiting.push(name);
                }
                if let Some(content) = m.name("content").map(|v| v.as_str()) {
                    self.pre_instructions.push((content, lineno));
                    for symb in &symbs_waiting {
                        if self.data_symbols.contains_key(symb)
                            || self.instructions_symbols.contains_key(symb)
                        {
                            return Err((lineno, ParsingErrorType::DuplicatedIdentifier(symb)));
                        }
                        self.instructions_symbols
                            .insert(symb, self.pre_instructions.len() - 1);
                    }
                    symbs_waiting.clear();
                }
            }
        }
        let instruction_padding = self.pre_instructions.len() + 1;
        for (_, v) in self.data_symbols.iter_mut() {
            *v += instruction_padding;
        }
        // Segunda passagem
        for (content, lineno) in &self.pre_instructions {
            let mut s = String::with_capacity(16);
            let symbs = content.split_whitespace();
            let symbs_count = symbs.clone().count();
            for (i, symb) in symbs.enumerate() {
                if let Ok(n) = symb.parse::<isize>() {
                    let bin = format!("{:b}", n as i16);
                    let cur_len = s.len();
                    if cur_len >= 16 {
                        let len = bin.len();
                        return Err((
                            *lineno,
                            ParsingErrorType::InstructionTooBig(s, bin, cur_len + len),
                        ));
                    }
                    let num_len = 16 - cur_len;
                    if n >= 0 {
                        if n > ((1 << (num_len - 1)) - 1) {
                            return Err((*lineno, ParsingErrorType::NumberTooBig(n, num_len)));
                        }
                        let zeros = "0".repeat(num_len - bin.len());
                        s.push_str(zeros.as_str());
                        s.push_str(bin.as_str());
                    } else {
                        if num_len < 2 || n < ((1 << (num_len - 2)) ^ -1) {
                            return Err((*lineno, ParsingErrorType::NumberTooSmall(n, num_len)));
                        }
                        let bin_len = bin.len();
                        s.push_str(&bin[bin_len - num_len..]);
                    }
                } else if let Some(bin) = self
                    .keyword_table
                    .get(symb)
                    .map(|v| String::from(*v))
                    .or(self.data_symbols.get(symb).map(|v| format!("{:b}", v)))
                    .or(self
                        .instructions_symbols
                        .get(symb)
                        .map(|v| format!("{:b}", v)))
                {
                    let new_len = s.len() + bin.len();
                    if new_len > 16 {
                        return Err((
                            *lineno,
                            ParsingErrorType::InstructionTooBig(s, bin, new_len),
                        ));
                    }
                    if i == symbs_count - 1 && symbs_count > 1 {
                        let zeros = "0".repeat(16 - s.len() - bin.len());
                        s.push_str(zeros.as_str());
                    }
                    s.push_str(bin.as_str());
                    if symbs_count == 1 {
                        let zeros = "0".repeat(16 - s.len());
                        s.push_str(zeros.as_str());
                    }
                } else {
                    return Err((*lineno, ParsingErrorType::UndefinedIdentifier(symb)));
                }
            }
            if let Ok(n) = u16::from_str_radix(s.as_str(), 2) {
                mem.push(n);
            } else {
                return Err((*lineno, ParsingErrorType::InvalidInstruction(s)));
            }
        }
        mem.push(0);
        mem.extend(data);
        Ok(mem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mem() {
        let mut parser = ASMParser::new();
        let mem = parser.parse_text(
            "aflkjaflkdsjf

.data
    TESTE1: .word 5
    TESTE2: .word 1, 2, 3, 4
    TESTE3: .word 1, 2, 3, 4,
    TESTE4: .word 5,

.text
MAIN:
    LODD TESTE1;
    SUBD TESTE2;

PRINT: LOCO TESTE4
    LOCO 1
    LOCO -1",
        ).unwrap();
        let expected = [
            0b0000000000000110,
            0b0011000000000111,
            0b0111000000001111,
            0b0111000000000001,
            0b0111111111111111,
            0b0000000000000000,
            0b0000000000000101,
            0b0000000000000001,
            0b0000000000000010,
            0b0000000000000011,
            0b0000000000000100,
            0b0000000000000001,
            0b0000000000000010,
            0b0000000000000011,
            0b0000000000000100,
            0b0000000000000101,
        ];
        for (i, s) in mem.iter().enumerate() {
            println!("Got:      {:016b}", s);
            println!("Expected: {:016b}", expected[i]);
        }
        assert_eq!(mem, expected);

        let mut parser = ASMParser::new();
        let mem = parser.parse_text(".data
TESTE1: .word 1 // Comentário
TESTE2: .word 2, # Comentário
TESTE3: .word 3, 4
TESTE4: .word 5, 6,
TESTE5: .word 7, 8, // Comentário
TESTE6: .byte 9
TESTE7: .ascii \"abc\"
TESTE8: .asciz \"abc\" // Comentário
").unwrap();
        let expected = [
            0b0000000000000000,
            0b0000000000000001,
            0b0000000000000010,
            0b0000000000000011,
            0b0000000000000100,
            0b0000000000000101,
            0b0000000000000110,
            0b0000000000000111,
            0b0000000000001000,
            0b0000000000001001,
            97,
            98,
            99,
            97,
            98,
            99,
            0,
        ];
        for (i, &s) in mem.iter().enumerate() {
            if s != expected[i] { println!("---"); }
            println!("Got:      {:016b}", s);
            println!("Expected: {:016b}", expected[i]);
            if s != expected[i] { println!("---"); }
        }
        assert_eq!(mem, expected);
    }
}
