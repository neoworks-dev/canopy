// Package proto defines the wire contract between the Neoworks daemon and the
// applications that embed it. Everything in here is shared by both sides, so a
// change is only safe when it is additive.
package proto

import "encoding/json"

// ProtocolVersion is bumped only for breaking wire changes. Clients and daemons
// exchange it during the handshake and refuse to talk across a mismatch.
const ProtocolVersion = 1

// Request is a single client call. Ids are client-assigned and only need to be
// unique for the lifetime of one connection.
type Request struct {
	Id     uint64          `json:"id"`
	Method string          `json:"method"`
	Params json.RawMessage `json:"params,omitempty"`
}

// Response carries either Result or Error, never both.
type Response struct {
	Id     uint64          `json:"id"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  *Error          `json:"error,omitempty"`
}

// Error is a machine-readable failure. Code is stable; Message is for logs and
// developer-facing surfaces, not for end users.
type Error struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

func (protocolError *Error) Error() string {
	return protocolError.Code + ": " + protocolError.Message
}

const (
	CodeUnknownMethod    = "unknownMethod"
	CodeInvalidParams    = "invalidParams"
	CodeProtocolMismatch = "protocolMismatch"
	CodeNotFound         = "notFound"
	CodeInternal         = "internal"
)

const (
	MethodHandshake   = "handshake"
	MethodAddRoot     = "index.addRoot"
	MethodRemoveRoot  = "index.removeRoot"
	MethodRoots       = "index.roots"
	MethodStatus      = "index.status"
	MethodRescan      = "index.rescan"
	MethodSearchQuery = "search.query"
	MethodShutdown    = "daemon.shutdown"
)
