//! High-performance, allocation-free, bitwise/lookup-based Luhn validation
//! conforming to French official SIRENE / INSEE rules.

/// Doubled digit lookup table for Luhn algorithm:
/// For digit `d` in 0..=9: `if 2*d > 9 { 2*d - 9 } else { 2*d }`.
const DOUBLED: [u8; 10] = [0, 2, 4, 6, 8, 1, 3, 5, 7, 9];

/// SIREN number of La Poste (Group SIREN: 356 000 000).
pub const LA_POSTE_SIREN: &[u8; 9] = b"356000000";

/// Computes the standard Luhn modulo 10 checksum over an ASCII digit slice.
///
/// Returns `true` if and only if all bytes are ASCII digits `b'0'..=b'9'`
/// and the checksum modulo 10 equals 0.
///
/// Evaluates without memory allocation in O(N) time with constant-time lookup.
#[inline]
pub fn luhn_checksum_ascii(digits: &[u8]) -> bool {
    let len = digits.len();
    if len == 0 {
        return false;
    }

    let mut sum: u32 = 0;
    for (i, &b) in digits.iter().enumerate() {
        let digit = b.wrapping_sub(b'0');
        if digit > 9 {
            return false;
        }

        // 1-based position from the right: len - i
        // If (len - i) is even, position is even from right -> double the digit
        let is_even_from_right = ((len - 1 - i) & 1) == 1;
        let term = if is_even_from_right {
            DOUBLED[digit as usize]
        } else {
            digit
        };
        sum += term as u32;
    }

    sum % 10 == 0
}

/// Checks whether the 9-digit SIREN corresponds to La Poste (356 000 000).
#[inline]
pub fn is_la_poste_siren_bytes(siren: &[u8; 9]) -> bool {
    siren == LA_POSTE_SIREN
}

/// Computes the sum of all digits and returns `true` if it is a multiple of 5.
///
/// This is the official INSEE derogation rule for secondary establishments
/// of La Poste whose NIC exhausted the standard Luhn sequence.
#[inline]
pub fn sum_digits_multiple_of_5(digits: &[u8; 14]) -> bool {
    let mut sum: u32 = 0;
    for &b in digits {
        let digit = b.wrapping_sub(b'0');
        if digit > 9 {
            return false;
        }
        sum += digit as u32;
    }
    sum % 5 == 0
}

/// Validates a 9-digit SIREN byte array using the Luhn algorithm.
#[inline]
pub fn validate_siren_bytes(digits: &[u8; 9]) -> bool {
    luhn_checksum_ascii(digits)
}

/// Validates a 14-digit SIRET byte array.
///
/// Evaluates:
/// 1. Standard Luhn modulo 10 algorithm.
/// 2. If the entity belongs to La Poste (SIREN `356000000`), accepts either
///    standard Luhn validity or the derogation rule (sum of digits % 5 == 0).
#[inline]
pub fn validate_siret_bytes(digits: &[u8; 14]) -> bool {
    let is_la_poste = &digits[0..9] == LA_POSTE_SIREN;
    if is_la_poste {
        luhn_checksum_ascii(digits) || sum_digits_multiple_of_5(digits)
    } else {
        luhn_checksum_ascii(digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_siren() {
        // INSEE: 732 829 320
        assert!(validate_siren_bytes(b"732829320"));
        // Google France: 443 061 841
        assert!(validate_siren_bytes(b"443061841"));
        // La Poste: 356 000 000
        assert!(validate_siren_bytes(b"356000000"));
    }

    #[test]
    fn test_invalid_siren() {
        // Typo in last digit
        assert!(!validate_siren_bytes(b"732829321"));
        // Letters
        assert!(!validate_siren_bytes(b"73282932A"));
    }

    #[test]
    fn test_valid_siret_standard() {
        // INSEE Paris: 732 829 320 00074
        assert!(validate_siret_bytes(b"73282932000074"));
        // Google France Paris: 443 061 841 00047
        assert!(validate_siret_bytes(b"44306184100047"));
    }

    #[test]
    fn test_la_poste_siret_exception() {
        // La Poste Siège (Luhn standard): 356 000 000 00048
        assert!(validate_siret_bytes(b"35600000000048"));

        // La Poste Établissement secondaire: 356 000 000 38497
        // Standard Luhn sum is 44 (not 0 mod 10), but sum of digits is 45 (0 mod 5).
        assert!(!luhn_checksum_ascii(b"35600000038497"));
        assert!(sum_digits_multiple_of_5(b"35600000038497"));
        assert!(validate_siret_bytes(b"35600000038497"));
    }

    #[test]
    fn test_invalid_siret() {
        // Wrong Luhn
        assert!(!validate_siret_bytes(b"73282932000075"));
        // Non-digit
        assert!(!validate_siret_bytes(b"7328293200007A"));
    }
}
