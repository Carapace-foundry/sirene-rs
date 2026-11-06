use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use crate::error::{CodeCommuneError, CodePostalError};

/// A French INSEE official geographical municipality code (Code Officiel Géographique - COG).
///
/// Distinct from postal routing codes:
/// - Mainland: 5 digits (e.g. `"75101"` for Paris 1er).
/// - Corsica: `"2A004"` (Corse-du-Sud) or `"2B033"` (Haute-Corse).
/// - Overseas (DOM/TOM): 5 digits starting with 97 or 98 (e.g. `"97105"` for Basse-Terre).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeCommune(pub [u8; 5]);

impl CodeCommune {
    /// Creates a `CodeCommune` after checking COG syntax rules.
    pub fn new(raw: [u8; 5]) -> Result<Self, CodeCommuneError> {
        let is_corse = (raw[0] == b'2' && (raw[1] == b'A' || raw[1] == b'B'))
            || (raw[0] == b'2' && (raw[1] == b'a' || raw[1] == b'b'));

        let mut normalized = raw;
        if is_corse {
            normalized[1] = normalized[1].to_ascii_uppercase();
        } else if !raw[0].is_ascii_digit() || !raw[1].is_ascii_digit() {
            let s = match std::str::from_utf8(&raw) {
                Ok(v) => v.to_string(),
                Err(_) => String::new(),
            };
            return Err(CodeCommuneError::InvalidFormat { input: s });
        }

        // Remaining 3 chars must always be ASCII digits
        if !normalized[2].is_ascii_digit()
            || !normalized[3].is_ascii_digit()
            || !normalized[4].is_ascii_digit()
        {
            let s = match std::str::from_utf8(&raw) {
                Ok(v) => v.to_string(),
                Err(_) => String::new(),
            };
            return Err(CodeCommuneError::InvalidFormat { input: s });
        }

        Ok(Self(normalized))
    }

    /// Creates a `CodeCommune` without validation.
    pub const fn new_unchecked(raw: [u8; 5]) -> Self {
        Self(raw)
    }

    /// Parses a 5-character string into a `CodeCommune`.
    pub fn parse(s: &str) -> Result<Self, CodeCommuneError> {
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() != 5 {
            return Err(CodeCommuneError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 5];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }

    /// Returns the department prefix (2 or 3 characters):
    /// - `"2A"` or `"2B"` for Corsica.
    /// - 3 digits (e.g. `"971"`, `"974"`) for DOM/TOM.
    /// - 2 digits (e.g. `"75"`, `"13"`, `"69"`) for mainland departments.
    #[inline]
    pub fn departement(&self) -> &str {
        let s = self.as_str();
        if self.is_outre_mer() {
            if s.len() >= 3 && s.is_char_boundary(3) {
                &s[0..3]
            } else {
                s
            }
        } else if s.len() >= 2 && s.is_char_boundary(2) {
            &s[0..2]
        } else {
            s
        }
    }

    /// Returns `true` if the municipality is located in Corsica (2A or 2B).
    #[inline]
    pub fn is_corse(&self) -> bool {
        self.0[0] == b'2' && (self.0[1] == b'A' || self.0[1] == b'B')
    }

    /// Returns `true` if the municipality is located in an overseas department or territory (97 or 98).
    #[inline]
    pub fn is_outre_mer(&self) -> bool {
        self.0[0] == b'9' && (self.0[1] == b'7' || self.0[1] == b'8')
    }

    /// Returns the underlying 5 bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 5] {
        &self.0
    }

    /// Returns a string slice representation of the 5-character code.
    #[inline]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl Deref for CodeCommune {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for CodeCommune {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CodeCommune {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for CodeCommune {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for CodeCommune {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeCommune(\"{}\")", self.as_str())
    }
}

impl FromStr for CodeCommune {
    type Err = CodeCommuneError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for CodeCommune {
    type Error = CodeCommuneError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// A 5-digit French postal routing code (Code Postal).
///
/// Distinct from the official territorial INSEE municipality code (Code Commune / COG).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodePostal(pub [u8; 5]);

impl CodePostal {
    /// Creates a `CodePostal` verifying that all 5 characters are ASCII digits.
    pub fn new(digits: [u8; 5]) -> Result<Self, CodePostalError> {
        for (i, &b) in digits.iter().enumerate() {
            if !b.is_ascii_digit() {
                return Err(CodePostalError::NonDigitCharacter {
                    position: i,
                    ch: b as char,
                });
            }
        }
        Ok(Self(digits))
    }

    /// Creates a `CodePostal` without verifying digits.
    pub const fn new_unchecked(digits: [u8; 5]) -> Self {
        Self(digits)
    }

    /// Parses a 5-digit postal code.
    pub fn parse(s: &str) -> Result<Self, CodePostalError> {
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() != 5 {
            return Err(CodePostalError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 5];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }

    /// Returns the department prefix (2 or 3 characters):
    /// - `"20"` for Corsica postal routing.
    /// - 3 digits (e.g. `"971"`) for DOM/TOM postal routing.
    /// - 2 digits (e.g. `"75"`, `"33"`) for mainland.
    #[inline]
    pub fn departement(&self) -> &str {
        let s = self.as_str();
        if (self.0[0] == b'9') && (self.0[1] == b'7' || self.0[1] == b'8') {
            if s.len() >= 3 && s.is_char_boundary(3) {
                &s[0..3]
            } else {
                s
            }
        } else if s.len() >= 2 && s.is_char_boundary(2) {
            &s[0..2]
        } else {
            s
        }
    }

    /// Returns the underlying 5 bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 5] {
        &self.0
    }

    /// Returns a string slice representation of the 5-digit postal code.
    #[inline]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl Deref for CodePostal {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for CodePostal {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CodePostal {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for CodePostal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for CodePostal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodePostal(\"{}\")", self.as_str())
    }
}

impl FromStr for CodePostal {
    type Err = CodePostalError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for CodePostal {
    type Error = CodePostalError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CodeCommune {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CodeCommune {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CodePostal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CodePostal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
