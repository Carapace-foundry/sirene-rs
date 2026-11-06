use core::fmt;
use core::str::FromStr;

use crate::error::EtatAdministratifError;

/// Administrative status of an establishment or legal unit in SIRENE V3.
///
/// - `Actif` (`'A'`): The enterprise or establishment is currently in activity.
/// - `Cesse` (`'C'`): The enterprise or establishment has ceased business operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EtatAdministratif {
    #[default]
    Actif,
    Cesse,
}

impl EtatAdministratif {
    /// Returns `true` if the status is active (`'A'`).
    #[inline]
    pub fn is_actif(self) -> bool {
        matches!(self, Self::Actif)
    }

    /// Returns `true` if the status is ceased/closed (`'C'`).
    #[inline]
    pub fn is_cesse(self) -> bool {
        matches!(self, Self::Cesse)
    }

    /// Returns the single ASCII character code (`'A'` or `'C'`).
    #[inline]
    pub fn as_char(self) -> char {
        match self {
            Self::Actif => 'A',
            Self::Cesse => 'C',
        }
    }

    /// Returns the single-byte ASCII slice (`b"A"` or `b"C"`).
    #[inline]
    pub fn as_bytes(self) -> &'static [u8; 1] {
        match self {
            Self::Actif => b"A",
            Self::Cesse => b"C",
        }
    }

    /// Parses an administrative status code from a string or byte.
    pub fn parse(s: &str) -> Result<Self, EtatAdministratifError> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("a") || trimmed.eq_ignore_ascii_case("actif") {
            Ok(Self::Actif)
        } else if trimmed.eq_ignore_ascii_case("c")
            || trimmed.eq_ignore_ascii_case("cesse")
            || trimmed.eq_ignore_ascii_case("cessée")
            || trimmed.eq_ignore_ascii_case("radiee")
            || trimmed.eq_ignore_ascii_case("radiée")
        {
            Ok(Self::Cesse)
        } else {
            Err(EtatAdministratifError::InvalidCode {
                code: s.to_string(),
            })
        }
    }
}

impl fmt::Display for EtatAdministratif {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Actif => "A",
            Self::Cesse => "C",
        })
    }
}

impl FromStr for EtatAdministratif {
    type Err = EtatAdministratifError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<char> for EtatAdministratif {
    type Error = EtatAdministratifError;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            'A' | 'a' => Ok(Self::Actif),
            'C' | 'c' => Ok(Self::Cesse),
            _ => Err(EtatAdministratifError::InvalidCode {
                code: c.to_string(),
            }),
        }
    }
}

impl TryFrom<u8> for EtatAdministratif {
    type Error = EtatAdministratifError;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'A' | b'a' => Ok(Self::Actif),
            b'C' | b'c' => Ok(Self::Cesse),
            _ => Err(EtatAdministratifError::InvalidCode {
                code: (b as char).to_string(),
            }),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EtatAdministratif {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Actif => "A",
            Self::Cesse => "C",
        })
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for EtatAdministratif {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
