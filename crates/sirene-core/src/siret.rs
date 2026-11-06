use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use crate::error::SiretError;
use crate::luhn::{is_la_poste_siren_bytes, sum_digits_multiple_of_5, validate_siret_bytes};
use crate::nic::Nic;
use crate::siren::Siren;

/// A strictly validated 14-digit French SIRET establishment identifier.
///
/// Guaranteed to:
/// - Be exactly 14 ASCII decimal digits (`'0'..'9'`).
/// - Contain a valid 9-digit SIREN prefix.
/// - Contain a valid 5-digit NIC suffix.
/// - Pass the official Luhn modulo 10 validation (or the official La Poste
///   derogation rule where sum of digits % 5 == 0 for SIREN `356 000 000`).
/// - Fit in a fixed-size `[u8; 14]` (14 bytes, zero heap allocation).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Siret(pub [u8; 14]);

impl Siret {
    /// Creates a `Siret` from a 14-byte array, checking digit validity, embedded SIREN and SIRET checksum.
    pub fn new(digits: [u8; 14]) -> Result<Self, SiretError> {
        for (i, &b) in digits.iter().enumerate() {
            if !b.is_ascii_digit() {
                return Err(SiretError::NonDigitCharacter {
                    position: i,
                    ch: b as char,
                });
            }
        }

        // Verify embedded SIREN (first 9 digits)
        let mut siren_bytes = [0u8; 9];
        siren_bytes.copy_from_slice(&digits[0..9]);
        Siren::new(siren_bytes).map_err(SiretError::InvalidSiren)?;

        // Verify SIRET checksum
        if !validate_siret_bytes(&digits) {
            let str_val = match std::str::from_utf8(&digits) {
                Ok(s) => s.to_string(),
                Err(_) => String::new(),
            };

            if is_la_poste_siren_bytes(&siren_bytes) {
                return Err(SiretError::InvalidLaPosteChecksum { siret: str_val });
            }
            return Err(SiretError::InvalidLuhnChecksum { siret: str_val });
        }

        Ok(Self(digits))
    }

    /// Creates a `Siret` without verification.
    pub const fn new_unchecked(digits: [u8; 14]) -> Self {
        Self(digits)
    }

    /// Constructs a `Siret` from a validated `Siren` and `Nic`.
    pub fn from_siren_and_nic(siren: Siren, nic: Nic) -> Result<Self, SiretError> {
        let mut digits = [0u8; 14];
        digits[0..9].copy_from_slice(siren.as_bytes());
        digits[9..14].copy_from_slice(nic.as_bytes());
        Self::new(digits)
    }

    /// Parses a raw string containing exactly 14 ASCII digits.
    pub fn parse_exact(s: &str) -> Result<Self, SiretError> {
        let bytes = s.as_bytes();
        if bytes.len() != 14 {
            return Err(SiretError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 14];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }

    /// Parses a SIRET from a user-supplied string, ignoring ASCII and Unicode whitespace, dots, and hyphens.
    pub fn parse(s: &str) -> Result<Self, SiretError> {
        let mut arr = [0u8; 14];
        let mut count = 0;

        for (pos, ch) in s.char_indices() {
            if ch.is_whitespace() || ch == '.' || ch == '-' || ch == '_' {
                continue;
            }
            if ch.is_ascii_digit() {
                if count < 14 {
                    arr[count] = ch as u8;
                    count += 1;
                } else {
                    count += 1;
                }
            } else {
                return Err(SiretError::NonDigitCharacter { position: pos, ch });
            }
        }

        if count != 14 {
            return Err(SiretError::InvalidLength { actual: count });
        }

        Self::new(arr)
    }

    /// Extracts the parent 9-digit `Siren`.
    #[inline]
    pub fn siren(&self) -> Siren {
        let mut arr = [0u8; 9];
        arr.copy_from_slice(&self.0[0..9]);
        Siren(arr)
    }

    /// Extracts the 5-digit `Nic` (establishment code).
    #[inline]
    pub fn nic(&self) -> Nic {
        let mut arr = [0u8; 5];
        arr.copy_from_slice(&self.0[9..14]);
        Nic(arr)
    }

    /// Returns the underlying 14-byte array.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 14] {
        &self.0
    }

    /// Returns a string slice representation of the 14-digit SIRET.
    #[inline]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }

    /// Formats the SIRET in standard 3-3-3-5 spaced format (e.g. `"732 829 320 00074"`).
    pub fn formatted(&self) -> String {
        let s = self.as_str();
        let mut out = String::with_capacity(17);
        if s.len() == 14 && s.is_char_boundary(3) && s.is_char_boundary(6) && s.is_char_boundary(9)
        {
            out.push_str(&s[0..3]);
            out.push(' ');
            out.push_str(&s[3..6]);
            out.push(' ');
            out.push_str(&s[6..9]);
            out.push(' ');
            out.push_str(&s[9..14]);
        }
        out
    }

    /// Returns `true` if this SIRET belongs to La Poste (`356 000 000`).
    #[inline]
    pub fn is_la_poste(&self) -> bool {
        let mut arr = [0u8; 9];
        arr.copy_from_slice(&self.0[0..9]);
        is_la_poste_siren_bytes(&arr)
    }

    /// Returns `true` if this establishment is verified via the La Poste derogation rule
    /// (sum of digits % 5 == 0).
    #[inline]
    pub fn is_la_poste_derogation(&self) -> bool {
        self.is_la_poste() && sum_digits_multiple_of_5(&self.0)
    }
}

impl Deref for Siret {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Siret {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for Siret {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Siret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Siret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Siret(\"{}\")", self.as_str())
    }
}

impl FromStr for Siret {
    type Err = SiretError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Siret {
    type Error = SiretError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&[u8]> for Siret {
    type Error = SiretError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 14 {
            return Err(SiretError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 14];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }
}

impl TryFrom<[u8; 14]> for Siret {
    type Error = SiretError;

    #[inline]
    fn try_from(digits: [u8; 14]) -> Result<Self, Self::Error> {
        Self::new(digits)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Siret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Siret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
