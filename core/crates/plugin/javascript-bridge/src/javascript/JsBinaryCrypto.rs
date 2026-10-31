use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use base64::{engine::general_purpose::STANDARD, Engine};
use md5::{Digest, Md5};

/// Dispatches binary cryptography without converting media bytes into UTF-8 text.
pub fn execute(operation: &str, args: &[String]) -> Result<String, String> {
    match (operation, args) {
        ("randomHex", [length]) => random_hex(length),
        ("md5", [data]) => digest_md5(data),
        ("encryptAesEcb", [data, key]) => encrypt_aes_ecb(data, key),
        ("decryptAesEcb", [data, key]) => decrypt_aes_ecb(data, key),
        _ => Err(format!("Invalid binary crypto call: {operation}")),
    }
}

/// Generates bounded cryptographically secure random bytes as hexadecimal.
fn random_hex(length: &str) -> Result<String, String> {
    let length = length.parse::<usize>().map_err(|error| error.to_string())?;
    if !(1..=64).contains(&length) {
        return Err("Random byte count must be between 1 and 64".into());
    }

    let mut bytes = vec![0; length];
    getrandom::getrandom(&mut bytes).map_err(|error| error.to_string())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Computes the protocol-required MD5 digest over decoded binary contents.
fn digest_md5(data: &str) -> Result<String, String> {
    let bytes = decode_base64(data)?;
    Ok(format!("{:x}", Md5::digest(bytes)))
}

/// Decodes the explicit base64 representation shared by file and network hosts.
fn decode_base64(data: &str) -> Result<Vec<u8>, String> {
    STANDARD.decode(data).map_err(|error| error.to_string())
}

/// Builds an AES-128 cipher from an exact hexadecimal protocol key.
fn aes_cipher(key: &str) -> Result<Aes128, String> {
    if key.len() != 32 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("AES-128 key must contain exactly 32 hexadecimal characters".into());
    }

    let bytes = (0..32)
        .step_by(2)
        .map(|index| u8::from_str_radix(&key[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Aes128::new_from_slice(&bytes).map_err(|error| error.to_string())
}

/// Encrypts bytes with AES-128-ECB and mandatory PKCS#7 padding.
fn encrypt_aes_ecb(data: &str, key: &str) -> Result<String, String> {
    let cipher = aes_cipher(key)?;
    let mut bytes = decode_base64(data)?;
    let padding = 16 - bytes.len() % 16;
    bytes.resize(bytes.len() + padding, padding as u8);

    for block in bytes.chunks_exact_mut(16) {
        cipher.encrypt_block(GenericArray::from_mut_slice(block));
    }
    Ok(STANDARD.encode(bytes))
}

/// Decrypts media bytes and validates every byte of the PKCS#7 padding.
fn decrypt_aes_ecb(data: &str, key: &str) -> Result<String, String> {
    let cipher = aes_cipher(key)?;
    let mut bytes = decode_base64(data)?;
    if bytes.is_empty() || bytes.len() % 16 != 0 {
        return Err("Invalid padded AES ciphertext length".into());
    }

    for block in bytes.chunks_exact_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(block));
    }

    let padding = usize::from(*bytes.last().ok_or("Empty AES plaintext")?);
    if !(1..=16).contains(&padding)
        || !bytes[bytes.len() - padding..]
            .iter()
            .all(|byte| usize::from(*byte) == padding)
    {
        return Err("Invalid AES PKCS#7 padding".into());
    }

    bytes.truncate(bytes.len() - padding);
    Ok(STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "000102030405060708090a0b0c0d0e0f";

    /// Checks the NIST AES-128 ECB block before the additional padding block.
    #[test]
    fn encrypts_known_binary_vector() {
        let data = STANDARD.encode([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ]);
        let encrypted = encrypt_aes_ecb(&data, KEY).unwrap();
        let bytes = STANDARD.decode(&encrypted).unwrap();
        assert_eq!(
            &bytes[..16],
            &[
                0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
                0xc5, 0x5a
            ]
        );
        assert_eq!(decrypt_aes_ecb(&encrypted, KEY).unwrap(), data);
    }

    /// Covers empty, unaligned, aligned and non-UTF-8 file bodies.
    #[test]
    fn round_trips_binary_media() {
        for length in [0, 1, 15, 16, 17, 255] {
            let data = STANDARD.encode(vec![0xff; length]);
            let encrypted = encrypt_aes_ecb(&data, KEY).unwrap();
            assert_eq!(decrypt_aes_ecb(&encrypted, KEY).unwrap(), data);
        }
    }

    /// Rejects malformed keys, ciphertexts, argument counts and padding.
    #[test]
    fn rejects_invalid_binary_calls() {
        assert!(execute("md5", &[]).is_err());
        assert!(execute("unknown", &["".into()]).is_err());
        assert!(encrypt_aes_ecb("?", KEY).is_err());
        assert!(encrypt_aes_ecb("", "éééééééééééééééé").is_err());
        assert!(decrypt_aes_ecb("", KEY).is_err());
        let cipher = aes_cipher(KEY).unwrap();
        for tail in [vec![0u8; 16], vec![17u8; 16], {
            let mut bytes = vec![2u8; 16];
            bytes[14] = 3;
            bytes
        }] {
            let mut bytes = tail;
            cipher.encrypt_block(GenericArray::from_mut_slice(&mut bytes));
            assert!(decrypt_aes_ecb(&STANDARD.encode(bytes), KEY).is_err());
        }
    }

    /// Hashes binary contents and obtains independently generated random bytes.
    #[test]
    fn hashes_and_generates_secure_bytes() {
        assert_eq!(
            digest_md5("aGVsbG8=").unwrap(),
            "5d41402abc4b2a76b9719d911017c592"
        );
        let first = random_hex("16").unwrap();
        let second = random_hex("16").unwrap();
        assert_eq!(first.len(), 32);
        assert_ne!(first, second);
        assert!(random_hex("0").is_err());
        assert!(random_hex("65").is_err());
    }
}
