use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use canopy_index::client::{Client, Options};
use canopy_index::index::{Indexer, IndexerOptions};
use canopy_index::proto::{self, Hit, ProtocolError, Request, Response, SearchParams};
use canopy_index::server::{Server, ServerOptions};
use canopy_index::stop::StopSignal;
use serde_json::json;

const EVENTUAL_TIMEOUT: Duration = Duration::from_secs(20);

static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

/// The runtime and data directory overrides are process-wide, so the tests that
/// set them take turns.
struct IsolatedEnvironment {
    _lock: MutexGuard<'static, ()>,
    _runtime_directory: tempfile::TempDir,
    _data_directory: tempfile::TempDir,
}

impl IsolatedEnvironment {
    fn new() -> Self {
        let lock = ENVIRONMENT_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let runtime_directory = tempfile::tempdir().unwrap();
        let data_directory = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var(proto::RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE, runtime_directory.path());
            std::env::set_var(proto::DATA_DIRECTORY_ENVIRONMENT_VARIABLE, data_directory.path());
        }
        IsolatedEnvironment { _lock: lock, _runtime_directory: runtime_directory, _data_directory: data_directory }
    }
}

impl Drop for IsolatedEnvironment {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var(proto::RUNTIME_DIRECTORY_ENVIRONMENT_VARIABLE);
            std::env::remove_var(proto::DATA_DIRECTORY_ENVIRONMENT_VARIABLE);
        }
    }
}

struct InProcessDaemon {
    socket_path: PathBuf,
    stop: StopSignal,
    serving: Option<JoinHandle<()>>,
    _directory: tempfile::TempDir,
}

impl Drop for InProcessDaemon {
    fn drop(&mut self) {
        self.stop.stop();
        if let Some(serving) = self.serving.take() {
            let _ = serving.join();
        }
    }
}

fn start_in_process_daemon() -> InProcessDaemon {
    let directory = tempfile::tempdir().unwrap();
    let socket_path = directory.path().join("daemon.sock");
    let stop = StopSignal::new();
    let indexer = Indexer::new(
        stop.clone(),
        IndexerOptions {
            database_path: directory.path().join("index.db"),
            rescan_interval: Duration::from_secs(3600),
            quiet_period: Duration::from_millis(50),
            ..IndexerOptions::default()
        },
    )
    .unwrap();
    indexer.start();
    let options = ServerOptions {
        socket_path: socket_path.clone(),
        idle_timeout: Duration::ZERO,
        daemon_version: "test".into(),
        index_path: "memory".into(),
    };
    let server = Server::new(Arc::new(indexer), options, stop.clone());
    let serving = thread::spawn(move || server.serve().unwrap());
    wait_until("the daemon accepts connections", || UnixStream::connect(&socket_path).is_ok());
    InProcessDaemon { socket_path, stop, serving: Some(serving), _directory: directory }
}

fn connect_test_client(socket_path: &Path) -> Client {
    Client::connect(Options {
        client_name: "test".into(),
        socket_path: Some(socket_path.to_path_buf()),
        spawn_timeout: Duration::from_secs(2),
        ..Options::default()
    })
    .unwrap()
}

fn build_tree(relative_paths: &[&str]) -> tempfile::TempDir {
    let tree = tempfile::tempdir().unwrap();
    for relative_path in relative_paths {
        let full_path = tree.path().join(relative_path);
        fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        fs::write(&full_path, relative_path).unwrap();
    }
    tree
}

fn search_or_fail(client: &Client, text: &str) -> Vec<Hit> {
    client.search(&SearchParams { text: text.into(), limit: 50, ..SearchParams::default() }).unwrap().hits
}

fn contains_path(hits: &[Hit], expected_path: &Path) -> bool {
    hits.iter().any(|hit| Path::new(&hit.path) == expected_path)
}

fn wait_until(description: &str, satisfied: impl Fn() -> bool) {
    let deadline = Instant::now() + EVENTUAL_TIMEOUT;
    while Instant::now() < deadline {
        if satisfied() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {description}");
}

fn send_raw_request(socket_path: &Path, request: &Request) -> Response {
    let mut connection = UnixStream::connect(socket_path).unwrap();
    proto::write_message(&mut connection, &serde_json::to_vec(request).unwrap()).unwrap();
    let payload = proto::read_message(&mut connection).unwrap();
    serde_json::from_slice(&payload).unwrap()
}

fn assert_error_code(response: &Response, expected_code: &str) {
    let error: &ProtocolError = response.error.as_ref().unwrap_or_else(|| panic!("expected an error, got {response:?}"));
    assert_eq!(error.code, expected_code);
}

#[test]
fn client_searches_through_a_running_daemon() {
    let tree = build_tree(&["quarterly_report.pdf", "shopping.txt"]);
    let daemon = start_in_process_daemon();
    let client = connect_test_client(&daemon.socket_path);
    client.add_root(&tree.path().to_string_lossy()).unwrap();

    let expected_path = tree.path().join("quarterly_report.pdf");
    wait_until("search returns the indexed file", || contains_path(&search_or_fail(&client, "report"), &expected_path));
}

#[test]
fn client_reads_roots_and_status() {
    let tree = build_tree(&["one.txt", "two.txt"]);
    let daemon = start_in_process_daemon();
    let client = connect_test_client(&daemon.socket_path);
    client.add_root(&tree.path().to_string_lossy()).unwrap();

    wait_until("status reports indexed files", || client.status().unwrap().files_indexed >= 2);
    let roots = client.roots().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(Path::new(&roots[0].path), tree.path());
}

#[test]
fn handshake_rejects_a_mismatched_protocol_version() {
    let daemon = start_in_process_daemon();
    let params = json!({"clientName": "test", "protocolVersion": proto::PROTOCOL_VERSION + 1});
    let request = Request { id: 1, method: proto::METHOD_HANDSHAKE.into(), params: Some(params) };
    assert_error_code(&send_raw_request(&daemon.socket_path, &request), proto::CODE_PROTOCOL_MISMATCH);
}

#[test]
fn methods_are_refused_before_the_handshake() {
    let daemon = start_in_process_daemon();
    let request = Request { id: 1, method: proto::METHOD_STATUS.into(), params: None };
    assert_error_code(&send_raw_request(&daemon.socket_path, &request), proto::CODE_PROTOCOL_MISMATCH);
}

#[test]
fn unknown_method_is_reported() {
    let daemon = start_in_process_daemon();
    let request = Request { id: 1, method: "does.notExist".into(), params: None };
    assert_error_code(&send_raw_request(&daemon.socket_path, &request), proto::CODE_UNKNOWN_METHOD);
}

#[test]
fn removing_an_unknown_root_reports_not_found() {
    let daemon = start_in_process_daemon();
    let client = connect_test_client(&daemon.socket_path);
    let error = client.remove_root("/nowhere/at/all").unwrap_err();
    assert!(error.to_string().starts_with(proto::CODE_NOT_FOUND), "{error}");
}

#[test]
fn shutdown_stops_the_daemon_and_removes_its_socket() {
    let mut daemon = start_in_process_daemon();
    let client = connect_test_client(&daemon.socket_path);
    client.shutdown().unwrap();
    daemon.serving.take().unwrap().join().unwrap();
    assert!(!daemon.socket_path.exists());
}

struct SpawnedDaemon {
    client: Client,
}

impl Drop for SpawnedDaemon {
    fn drop(&mut self) {
        let _ = self.client.shutdown();
        self.client.close();
    }
}

fn connect_spawning_daemon(socket_path: &Path, client_name: &str) -> SpawnedDaemon {
    let client = Client::connect(Options {
        client_name: client_name.into(),
        socket_path: Some(socket_path.to_path_buf()),
        binary_path: Some(PathBuf::from(env!("CARGO_BIN_EXE_canopyd"))),
        spawn_timeout: EVENTUAL_TIMEOUT,
    })
    .unwrap();
    SpawnedDaemon { client }
}

// The election path cannot be exercised in process: the client must find no
// daemon, start the bundled binary, and connect to it.
#[test]
fn connect_spawns_the_daemon_binary() {
    let _environment = IsolatedEnvironment::new();
    let socket_directory = tempfile::tempdir().unwrap();
    let tree = build_tree(&["spawned_result.txt"]);

    let spawned = connect_spawning_daemon(&socket_directory.path().join("daemon.sock"), "spawn-test");
    assert!(!spawned.client.daemon_version().is_empty());
    spawned.client.add_root(&tree.path().to_string_lossy()).unwrap();
    let expected_path = tree.path().join("spawned_result.txt");
    wait_until("the spawned daemon indexes the tree", || {
        contains_path(&search_or_fail(&spawned.client, "spawned"), &expected_path)
    });
}

// A second daemon must refuse to run rather than compete for the socket.
#[test]
fn second_daemon_exits_while_the_first_holds_the_lock() {
    let _environment = IsolatedEnvironment::new();
    let socket_directory = tempfile::tempdir().unwrap();
    let _first = connect_spawning_daemon(&socket_directory.path().join("daemon.sock"), "lock-test");

    let second_socket_path = socket_directory.path().join("second.sock");
    let output = Command::new(env!("CARGO_BIN_EXE_canopyd")).arg("-socket").arg(&second_socket_path).output().unwrap();
    assert!(output.status.success(), "second daemon failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(!second_socket_path.exists(), "the second daemon should not have created a socket");
}

#[test]
fn version_flag_prints_the_daemon_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_canopyd")).arg("-version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), canopy_index::daemon::DAEMON_VERSION);
}
