use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::hex;

/// How much is read from each end of a file for its signature. Files that
/// differ usually differ near one end or the other, so this settles most of
/// them without reading what lies between.
pub(super) const SIGNATURE_LENGTH: u64 = 4096;

/// Digests both ends of a file. One small enough to be covered by a single read
/// is simply digested whole.
pub(super) fn signature_of(path: &Path, size: i64) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    digest_section(&mut digest, &mut file, 0, SIGNATURE_LENGTH)?;
    if size as u64 > SIGNATURE_LENGTH {
        digest_section(&mut digest, &mut file, size as u64 - SIGNATURE_LENGTH, SIGNATURE_LENGTH)?;
    }
    Ok(hex::encode(&digest.finalize()))
}

/// Digests a whole file, which is what settles the files a matching signature
/// leaves indistinguishable.
pub(super) fn contents_of(path: &Path, _size: i64) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    std::io::copy(&mut file, &mut digest)?;
    Ok(hex::encode(&digest.finalize()))
}

fn digest_section(digest: &mut Sha256, file: &mut File, offset: u64, length: u64) -> Result<()> {
    file.seek(SeekFrom::Start(offset))?;
    std::io::copy(&mut file.take(length), digest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_file_is_digested_whole() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("small.txt");
        std::fs::write(&path, "abc").unwrap();

        assert_eq!(signature_of(&path, 3).unwrap(), contents_of(&path, 3).unwrap());
        assert_eq!(contents_of(&path, 3).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn the_signature_ignores_the_middle_of_a_large_file() {
        let directory = tempfile::tempdir().unwrap();
        let end = "e".repeat(SIGNATURE_LENGTH as usize);
        let first = directory.path().join("first.bin");
        let second = directory.path().join("second.bin");
        std::fs::write(&first, format!("{end}middle one{end}")).unwrap();
        std::fs::write(&second, format!("{end}middle two{end}")).unwrap();
        let size = (2 * SIGNATURE_LENGTH + 10) as i64;

        assert_eq!(signature_of(&first, size).unwrap(), signature_of(&second, size).unwrap());
        assert_ne!(contents_of(&first, size).unwrap(), contents_of(&second, size).unwrap());
    }
}
