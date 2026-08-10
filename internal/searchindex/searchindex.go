// Package searchindex is the explorer's side of the shared Neoworks daemon. It
// hides the daemon's lifecycle from the rest of the application: the first search
// connects, starting the daemon if no other Neoworks application has already.
package searchindex

import (
	"os"
	"sync"

	"github.com/neoworks-dev/neoworksd/client"
	"github.com/neoworks-dev/neoworksd/proto"
)

const clientName = "neoworks-files"

// defaultSearchLimit is what fits in the results view without paging.
const defaultSearchLimit = 50

// Connection lazily holds one daemon client. Connecting on first use rather than
// at startup keeps a daemon that fails to start from blocking the window.
type Connection struct {
	mutex         sync.Mutex
	daemonClient  *client.Client
	homeRootAdded bool
}

func New() *Connection {
	return &Connection{}
}

// Search matches file and directory names anywhere in the indexed roots.
func (connection *Connection) Search(text string) (proto.SearchResult, error) {
	daemonClient, err := connection.ensureClient()
	if err != nil {
		return proto.SearchResult{}, err
	}
	return daemonClient.Search(text, defaultSearchLimit, false)
}

func (connection *Connection) Status() (proto.StatusResult, error) {
	daemonClient, err := connection.ensureClient()
	if err != nil {
		return proto.StatusResult{}, err
	}
	return daemonClient.Status()
}

// Close drops the connection. The daemon keeps running for other applications and
// exits on its own idle timeout.
func (connection *Connection) Close() error {
	connection.mutex.Lock()
	defer connection.mutex.Unlock()

	if connection.daemonClient == nil {
		return nil
	}
	err := connection.daemonClient.Close()
	connection.daemonClient = nil
	return err
}

func (connection *Connection) ensureClient() (*client.Client, error) {
	connection.mutex.Lock()
	defer connection.mutex.Unlock()

	if connection.daemonClient != nil {
		return connection.daemonClient, nil
	}
	daemonClient, err := client.Connect(client.Options{ClientName: clientName})
	if err != nil {
		return nil, err
	}
	connection.daemonClient = daemonClient
	connection.registerHomeDirectory(daemonClient)
	return daemonClient, nil
}

// registerHomeDirectory makes the explorer useful on first run without asking the
// user to configure anything. A failure is not fatal: search still works over
// whatever roots another Neoworks application already registered.
func (connection *Connection) registerHomeDirectory(daemonClient *client.Client) {
	if connection.homeRootAdded {
		return
	}
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return
	}
	if err := daemonClient.AddRoot(homeDirectory); err != nil {
		return
	}
	connection.homeRootAdded = true
}
