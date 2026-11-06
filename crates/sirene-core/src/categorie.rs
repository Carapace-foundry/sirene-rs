use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use crate::error::CategorieJuridiqueError;

/// INSEE Legal Category (Nomenclature des Catégories Juridiques).
///
/// 4-digit hierarchical code structured in 3 levels:
/// - Level I: 1 digit (e.g. `"5"` = Commercial companies).
/// - Level II: 2 digits (e.g. `"57"` = Simplified joint-stock company / SAS).
/// - Level III: 4 digits (e.g. `"5710"` = Single-shareholder SAS / SASU).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CategorieJuridique(pub [u8; 4]);

impl CategorieJuridique {
    /// Creates a `CategorieJuridique` from a 4-byte digit array.
    pub fn new(digits: [u8; 4]) -> Result<Self, CategorieJuridiqueError> {
        for (i, &b) in digits.iter().enumerate() {
            if !b.is_ascii_digit() {
                return Err(CategorieJuridiqueError::NonDigitCharacter {
                    position: i,
                    ch: b as char,
                });
            }
        }
        Ok(Self(digits))
    }

    /// Creates a `CategorieJuridique` without verification.
    pub const fn new_unchecked(digits: [u8; 4]) -> Self {
        Self(digits)
    }

    /// Parses a 4-digit string into a `CategorieJuridique`.
    pub fn parse(s: &str) -> Result<Self, CategorieJuridiqueError> {
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() != 4 {
            return Err(CategorieJuridiqueError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 4];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }

    /// Returns the Level I code (1 digit, e.g. `"5"`).
    #[inline]
    pub fn niveau1(&self) -> &str {
        std::str::from_utf8(&self.0[0..1]).unwrap_or_default()
    }

    /// Returns the Level II code (2 digits, e.g. `"57"`).
    #[inline]
    pub fn niveau2(&self) -> &str {
        std::str::from_utf8(&self.0[0..2]).unwrap_or_default()
    }

    /// Returns the Level III code (4 digits, e.g. `"5710"`).
    #[inline]
    pub fn niveau3(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }

    /// Returns true if this legal category represents a natural person (Level 1 == "1").
    #[inline]
    pub fn is_personne_physique(&self) -> bool {
        self.0[0] == b'1'
    }

    /// Returns true if this legal category represents a commercial company (Level 1 == "5").
    #[inline]
    pub fn is_societe_commerciale(&self) -> bool {
        self.0[0] == b'5'
    }

    /// Returns true if this legal category represents a public law entity (Level 1 == "7").
    #[inline]
    pub fn is_personne_morale_droit_public(&self) -> bool {
        self.0[0] == b'7'
    }

    /// Returns a human-friendly summary label for the most common legal categories.
    pub fn libelle_sommaire(&self) -> Option<&'static str> {
        match &self.0 {
            b"1000" => Some("Entrepreneur individuel"),
            b"5498" => Some("SARL de droit commun"),
            b"5499" => Some("Société à responsabilité limitée (sans autre indication)"),
            b"5710" => Some("SAS, société par actions simplifiée à associé unique ou société par actions simplifiée unipersonnelle"),
            b"5720" => Some("Société par actions simplifiée"),
            b"5599" => Some("SA à conseil d'administration (s.a.i.)"),
            b"6540" => Some("Société civile immobilière (SCI)"),
            b"5202" => Some("Société en nom collectif (SNC)"),
            b"6599" => Some("Autre société civile"),
            b"9220" => Some("Association déclarée"),
            b"9221" => Some("Association déclarée d'utilité publique"),
            b"7389" => Some("Établissement public local"),
            b"7172" => Some("Commune et commune nouvelle"),
            _ => None,
        }
    }

    /// Returns the underlying 4 bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Returns a string slice of the 4-digit code.
    #[inline]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl Deref for CategorieJuridique {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for CategorieJuridique {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for CategorieJuridique {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for CategorieJuridique {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for CategorieJuridique {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CategorieJuridique(\"{}\")", self.as_str())
    }
}

impl FromStr for CategorieJuridique {
    type Err = CategorieJuridiqueError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for CategorieJuridique {
    type Error = CategorieJuridiqueError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CategorieJuridique {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CategorieJuridique {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
