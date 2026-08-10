//go:build unix && !linux

package server

import (
	"errors"
	"net"
)

// peerIsSameUser fails closed on platforms where the credential check is not
// implemented yet. Accepting unverified peers would be worse than refusing to
// run, so ports must add a real implementation rather than a stub that allows.
func peerIsSameUser(connection *net.UnixConn) (bool, error) {
	return false, errors.New("server: peer credential check unavailable on this platform")
}
