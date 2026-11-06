use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use crate::error::SirenError;
use crate::luhn::{is_la_poste_siren_bytes, validate_siren_bytes};

/// A strictly validated 9-digit French SIREN business identifier.
///
/// Guaranteed to:
/// - Be exactly 9 ASCII decimal digits (`'0'..'9'`).
/// - Pass the official INSEE Luhn modulo 10 validation.
/// - Fit in a fixed-size `[u8; 9]` (9 bytes, zero heap allocation).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Siren(pub [u8; 9]);

impl Siren {
    /// Creates a `Siren` from a 9-byte array, checking digit validity and Luhn checksum.
    pub fn new(digits: [u8; 9]) -> Result<Self, SirenError> {
        for (i, &b) in digits.iter().enumerate() {
            if !b.is_ascii_digit() {
                return Err(SirenError::NonDigitCharacter {
                    position: i,
                    ch: b as char,
                });
            }
        }

        if !validate_siren_bytes(&digits) {
            let str_val = match std::str::from_utf8(&digits) {
                Ok(s) => s.to_string(),
                Err(_) => String::new(),
            };
            return Err(SirenError::InvalidLuhnChecksum { siren: str_val });
        }

        Ok(Self(digits))
    }

    /// Creates a `Siren` without verifying digits or Luhn checksum.
    pub const fn new_unchecked(digits: [u8; 9]) -> Self {
        Self(digits)
    }

    /// Parses a raw string containing exactly 9 ASCII digits.
    pub fn parse_exact(s: &str) -> Result<Self, SirenError> {
        let bytes = s.as_bytes();
        if bytes.len() != 9 {
            return Err(SirenError::InvalidLength {
                actual: bytes.len(),
            });
        }

        let mut arr = [0u8; 9];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }

    /// Parses a SIREN from a user-supplied string, ignoring ASCII and Unicode whitespace, dots, and hyphens.
    pub fn parse(s: &str) -> Result<Self, SirenError> {
        let mut arr = [0u8; 9];
        let mut count = 0;

        for (pos, ch) in s.char_indices() {
            if ch.is_whitespace() || ch == '.' || ch == '-' || ch == '_' {
                continue;
            }
            if ch.is_ascii_digit() {
                if count < 9 {
                    arr[count] = ch as u8;
                    count += 1;
                } else {
                    // More than 9 digits encountered
                    count += 1;
                }
            } else {
                return Err(SirenError::NonDigitCharacter { position: pos, ch });
            }
        }

        if count != 9 {
            return Err(SirenError::InvalidLength { actual: count });
        }

        Self::new(arr)
    }

    /// Returns the underlying 9-byte array.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 9] {
        &self.0
    }

    /// Returns a string slice representation of the 9-digit SIREN.
    #[inline]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }

    /// Formats the SIREN in standard 3-3-3 spaced format (e.g. `"732 829 320"`).
    pub fn formatted(&self) -> String {
        let s = self.as_str();
        let mut out = String::with_capacity(11);
        if s.len() == 9 && s.is_char_boundary(3) && s.is_char_boundary(6) {
            out.push_str(&s[0..3]);
            out.push(' ');
            out.push_str(&s[3..6]);
            out.push(' ');
            out.push_str(&s[6..9]);
        }
        out
    }

    /// Returns `true` if this SIREN belongs to La Poste (`356 000 000`).
    #[inline]
    pub fn is_la_poste(&self) -> bool {
        is_la_poste_siren_bytes(&self.0)
    }
}

impl Deref for Siren {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Siren {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for Siren {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Siren {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Siren {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Siren(\"{}\")", self.as_str())
    }
}

impl FromStr for Siren {
    type Err = SirenError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Siren {
    type Error = SirenError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&[u8]> for Siren {
    type Error = SirenError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 9 {
            return Err(SirenError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 9];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }
}

impl TryFrom<[u8; 9]> for Siren {
    type Error = SirenError;

    #[inline]
    fn try_from(digits: [u8; 9]) -> Result<Self, Self::Error> {
        Self::new(digits)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Siren {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Siren {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
