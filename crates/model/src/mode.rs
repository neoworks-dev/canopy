use std::fs::Metadata;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};

/// Permissions in the `ls -l` style the UI shows: a type letter, then `rwx`
/// for owner, group and others. Symlinks are `L`, as Go wrote them.
pub fn mode_string(metadata: &Metadata) -> String {
    let mut text = String::with_capacity(10);
    text.push(type_letter(metadata));
    let bits = metadata.permissions().mode();
    for shift in [6, 3, 0] {
        let triple = (bits >> shift) & 0o7;
        text.push(permission_letter(triple & 0o4 != 0, 'r'));
        text.push(permission_letter(triple & 0o2 != 0, 'w'));
        text.push(permission_letter(triple & 0o1 != 0, 'x'));
    }
    text
}

fn type_letter(metadata: &Metadata) -> char {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        return 'd';
    }
    if file_type.is_symlink() {
        return 'L';
    }
    if file_type.is_fifo() {
        return 'p';
    }
    if file_type.is_socket() {
        return 'S';
    }
    if file_type.is_char_device() {
        return 'c';
    }
    if file_type.is_block_device() {
        return 'D';
    }
    '-'
}

fn permission_letter(set: bool, letter: char) -> char {
    if set {
        return letter;
    }
    '-'
}
