package server

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/neoworks-dev/neoworksd/internal/index"
	"github.com/neoworks-dev/neoworksd/proto"
)

const idleCheckInterval = 30 * time.Second

type Options struct {
	SocketPath    string
	IdleTimeout   time.Duration
	DaemonVersion string
	IndexPath     string
	Logger        *slog.Logger
}

// Server exposes the indexer over a unix socket. One accepted connection is one
// application; the daemon exits on its own once the last one goes away.
type Server struct {
	indexer  *index.Indexer
	options  Options
	handlers map[string]handler
	logger   *slog.Logger

	stateMutex        sync.Mutex
	activeConnections int
	lastActivity      time.Time
	stopServing       func()
}

// connectionSession is per-connection state. The handshake gate keeps a client
// from using methods before both sides agree on the protocol version.
type connectionSession struct {
	handshakeComplete bool
	clientName        string
}

type handler func(context.Context, json.RawMessage, *connectionSession) (any, error)

func New(indexer *index.Indexer, options Options) *Server {
	server := &Server{
		indexer:      indexer,
		options:      options,
		logger:       options.Logger,
		lastActivity: time.Now(),
	}
	server.handlers = server.buildHandlers()
	return server
}

// Serve listens until the context ends, a shutdown is requested, or the idle
// timeout expires with no clients connected.
func (server *Server) Serve(runContext context.Context) error {
	listener, err := server.listen()
	if err != nil {
		return err
	}
	defer listener.Close()
	defer os.Remove(server.options.SocketPath)

	serveContext, stopServing := context.WithCancel(runContext)
	defer stopServing()
	server.recordStopFunction(stopServing)

	go closeListenerOnDone(serveContext, listener)
	go server.acceptConnections(serveContext, listener, stopServing)
	go server.watchForIdleness(serveContext, stopServing)

	<-serveContext.Done()
	server.logger.Info("daemon stopping")
	return nil
}

// listen replaces any socket left behind by a crashed daemon. This is only safe
// because the caller already holds the daemon lock, which proves no live daemon
// owns that path.
func (server *Server) listen() (*net.UnixListener, error) {
	if err := proto.EnsurePrivateDirectory(filepath.Dir(server.options.SocketPath)); err != nil {
		return nil, err
	}
	if err := removeStaleSocket(server.options.SocketPath); err != nil {
		return nil, err
	}
	address := &net.UnixAddr{Name: server.options.SocketPath, Net: "unix"}
	listener, err := net.ListenUnix("unix", address)
	if err != nil {
		return nil, fmt.Errorf("listen on %s: %w", server.options.SocketPath, err)
	}
	if err := os.Chmod(server.options.SocketPath, 0o600); err != nil {
		listener.Close()
		return nil, err
	}
	return listener, nil
}

func (server *Server) acceptConnections(serveContext context.Context, listener *net.UnixListener, stopServing func()) {
	defer stopServing()
	for {
		connection, err := listener.AcceptUnix()
		if err != nil {
			return
		}
		go server.handleConnection(serveContext, connection)
	}
}

func (server *Server) handleConnection(serveContext context.Context, connection *net.UnixConn) {
	defer connection.Close()

	if !server.acceptPeer(connection) {
		return
	}
	server.connectionOpened()
	defer server.connectionClosed()

	session := &connectionSession{}
	reader := bufio.NewReader(connection)
	for server.serveRequest(serveContext, reader, connection, session) {
	}
}

func (server *Server) acceptPeer(connection *net.UnixConn) bool {
	sameUser, err := peerIsSameUser(connection)
	if err != nil {
		server.logger.Warn("cannot verify peer, refusing connection", "error", err)
		return false
	}
	if !sameUser {
		server.logger.Warn("refused connection from another user")
		return false
	}
	return true
}

// serveRequest handles one request and reports whether the connection stays open.
func (server *Server) serveRequest(serveContext context.Context, reader io.Reader, writer io.Writer, session *connectionSession) bool {
	payload, err := proto.ReadMessage(reader)
	if err != nil {
		return false
	}
	server.noteActivity()
	response := server.buildResponse(serveContext, payload, session)
	return proto.WriteMessage(writer, response) == nil
}

func (server *Server) buildResponse(serveContext context.Context, payload []byte, session *connectionSession) []byte {
	var request proto.Request
	if err := json.Unmarshal(payload, &request); err != nil {
		return encodeResponse(proto.Response{Error: newError(proto.CodeInvalidParams, err.Error())})
	}
	result, err := server.invoke(serveContext, request, session)
	if err != nil {
		return encodeResponse(proto.Response{Id: request.Id, Error: translateError(err)})
	}
	return encodeResponse(successResponse(request.Id, result))
}

func (server *Server) invoke(serveContext context.Context, request proto.Request, session *connectionSession) (any, error) {
	selectedHandler, known := server.handlers[request.Method]
	if !known {
		return nil, newError(proto.CodeUnknownMethod, request.Method)
	}
	if err := requireHandshake(request.Method, session); err != nil {
		return nil, err
	}
	return selectedHandler(serveContext, request.Params, session)
}

func (server *Server) connectionOpened() {
	server.stateMutex.Lock()
	defer server.stateMutex.Unlock()
	server.activeConnections++
	server.lastActivity = time.Now()
}

func (server *Server) connectionClosed() {
	server.stateMutex.Lock()
	defer server.stateMutex.Unlock()
	server.activeConnections--
	server.lastActivity = time.Now()
}

func (server *Server) recordStopFunction(stopServing func()) {
	server.stateMutex.Lock()
	defer server.stateMutex.Unlock()
	server.stopServing = stopServing
}

// requestStop ends serving from a handler. It is a no-op before Serve runs.
func (server *Server) requestStop() {
	server.stateMutex.Lock()
	stopServing := server.stopServing
	server.stateMutex.Unlock()

	if stopServing == nil {
		return
	}
	stopServing()
}

func (server *Server) noteActivity() {
	server.stateMutex.Lock()
	defer server.stateMutex.Unlock()
	server.lastActivity = time.Now()
}

func (server *Server) watchForIdleness(serveContext context.Context, stopServing func()) {
	if server.options.IdleTimeout <= 0 {
		return
	}
	ticker := time.NewTicker(idleCheckInterval)
	defer ticker.Stop()

	for waitForTick(serveContext, ticker.C) {
		server.stopIfIdle(stopServing)
	}
}

// stopIfIdle exits only when nothing is connected and no scan is in flight, so a
// first-run crawl is never abandoned halfway.
func (server *Server) stopIfIdle(stopServing func()) {
	if !server.isIdle() {
		return
	}
	if server.isScanning() {
		return
	}
	server.logger.Info("idle timeout reached", "timeout", server.options.IdleTimeout)
	stopServing()
}

func (server *Server) isIdle() bool {
	server.stateMutex.Lock()
	defer server.stateMutex.Unlock()
	if server.activeConnections > 0 {
		return false
	}
	return time.Since(server.lastActivity) > server.options.IdleTimeout
}

func (server *Server) isScanning() bool {
	status, err := server.indexer.Status()
	if err != nil {
		return false
	}
	return status.Scanning
}

// requireHandshake gates every method but the handshake itself.
func requireHandshake(method string, session *connectionSession) error {
	if method == proto.MethodHandshake {
		return nil
	}
	if session.handshakeComplete {
		return nil
	}
	return newError(proto.CodeProtocolMismatch, "handshake required before "+method)
}

func newError(code string, message string) *proto.Error {
	return &proto.Error{Code: code, Message: message}
}

func translateError(err error) *proto.Error {
	var protocolError *proto.Error
	if errors.As(err, &protocolError) {
		return protocolError
	}
	if errors.Is(err, index.ErrRootNotFound) || errors.Is(err, os.ErrNotExist) {
		return newError(proto.CodeNotFound, err.Error())
	}
	return newError(proto.CodeInternal, err.Error())
}

func successResponse(requestId uint64, result any) proto.Response {
	encoded, err := json.Marshal(result)
	if err != nil {
		return proto.Response{Id: requestId, Error: newError(proto.CodeInternal, err.Error())}
	}
	return proto.Response{Id: requestId, Result: encoded}
}

// encodeResponse cannot fail in practice: every field is already valid JSON.
func encodeResponse(response proto.Response) []byte {
	encoded, err := json.Marshal(response)
	if err != nil {
		return []byte(`{"id":0,"error":{"code":"internal","message":"response encoding failed"}}`)
	}
	return encoded
}

func removeStaleSocket(socketPath string) error {
	err := os.Remove(socketPath)
	if err == nil || errors.Is(err, os.ErrNotExist) {
		return nil
	}
	return fmt.Errorf("remove stale socket: %w", err)
}

func closeListenerOnDone(serveContext context.Context, listener *net.UnixListener) {
	<-serveContext.Done()
	listener.Close()
}

func waitForTick(serveContext context.Context, tick <-chan time.Time) bool {
	select {
	case <-serveContext.Done():
		return false
	case <-tick:
		return true
	}
}
