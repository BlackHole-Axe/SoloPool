use sha2::{Digest, Sha256};

/// Bitcoin hashing helper.
///
/// IMPORTANT:
/// We return the raw SHA256d output bytes *without reversing*.
///
/// Bitcoin commonly *displays* hashes in reversed (big-endian) hex, but the
/// consensus engine treats the SHA256d byte array as the underlying 256-bit
/// value. Therefore, for internal comparisons against targets and for merkle
/// construction (when inputs are already in Bitcoin's internal byte order), we
/// keep the digest as produced by SHA256.
pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(&first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

/// Merkle step where inputs/outputs are **little-endian**.
pub fn merkle_step(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(left);
    buf[32..].copy_from_slice(right);
    double_sha256(&buf)
}
