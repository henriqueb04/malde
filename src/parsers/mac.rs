use std::collections::HashMap;

pub struct ASMParser<'a> {
    keyword_table: HashMap<&'a str, &'a str>,
    symbol_table: HashMap<&'a str, &'a str>,
    instructions: Vec<u32>,
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
            ]),
            symbol_table: HashMap::new(),
            instructions: Vec::new(),
        }
    }

    pub fn set_keywords(&mut self, keywords: &[(&'a str, &'a str)]) {
        self.keyword_table.clear();
        for (k, v) in keywords {
            self.keyword_table.insert(k, v);
        }
    }
}
