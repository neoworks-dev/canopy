package server

import (
	"fmt"
	"net"
	"os"

	"golang.org/x/sys/unix"
)

// peerIsSameUser reads the connecting process's credentials from the kernel. The
// socket already lives in a 0700 directory; this is the second check, and the one
// that cannot be defeated by a permissions mistake.
//
// It establishes only that the peer runs as the same user. It cannot tell one of
// that user's applications from another, so the daemon grants every accepted
// connection the same authority.
func peerIsSameUser(connection *net.UnixConn) (bool, error) {
	rawConnection, err := connection.SyscallConn()
	if err != nil {
		return false, err
	}
	credentials, err := readPeerCredentials(rawConnection)
	if err != nil {
		return false, err
	}
	return int(credentials.Uid) == os.Getuid(), nil
}

// credentialSource is the part of syscall.RawConn the credential read needs.
type credentialSource interface {
	Control(callback func(fileDescriptor uintptr)) error
}

func readPeerCredentials(connection credentialSource) (*unix.Ucred, error) {
	var credentials *unix.Ucred
	var credentialsError error
	controlError := connection.Control(func(fileDescriptor uintptr) {
		credentials, credentialsError = unix.GetsockoptUcred(
			int(fileDescriptor), unix.SOL_SOCKET, unix.SO_PEERCRED)
	})
	if controlError != nil {
		return nil, controlError
	}
	if credentialsError != nil {
		return nil, fmt.Errorf("read peer credentials: %w", credentialsError)
	}
	return credentials, nil
}
