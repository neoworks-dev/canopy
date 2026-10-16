use std::io;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

/// Reads the connecting process's credentials from the kernel. The socket
/// already lives in a 0700 directory; this is the second check, and the one that
/// cannot be defeated by a permissions mistake.
///
/// It establishes only that the peer runs as the same user. It cannot tell one
/// of that user's applications from another, so the daemon grants every accepted
/// connection the same authority.
pub fn peer_is_same_user(connection: &UnixStream) -> io::Result<bool> {
    let mut credentials: libc::ucred = unsafe { mem::zeroed() };
    let mut length = mem::size_of::<libc::ucred>() as libc::socklen_t;
    let outcome = unsafe {
        libc::getsockopt(
            connection.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    if outcome != 0 {
        let error = io::Error::last_os_error();
        return Err(io::Error::new(error.kind(), format!("read peer credentials: {error}")));
    }
    Ok(credentials.uid == unsafe { libc::getuid() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_connection_from_this_process_is_the_same_user() {
        let (first, _second) = UnixStream::pair().unwrap();
        assert!(peer_is_same_user(&first).unwrap());
    }
}
