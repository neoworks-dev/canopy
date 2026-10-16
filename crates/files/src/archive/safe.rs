use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::paths;

/// Where a member lands under a destination folder, refusing any name that
/// would not stay inside it. An archive can name anything it likes, including a
/// path that climbs out of the destination, and unpacking that would write
/// wherever the archive said.
pub fn destination_for(destination: &Path, member_name: &str) -> Result<PathBuf> {
    let cleaned = paths::clean_slash_path(member_name);
    if cleaned.starts_with('/') || cleaned == ".." || cleaned.starts_with("../") {
        return Err(Error::EscapingMember);
    }
    let target = paths::clean(&destination.join(&cleaned));
    let inside_prefix = format!("{}/", destination.to_string_lossy());
    if !target.to_string_lossy().starts_with(&inside_prefix) {
        return Err(Error::EscapingMember);
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_pointing_outside_the_folder_is_refused() {
        for name in ["../outside.txt", "/etc/passwd", "nested/../../outside.txt"] {
            let result = destination_for(Path::new("/home/moritz/unpacked"), name);
            assert!(matches!(result, Err(Error::EscapingMember)), "{name}");
        }
    }

    #[test]
    fn a_member_staying_inside_lands_under_the_folder() {
        let target = destination_for(Path::new("/home/moritz/unpacked"), "project/top.txt").unwrap();
        assert_eq!(target, PathBuf::from("/home/moritz/unpacked/project/top.txt"));
    }
}
