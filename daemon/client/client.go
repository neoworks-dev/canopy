// Package client connects an application to the shared Neoworks daemon. Callers
// never manage the daemon's lifetime: Connect starts one if none is running, and
// the daemon exits by itself once the last application disconnects.
package client

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
	"sync"
	"time"

	"github.com/neoworks-dev/neoworksd/proto"
)

const defaultSpawnTimeout = 10 * time.Second

type Options struct {
	// ClientName identifies the application in daemon logs.
	ClientName string
	// SocketPath and BinaryPath are empty in normal use; the defaults follow the
	// XDG directories the daemon also uses.
	SocketPath   string
	BinaryPath   string
	SpawnTimeout time.Duration
}

// Client is a connection to the daemon. Calls are serialized, so one Client is
// safe to share across an application's goroutines.
type Client struct {
	options Options

	mutex         sync.Mutex
	connection    net.Conn
	reader        *bufio.Reader
	nextRequestId uint64
	daemonVersion string
}

// Connect returns a connected client, spawning the daemon when necessary.
func Connect(options Options) (*Client, error) {
	client := &Client{options: withDefaults(options)}
	client.mutex.Lock()
	defer client.mutex.Unlock()

	if err := client.establishConnection(); err != nil {
		return nil, err
	}
	return client, nil
}

// DaemonVersion is the version reported by the handshake.
func (client *Client) DaemonVersion() string {
	client.mutex.Lock()
	defer client.mutex.Unlock()
	return client.daemonVersion
}

func (client *Client) Close() error {
	client.mutex.Lock()
	defer client.mutex.Unlock()

	if client.connection == nil {
		return nil
	}
	err := client.connection.Close()
	client.connection = nil
	client.reader = nil
	return err
}

// AddRoot registers a directory tree for indexing. It returns as soon as the
// daemon accepts the root; the crawl continues in the background.
func (client *Client) AddRoot(path string) error {
	return client.call(proto.MethodAddRoot, proto.AddRootParams{Path: path}, nil)
}

func (client *Client) RemoveRoot(path string) error {
	return client.call(proto.MethodRemoveRoot, proto.RemoveRootParams{Path: path}, nil)
}

func (client *Client) Roots() ([]proto.Root, error) {
	var result proto.RootsResult
	if err := client.call(proto.MethodRoots, nil, &result); err != nil {
		return nil, err
	}
	return result.Roots, nil
}

func (client *Client) Status() (proto.StatusResult, error) {
	var result proto.StatusResult
	err := client.call(proto.MethodStatus, nil, &result)
	return result, err
}

// Rescan reindexes one root, or every root when path is empty.
func (client *Client) Rescan(path string) error {
	return client.call(proto.MethodRescan, proto.RescanParams{Path: path}, nil)
}

func (client *Client) Search(params proto.SearchParams) (proto.SearchResult, error) {
	var result proto.SearchResult
	err := client.call(proto.MethodSearchQuery, params, &result)
	return result, err
}

// Shutdown asks the daemon to stop. Applications do not normally call this; it
// exists for tests and for replacing a daemon with a newer build.
func (client *Client) Shutdown() error {
	return client.call(proto.MethodShutdown, nil, nil)
}

// call retries once on a broken connection, which covers the daemon having exited
// on its idle timeout between two user actions.
func (client *Client) call(method string, params any, result any) error {
	client.mutex.Lock()
	defer client.mutex.Unlock()

	if err := client.ensureConnection(); err != nil {
		return err
	}
	err := client.exchange(method, params, result)
	if !isConnectionLost(err) {
		return err
	}
	client.discardConnection()
	if err := client.ensureConnection(); err != nil {
		return err
	}
	return client.exchange(method, params, result)
}

func (client *Client) ensureConnection() error {
	if client.connection != nil {
		return nil
	}
	return client.establishConnection()
}

func (client *Client) establishConnection() error {
	connection, err := dialOrSpawn(client.options)
	if err != nil {
		return err
	}
	client.connection = connection
	client.reader = bufio.NewReader(connection)
	if err := client.handshake(); err != nil {
		client.discardConnection()
		return err
	}
	return nil
}

func (client *Client) handshake() error {
	var result proto.HandshakeResult
	err := client.exchange(proto.MethodHandshake, proto.HandshakeParams{
		ClientName:      client.options.ClientName,
		ProtocolVersion: proto.ProtocolVersion,
	}, &result)
	if err != nil {
		return err
	}
	client.daemonVersion = result.DaemonVersion
	return nil
}

func (client *Client) exchange(method string, params any, result any) error {
	client.nextRequestId++
	payload, err := encodeRequest(client.nextRequestId, method, params)
	if err != nil {
		return err
	}
	if err := proto.WriteMessage(client.connection, payload); err != nil {
		return err
	}
	responsePayload, err := proto.ReadMessage(client.reader)
	if err != nil {
		return err
	}
	return decodeResponse(responsePayload, result)
}

func (client *Client) discardConnection() {
	if client.connection == nil {
		return
	}
	client.connection.Close()
	client.connection = nil
	client.reader = nil
}

func encodeRequest(requestId uint64, method string, params any) ([]byte, error) {
	request := proto.Request{Id: requestId, Method: method}
	if params == nil {
		return json.Marshal(request)
	}
	encodedParams, err := json.Marshal(params)
	if err != nil {
		return nil, err
	}
	request.Params = encodedParams
	return json.Marshal(request)
}

func decodeResponse(payload []byte, result any) error {
	var response proto.Response
	if err := json.Unmarshal(payload, &response); err != nil {
		return fmt.Errorf("decode response: %w", err)
	}
	if response.Error != nil {
		return response.Error
	}
	if result == nil || len(response.Result) == 0 {
		return nil
	}
	return json.Unmarshal(response.Result, result)
}

// isConnectionLost distinguishes a dead socket, which is worth retrying, from a
// daemon-reported error, which is not.
func isConnectionLost(err error) bool {
	if err == nil {
		return false
	}
	var protocolError *proto.Error
	if errors.As(err, &protocolError) {
		return false
	}
	if errors.Is(err, io.EOF) || errors.Is(err, io.ErrUnexpectedEOF) {
		return true
	}
	if errors.Is(err, os.ErrClosed) || errors.Is(err, net.ErrClosed) {
		return true
	}
	var networkError net.Error
	return errors.As(err, &networkError)
}

func withDefaults(options Options) Options {
	if options.ClientName == "" {
		options.ClientName = "unknown"
	}
	if options.SocketPath == "" {
		options.SocketPath = proto.SocketPath()
	}
	if options.SpawnTimeout <= 0 {
		options.SpawnTimeout = defaultSpawnTimeout
	}
	return options
}
