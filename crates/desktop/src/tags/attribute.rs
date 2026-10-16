use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const ATTRIBUTE_NAME: &[u8] = b"user.xdg.tags\0";

/// The raw attribute value, `None` when the file has none or cannot carry one.
pub(super) fn read(path: &str) -> Option<String> {
    let path = c_path(path).ok()?;
    let size = unsafe { libc::getxattr(path.as_ptr(), attribute_name(), std::ptr::null_mut(), 0) };
    if size <= 0 {
        return None;
    }
    let mut stored = vec![0u8; size as usize];
    let read = unsafe { libc::getxattr(path.as_ptr(), attribute_name(), stored.as_mut_ptr().cast(), stored.len()) };
    if read < 0 {
        return None;
    }
    stored.truncate(read as usize);
    Some(String::from_utf8_lossy(&stored).into_owned())
}

pub(super) fn write(path: &str, value: &[u8]) -> io::Result<()> {
    let path = c_path(path)?;
    let status = unsafe { libc::setxattr(path.as_ptr(), attribute_name(), value.as_ptr().cast(), value.len(), 0) };
    check(status)
}

/// A file that never had the attribute is already in the state clearing asks
/// for, so its absence is not an error.
pub(super) fn clear(path: &str) -> io::Result<()> {
    let path = c_path(path)?;
    let status = unsafe { libc::removexattr(path.as_ptr(), attribute_name()) };
    match check(status) {
        Err(error) if error.raw_os_error() == Some(libc::ENODATA) => Ok(()),
        other => other,
    }
}

fn check(status: libc::c_int) -> io::Result<()> {
    if status == 0 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

fn attribute_name() -> *const libc::c_char {
    ATTRIBUTE_NAME.as_ptr().cast()
}

fn c_path(path: &str) -> io::Result<CString> {
    CString::new(Path::new(path).as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))
}
