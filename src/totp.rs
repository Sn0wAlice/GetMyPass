use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

/// Decode a base32 string (RFC 4648) into bytes
fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let input = input
        .to_uppercase()
        .replace(' ', "")
        .replace('-', "");
    let input = input.trim_end_matches('=');

    let mut bits: Vec<u8> = Vec::new();
    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u8 - b'A',
            '2'..='7' => c as u8 - b'2' + 26,
            _ => return None,
        };
        for i in (0..5).rev() {
            bits.push((val >> i) & 1);
        }
    }

    let mut bytes = Vec::new();
    for chunk in bits.chunks(8) {
        if chunk.len() == 8 {
            let byte = chunk
                .iter()
                .enumerate()
                .fold(0u8, |acc, (i, &bit)| acc | (bit << (7 - i)));
            bytes.push(byte);
        }
    }

    Some(bytes)
}

/// Generate a TOTP code (6 digits, 30s period)
/// Returns (code_string, seconds_remaining)
pub fn generate_totp(secret_base32: &str) -> Option<(String, u64)> {
    let secret = base32_decode(secret_base32)?;
    if secret.is_empty() {
        return None;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let time_step = now.as_secs() / 30;
    let remaining = 30 - (now.as_secs() % 30);

    let counter_bytes = time_step.to_be_bytes();

    let mut mac = HmacSha1::new_from_slice(&secret).ok()?;
    mac.update(&counter_bytes);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0x0f) as usize;
    let code = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);
    let code = code % 1_000_000;

    Some((format!("{:06}", code), remaining))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_decode_simple() {
        // "GEZDGNBV" is base32 for "12345"
        let result = base32_decode("GEZDGNBV");
        assert!(result.is_some());
        assert_eq!(&result.unwrap(), b"12345");
    }

    #[test]
    fn test_base32_decode_known_value() {
        // Verify deterministic decoding: same input -> same output
        let r1 = base32_decode("JBSWY3DPEHPK3PXP");
        let r2 = base32_decode("JBSWY3DPEHPK3PXP");
        assert!(r1.is_some());
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_base32_decode_lowercase() {
        let upper = base32_decode("GEZDGNBV");
        let lower = base32_decode("gezdgnbv");
        assert_eq!(upper, lower);
    }

    #[test]
    fn test_base32_decode_with_spaces() {
        let clean = base32_decode("GEZDGNBV");
        let spaced = base32_decode("GEZD GNBV");
        assert_eq!(clean, spaced);
    }

    #[test]
    fn test_base32_decode_invalid() {
        let result = base32_decode("!!!INVALID!!!");
        assert!(result.is_none());
    }

    #[test]
    fn test_base32_decode_empty() {
        let result = base32_decode("");
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_generate_totp_valid_secret() {
        // JBSWY3DPEHPK3PXP is a well-known test secret
        let result = generate_totp("JBSWY3DPEHPK3PXP");
        assert!(result.is_some());
        let (code, remaining) = result.unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        assert!(remaining > 0 && remaining <= 30);
    }

    #[test]
    fn test_generate_totp_invalid_secret() {
        let result = generate_totp("!!!INVALID!!!");
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_totp_empty_secret() {
        let result = generate_totp("");
        assert!(result.is_none());
    }

    #[test]
    fn test_totp_code_is_six_digits() {
        let result = generate_totp("GEZDGNBVGY3TQOJQ");
        assert!(result.is_some());
        let (code, _) = result.unwrap();
        assert_eq!(code.len(), 6);
        // Should be parseable as a number
        let num: u32 = code.parse().unwrap();
        assert!(num < 1_000_000);
    }
}
