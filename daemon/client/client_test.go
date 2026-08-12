package client_test

import (
	"context"
	"encoding/json"
	"log/slog"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"canopy/daemon/client"
	"canopy/daemon/internal/index"
	"canopy/daemon/internal/server"
	"canopy/daemon/proto"
)

const eventualTimeout = 20 * time.Second

func TestClientSearchesThroughARunningDaemon(t *testing.T) {
	treePath := buildTree(t, []string{"quarterly_report.pdf", "shopping.txt"})
	socketPath := startInProcessDaemon(t)

	daemonClient := connectTestClient(t, socketPath)
	if err := daemonClient.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	expectedPath := filepath.Join(treePath, "quarterly_report.pdf")
	waitUntil(t, "search returns the indexed file", func() bool {
		return containsPath(searchOrFail(t, daemonClient, "report"), expectedPath)
	})
}

func TestClientReadsRootsAndStatus(t *testing.T) {
	treePath := buildTree(t, []string{"one.txt", "two.txt"})
	socketPath := startInProcessDaemon(t)

	daemonClient := connectTestClient(t, socketPath)
	if err := daemonClient.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	waitUntil(t, "status reports indexed files", func() bool {
		status, err := daemonClient.Status()
		if err != nil {
			t.Fatalf("status: %v", err)
		}
		return status.FilesIndexed >= 2
	})
	roots, err := daemonClient.Roots()
	if err != nil {
		t.Fatalf("roots: %v", err)
	}
	if len(roots) != 1 || roots[0].Path != treePath {
		t.Fatalf("unexpected roots: %+v", roots)
	}
}

func TestHandshakeRejectsAMismatchedProtocolVersion(t *testing.T) {
	socketPath := startInProcessDaemon(t)

	response := sendRawRequest(t, socketPath, proto.Request{
		Id:     1,
		Method: proto.MethodHandshake,
		Params: encodeJson(t, proto.HandshakeParams{ClientName: "test", ProtocolVersion: proto.ProtocolVersion + 1}),
	})
	if response.Error == nil || response.Error.Code != proto.CodeProtocolMismatch {
		t.Fatalf("expected a protocol mismatch error, got %+v", response)
	}
}

func TestMethodsAreRefusedBeforeTheHandshake(t *testing.T) {
	socketPath := startInProcessDaemon(t)

	response := sendRawRequest(t, socketPath, proto.Request{Id: 1, Method: proto.MethodStatus})
	if response.Error == nil || response.Error.Code != proto.CodeProtocolMismatch {
		t.Fatalf("expected the handshake gate to reject the call, got %+v", response)
	}
}

func TestUnknownMethodIsReported(t *testing.T) {
	socketPath := startInProcessDaemon(t)

	response := sendRawRequest(t, socketPath, proto.Request{Id: 1, Method: "does.notExist"})
	if response.Error == nil || response.Error.Code != proto.CodeUnknownMethod {
		t.Fatalf("expected an unknown method error, got %+v", response)
	}
}

// The election path is the part that cannot be exercised in process: the client
// must find no daemon, start the bundled binary, and connect to it.
func TestConnectSpawnsTheDaemonBinary(t *testing.T) {
	binaryPath := buildDaemonBinary(t)
	socketPath := filepath.Join(t.TempDir(), "daemon.sock")
	t.Setenv(proto.RuntimeDirectoryEnvironmentVariable, t.TempDir())
	t.Setenv(proto.DataDirectoryEnvironmentVariable, t.TempDir())

	treePath := buildTree(t, []string{"spawned_result.txt"})
	daemonClient, err := client.Connect(client.Options{
		ClientName:   "spawn-test",
		SocketPath:   socketPath,
		BinaryPath:   binaryPath,
		SpawnTimeout: eventualTimeout,
	})
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	t.Cleanup(func() {
		daemonClient.Shutdown()
		daemonClient.Close()
	})

	if daemonClient.DaemonVersion() == "" {
		t.Fatal("handshake did not report a daemon version")
	}
	if err := daemonClient.AddRoot(treePath); err != nil {
		t.Fatalf("add root: %v", err)
	}
	expectedPath := filepath.Join(treePath, "spawned_result.txt")
	waitUntil(t, "the spawned daemon indexes the tree", func() bool {
		return containsPath(searchOrFail(t, daemonClient, "spawned"), expectedPath)
	})
}

// A second daemon must refuse to run rather than compete for the socket.
func TestSecondDaemonExitsWhileTheFirstHoldsTheLock(t *testing.T) {
	binaryPath := buildDaemonBinary(t)
	runtimeDirectory := t.TempDir()
	t.Setenv(proto.RuntimeDirectoryEnvironmentVariable, runtimeDirectory)
	t.Setenv(proto.DataDirectoryEnvironmentVariable, t.TempDir())

	socketPath := filepath.Join(t.TempDir(), "daemon.sock")
	daemonClient, err := client.Connect(client.Options{
		ClientName:   "lock-test",
		SocketPath:   socketPath,
		BinaryPath:   binaryPath,
		SpawnTimeout: eventualTimeout,
	})
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	t.Cleanup(func() {
		daemonClient.Shutdown()
		daemonClient.Close()
	})

	secondSocketPath := filepath.Join(t.TempDir(), "second.sock")
	output, err := exec.Command(binaryPath, "-socket", secondSocketPath).CombinedOutput()
	if err != nil {
		t.Fatalf("second daemon exited with an error: %v (%s)", err, output)
	}
	if _, statError := os.Stat(secondSocketPath); statError == nil {
		t.Fatal("the second daemon should not have created a socket")
	}
}

func startInProcessDaemon(t *testing.T) string {
	t.Helper()
	socketPath := filepath.Join(t.TempDir(), "daemon.sock")
	runContext, stopRunning := context.WithCancel(context.Background())
	indexer := newTestIndexer(t, runContext)

	daemonServer := server.New(indexer, server.Options{
		SocketPath:    socketPath,
		DaemonVersion: "test",
		IndexPath:     "memory",
		Logger:        slog.New(slog.DiscardHandler),
	})
	serveResult := make(chan error, 1)
	go func() {
		serveResult <- daemonServer.Serve(runContext)
	}()
	t.Cleanup(func() {
		stopRunning()
		<-serveResult
		indexer.Close()
	})
	waitForSocket(t, socketPath)
	return socketPath
}

func newTestIndexer(t *testing.T, runContext context.Context) *index.Indexer {
	t.Helper()
	indexer, err := index.New(runContext, index.Options{
		DatabasePath:   filepath.Join(t.TempDir(), "index.db"),
		RescanInterval: time.Hour,
		QuietPeriod:    50 * time.Millisecond,
		Logger:         slog.New(slog.DiscardHandler),
	})
	if err != nil {
		t.Fatalf("new indexer: %v", err)
	}
	indexer.Start()
	return indexer
}

func connectTestClient(t *testing.T, socketPath string) *client.Client {
	t.Helper()
	daemonClient, err := client.Connect(client.Options{
		ClientName:   "test",
		SocketPath:   socketPath,
		SpawnTimeout: 2 * time.Second,
	})
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	t.Cleanup(func() {
		daemonClient.Close()
	})
	return daemonClient
}

func buildDaemonBinary(t *testing.T) string {
	t.Helper()
	binaryPath := filepath.Join(t.TempDir(), "canopyd")
	command := exec.Command("go", "build", "-o", binaryPath, "./cmd/canopyd")
	command.Dir = ".."
	output, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("build daemon: %v (%s)", err, output)
	}
	return binaryPath
}

func sendRawRequest(t *testing.T, socketPath string, request proto.Request) proto.Response {
	t.Helper()
	connection, err := net.Dial("unix", socketPath)
	if err != nil {
		t.Fatalf("dial: %v", err)
	}
	defer connection.Close()

	if err := proto.WriteMessage(connection, encodeJson(t, request)); err != nil {
		t.Fatalf("write request: %v", err)
	}
	payload, err := proto.ReadMessage(connection)
	if err != nil {
		t.Fatalf("read response: %v", err)
	}
	var response proto.Response
	if err := json.Unmarshal(payload, &response); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	return response
}

func encodeJson(t *testing.T, value any) []byte {
	t.Helper()
	encoded, err := json.Marshal(value)
	if err != nil {
		t.Fatalf("encode: %v", err)
	}
	return encoded
}

func buildTree(t *testing.T, relativePaths []string) string {
	t.Helper()
	treePath := t.TempDir()
	for _, relativePath := range relativePaths {
		writeTreeFile(t, treePath, relativePath)
	}
	return treePath
}

func writeTreeFile(t *testing.T, treePath string, relativePath string) {
	t.Helper()
	fullPath := filepath.Join(treePath, relativePath)
	if err := os.MkdirAll(filepath.Dir(fullPath), 0o700); err != nil {
		t.Fatalf("create directory for %s: %v", relativePath, err)
	}
	if err := os.WriteFile(fullPath, []byte(relativePath), 0o600); err != nil {
		t.Fatalf("write %s: %v", relativePath, err)
	}
}

func searchOrFail(t *testing.T, daemonClient *client.Client, query string) []proto.Hit {
	t.Helper()
	result, err := daemonClient.Search(proto.SearchParams{Text: query, Limit: 50})
	if err != nil {
		t.Fatalf("search %q: %v", query, err)
	}
	return result.Hits
}

func containsPath(hits []proto.Hit, expectedPath string) bool {
	for _, hit := range hits {
		if hit.Path == expectedPath {
			return true
		}
	}
	return false
}

func waitForSocket(t *testing.T, socketPath string) {
	t.Helper()
	waitUntil(t, "the daemon accepts connections", func() bool {
		connection, err := net.Dial("unix", socketPath)
		if err != nil {
			return false
		}
		connection.Close()
		return true
	})
}

func waitUntil(t *testing.T, description string, satisfied func() bool) {
	t.Helper()
	deadline := time.Now().Add(eventualTimeout)
	for time.Now().Before(deadline) {
		if satisfied() {
			return
		}
		time.Sleep(25 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %s", description)
}
