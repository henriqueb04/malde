use std::{collections::HashMap, fs, path::PathBuf};

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
    str_values: Vec<(String, String)>,
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
        KeywordMap {
            map,
            str_values: DEFAULT_KEYWORDS
                .map(|(name, op)| (name.to_string(), op.to_string()))
                .into_iter()
                .collect(),
        }
    }
}

impl TryFrom<Vec<(String, String)>> for KeywordMap {
    type Error = (usize, KeywordMapError);
    fn try_from(value: Vec<(String, String)>) -> Result<Self, Self::Error> {
        let str_values = value.clone();
        let map: HashMap<String, (usize, usize)> = value
            .into_iter()
            .enumerate()
            .map(|(i, pair)| {
                KeywordMap::validate_pair(pair.0.trim(), pair.1.trim()).map_err(|err| (i, err))
            })
            .collect::<Result<HashMap<String, (usize, usize)>, Self::Error>>()?;
        Ok(KeywordMap { map, str_values })
    }
}

impl<'a> TryFrom<Vec<(&'a str, &'a str)>> for KeywordMap {
    type Error = (usize, KeywordMapError);
    fn try_from(value: Vec<(&'a str, &'a str)>) -> Result<Self, Self::Error> {
        let str_values = value
            .iter()
            .map(|(name, op)| (name.to_string(), op.to_string()))
            .collect();
        let map: HashMap<String, (usize, usize)> = value
            .into_iter()
            .enumerate()
            .map(|(i, pair)| {
                KeywordMap::validate_pair(pair.0.trim(), pair.1.trim()).map_err(|err| (i, err))
            })
            .collect::<Result<HashMap<String, (usize, usize)>, Self::Error>>()?;
        Ok(KeywordMap { map, str_values })
    }
}

impl KeywordMap {
    pub fn get(&self, key: &str) -> Option<&(usize, usize)> {
        self.map.get(key)
    }

    pub fn str_values(&self) -> Vec<(String, String)> {
        self.str_values.clone()
    }

    pub fn from_filename(path: PathBuf) -> Result<Self, (Option<usize>, KeywordMapError)> {
        let content = fs::read_to_string(path).map_err(|err| (None, err.into()))?;
        let mut map = HashMap::new();
        let mut str_values = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let mut contents = line.split(',');
            if let Some(name) = contents.next().map(|s| s.trim())
                && let Some(op) = contents.next().map(|s| s.trim())
            {
                let (name, pair) =
                    KeywordMap::validate_pair(name, op).map_err(|err| (Some(i), err))?;
                str_values.push((name.clone(), op.to_string()));
                map.insert(name, pair);
            } else {
                return Err((Some(i), KeywordMapError::WrongFormat));
            }
        }
        Ok(KeywordMap { map, str_values })
    }

    pub fn validate_pair(
        name: &str,
        op: &str,
    ) -> Result<(String, (usize, usize)), KeywordMapError> {
        if name.is_empty() {
            Err(KeywordMapError::EmptyName)
        } else if op.is_empty() {
            Err(KeywordMapError::EmptyOpCode)
        } else if !op.chars().all(|c| c == '0' || c == '1') {
            Err(KeywordMapError::InvalidOpCode)
        } else if op.len() > 16 {
            Err(KeywordMapError::OpCodeTooLong)
        } else {
            match u16::from_str_radix(op, 2) {
                Ok(n) => Ok((name.to_string(), (n as usize, 16 - op.len()))),
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
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Formato inválido")]
    WrongFormat,
}
