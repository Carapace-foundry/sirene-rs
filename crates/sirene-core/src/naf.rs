use core::fmt;
use core::str::FromStr;

use crate::error::CodeNafError;

/// Nomenclature d'Activités Française (NAF Rév. 2).
///
/// Encodes 4 digits followed by 1 uppercase letter (e.g. `"6201Z"` for `"62.01Z"`).
/// Fits in a compact 5-byte array (`[u8; 5]`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeNaf(pub [u8; 5]);

impl CodeNaf {
    /// Creates a `CodeNaf` from a 5-byte array (e.g. `*b"6201Z"`), verifying format.
    pub fn new(mut raw: [u8; 5]) -> Result<Self, CodeNafError> {
        if !raw[0].is_ascii_digit()
            || !raw[1].is_ascii_digit()
            || !raw[2].is_ascii_digit()
            || !raw[3].is_ascii_digit()
        {
            let s = match std::str::from_utf8(&raw) {
                Ok(v) => v.to_string(),
                Err(_) => String::new(),
            };
            return Err(CodeNafError::InvalidFormat { input: s });
        }

        raw[4] = raw[4].to_ascii_uppercase();
        if !raw[4].is_ascii_uppercase() {
            let s = match std::str::from_utf8(&raw) {
                Ok(v) => v.to_string(),
                Err(_) => String::new(),
            };
            return Err(CodeNafError::InvalidFormat { input: s });
        }

        Ok(Self(raw))
    }

    /// Creates a `CodeNaf` without verification.
    pub const fn new_unchecked(raw: [u8; 5]) -> Self {
        Self(raw)
    }

    /// Parses a NAF code from a string, handling both `"62.01Z"` and `"6201Z"`.
    pub fn parse(s: &str) -> Result<Self, CodeNafError> {
        let trimmed = s.trim();
        let mut bytes = [0u8; 5];
        let mut idx = 0;

        for &b in trimmed.as_bytes() {
            if b == b'.' || b == b' ' || b == b'-' {
                continue;
            }
            if idx < 5 {
                bytes[idx] = b;
                idx += 1;
            } else {
                return Err(CodeNafError::InvalidFormat {
                    input: s.to_string(),
                });
            }
        }

        if idx != 5 {
            return Err(CodeNafError::InvalidFormat {
                input: s.to_string(),
            });
        }

        Self::new(bytes)
    }

    /// Returns the NAF division (first 2 digits, e.g. `"62"` for IT programming/consulting).
    #[inline]
    pub fn division(&self) -> &str {
        std::str::from_utf8(&self.0[0..2]).unwrap_or_default()
    }

    /// Returns the NAF group (first 3 characters, e.g. `"620"`).
    #[inline]
    pub fn groupe(&self) -> &str {
        std::str::from_utf8(&self.0[0..3]).unwrap_or_default()
    }

    /// Returns the NAF class (first 4 digits, e.g. `"6201"`).
    #[inline]
    pub fn classe(&self) -> &str {
        std::str::from_utf8(&self.0[0..4]).unwrap_or_default()
    }

    /// Returns the NAF sub-class (all 5 characters, e.g. `"6201Z"`).
    #[inline]
    pub fn sous_classe(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }

    /// Returns a string slice of the 5-character NAF code (e.g. `"6201Z"`).
    #[inline]
    pub fn as_str(&self) -> &str {
        self.sous_classe()
    }

    /// Returns the NAF Rev. 2 section letter (A to U) corresponding to the 2-digit division.
    pub fn section(&self) -> char {
        let div = match self.division().parse::<u8>() {
            Ok(d) => d,
            Err(_) => return '?',
        };
        match div {
            1..=3 => 'A',
            5..=9 => 'B',
            10..=33 => 'C',
            35 => 'D',
            36..=39 => 'E',
            41..=43 => 'F',
            45..=47 => 'G',
            49..=53 => 'H',
            55..=56 => 'I',
            58..=63 => 'J',
            64..=66 => 'K',
            68 => 'L',
            69..=75 => 'M',
            77..=82 => 'N',
            84 => 'O',
            85 => 'P',
            86..=88 => 'Q',
            90..=93 => 'R',
            94..=96 => 'S',
            97..=98 => 'T',
            99 => 'U',
            _ => '?',
        }
    }

    /// Returns the human-readable formatted string with dot (e.g. `"62.01Z"`).
    pub fn formatted(&self) -> String {
        let mut s = String::with_capacity(6);
        let raw = self.sous_classe();
        if raw.len() == 5 && raw.is_char_boundary(2) {
            s.push_str(&raw[0..2]);
            s.push('.');
            s.push_str(&raw[2..5]);
        }
        s
    }

    /// Returns the underlying 5 bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 5] {
        &self.0
    }
}

impl fmt::Display for CodeNaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.formatted())
    }
}

impl fmt::Debug for CodeNaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeNaf(\"{}\")", self.formatted())
    }
}

impl FromStr for CodeNaf {
    type Err = CodeNafError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for CodeNaf {
    type Error = CodeNafError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CodeNaf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.formatted())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CodeNaf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
