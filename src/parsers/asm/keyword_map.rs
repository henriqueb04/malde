use std::collections::HashMap;

use thiserror::Error;

pub const DEFAULT_KEYWORDS: [(&str, &str); 30] = [
    ("LODD", "0000"),
    ("STOD", "0001"),
    ("ADDD", "0010"),
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
    ("SWAPA", "1111111100000000"),
    ("SWAPB", "1111111100100000"),
    ("SWAPC", "1111111101000000"),
    ("SWAPD", "1111111101100000"),
    ("SWAPE", "1111111110000000"),
    ("ECALL", "1111111111000000"),
    ("HALT", "0000000000000000"),
    ("INSP", "11111100"),
    ("DESP", "11111110"),
];

#[derive(Debug, Clone)]
pub struct KeywordMap {
    map: HashMap<String, (usize, usize)>,
}

impl Default for KeywordMap {
    fn default() -> Self {
        let map = DEFAULT_KEYWORDS
            .map(|(name, bin)| {
                (
                    name.to_string(),
                    (
                        usize::from_str_radix(bin, 2).expect("Keywords padrão mal formatadas"),
                        16 - bin.len(),
                    ),
                )
            })
            .into_iter()
            .collect::<HashMap<String, (usize, usize)>>();
        KeywordMap { map }
    }
}

impl TryFrom<Vec<(String, String)>> for KeywordMap {
    type Error = (usize, KeywordMapError);
    fn try_from(value: Vec<(String, String)>) -> Result<Self, Self::Error> {
        let map: HashMap<String, (usize, usize)> = value
            .into_iter()
            .enumerate()
            .map(|(i, pair)| {
                KeywordMap::validate_pair(&pair)
                    .map(|spec| (pair.0, spec))
                    .map_err(|err| (i, err))
            })
            .collect::<Result<HashMap<String, (usize, usize)>, Self::Error>>()?;
        Ok(KeywordMap { map })
    }
}

impl<'a> TryFrom<Vec<(&'a str, &'a str)>> for KeywordMap {
    type Error = (usize, KeywordMapError);
    fn try_from(value: Vec<(&'a str, &'a str)>) -> Result<Self, Self::Error> {
        let v = value
            .into_iter()
            .map(|(name, op)| (name.to_string(), op.to_string()))
            .collect::<Vec<(String, String)>>();
        KeywordMap::try_from(v)
    }
}

impl KeywordMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, key: &str) -> Option<&(usize, usize)> {
        self.map.get(key)
    }
    pub fn insert(&mut self, key: String, value: (usize, usize)) {
        self.map.insert(key, value);
    }

    pub fn validate_pair((name, op): &(String, String)) -> Result<(usize, usize), KeywordMapError> {
        if name.is_empty() {
            Err(KeywordMapError::EmptyName)
        } else if op.is_empty() {
            Err(KeywordMapError::EmptyOpCode)
        } else if !op.chars().all(|c| c == '0' || c == '1') {
            Err(KeywordMapError::InvalidOpCode)
        } else if op.trim().len() > 16 {
            Err(KeywordMapError::OpCodeTooLong)
        } else {
            match u16::from_str_radix(op.as_str(), 2) {
                Ok(n) => Ok((n as usize, 16 - op.trim().len())),
                Err(..) => Err(KeywordMapError::InvalidOpCode),
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum KeywordMapError {
    #[error("Nome vazio")]
    EmptyName,
    #[error("Código vazio")]
    EmptyOpCode,
    #[error("Código muito longo")]
    OpCodeTooLong,
    #[error("Código inválido")]
    InvalidOpCode,
}
