use blake2::Digest as _;

pub type Checksum = [u8; 64];

pub fn compute_checksum(data: &[u8]) -> Checksum {
    blake2::Blake2b512::digest(data).try_into().expect("blake2b output is 64 byte long")
}

pub struct Mismatch {
    expected: Checksum,
    actual: Checksum,
}

impl core::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Checksum mismatch: expected {}, got {}",
            hexhex::Hex::new(self.expected),
            hexhex::Hex::new(self.actual)
        )
    }
}

impl core::fmt::Debug for Mismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        <Self as core::fmt::Display>::fmt(self, f)
    }
}

impl std::error::Error for Mismatch {}

#[allow(clippy::result_large_err)]
pub fn check(data: &[u8], expected: &Checksum) -> Result<(), Mismatch> {
    let actual = compute_checksum(data);
    if &actual != expected {
        Err(Mismatch { expected: *expected, actual })
    } else {
        Ok(())
    }
}
