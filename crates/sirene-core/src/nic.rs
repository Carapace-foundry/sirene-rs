use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use crate::error::NicError;

/// A 5-digit French NIC (Numéro Interne de Classement) identifier.
///
/// Represents the establishment suffix in a 14-digit SIRET.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nic(pub [u8; 5]);

impl Nic {
    /// Creates a `Nic` from a 5-byte array, checking digit validity.
    pub fn new(digits: [u8; 5]) -> Result<Self, NicError> {
        for (i, &b) in digits.iter().enumerate() {
            if !b.is_ascii_digit() {
                return Err(NicError::NonDigitCharacter {
                    position: i,
                    ch: b as char,
                });
            }
        }
        Ok(Self(digits))
    }

    /// Creates a `Nic` without verifying digits.
    pub const fn new_unchecked(digits: [u8; 5]) -> Self {
        Self(digits)
    }

    /// Parses a raw string containing exactly 5 ASCII digits.
    pub fn parse_exact(s: &str) -> Result<Self, NicError> {
        let bytes = s.as_bytes();
        if bytes.len() != 5 {
            return Err(NicError::InvalidLength {
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 5];
        arr.copy_from_slice(bytes);
        Self::new(arr)
    }

    /// Parses a NIC from a user-supplied string, ignoring spaces, dots, and hyphens.
    pub fn parse(s: &str) -> Result<Self, NicError> {
        let mut arr = [0u8; 5];
        let mut count = 0;

        for (pos, b) in s.bytes().enumerate() {
            match b {
                b' ' | b'.' | b'-' | b'_' => continue,
                b'0'..=b'9' => {
                    if count < 5 {
                        arr[count] = b;
                        count += 1;
                    } else {
                        count += 1;
                    }
                }
                _ => {
                    return Err(NicError::NonDigitCharacter {
                        position: pos,
                        ch: b as char,
                    });
                }
            }
        }

        if count != 5 {
            return Err(NicError::InvalidLength { actual: count });
        }

        Self::new(arr)
    }

    /// Returns the underlying 5-byte array.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 5] {
        &self.0
    }

    /// Returns a string slice representation of the 5-digit NIC.
    #[inline]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl Deref for Nic {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Nic {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for Nic {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Nic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Nic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Nic(\"{}\")", self.as_str())
    }
}

impl FromStr for Nic {
    type Err = NicError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Nic {
    type Error = NicError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<[u8; 5]> for Nic {
    type Error = NicError;

    #[inline]
    fn try_from(digits: [u8; 5]) -> Result<Self, Self::Error> {
        Self::new(digits)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Nic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Nic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}
