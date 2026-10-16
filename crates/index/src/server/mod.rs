//! Exposes the indexer over a unix socket. One accepted connection is one
//! application; the daemon exits on its own once the last one goes away.

mod handlers;
mod peer;

use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::index::{IndexError, Indexer};
use crate::proto::{self, ProtocolError};
use crate::stop::StopSignal;

pub use peer::peer_is_same_user;

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ServerOptions {
    pub socket_path: PathBuf,
    /// Zero disables the idle exit.
    pub idle_timeout: Duration,
    pub daemon_version: String,
    pub index_path: String,
}

struct ServerState {
    active_connections: usize,
    last_activity: Instant,
}

pub struct Server {
    indexer: Arc<Indexer>,
    options: ServerOptions,
    stop: StopSignal,
    state: Mutex<ServerState>,
}

/// Per-connection state. The handshake gate keeps a client from using methods
/// before both sides agree on the protocol version.
#[derive(Default)]
struct ConnectionSession {
    handshake_complete: bool,
    client_name: String,
    stop_after_response: bool,
}

/// Counts a connection as active for as long as it lives.
struct ActiveConnection<'server> {
    server: &'server Server,
}

impl<'server> ActiveConnection<'server> {
    fn open(server: &'server Server) -> Self {
        server.update_state(|state| {
            state.active_connections += 1;
            state.last_activity = Instant::now();
        });
        ActiveConnection { server }
    }
}

impl Drop for ActiveConnection<'_> {
    fn drop(&mut self) {
        self.server.update_state(|state| {
            state.active_connections -= 1;
            state.last_activity = Instant::now();
        });
    }
}

impl Server {
    /// `stop` ends serving when raised; a shutdown request, the idle timeout and
    /// a listener failure all raise it.
    pub fn new(indexer: Arc<Indexer>, options: ServerOptions, stop: StopSignal) -> Arc<Server> {
        let state = ServerState { active_connections: 0, last_activity: Instant::now() };
        Arc::new(Server { indexer, options, stop, state: Mutex::new(state) })
    }

    /// Listens until `stop` is raised.
    pub fn serve(self: &Arc<Self>) -> io::Result<()> {
        let listener = self.listen()?;
        let accept_wakeup = listener.try_clone()?;
        let accepting = Arc::clone(self);
        let accept_thread = thread::spawn(move || accepting.accept_connections(listener));
        let idle_watching = Arc::clone(self);
        let idle_thread = thread::spawn(move || idle_watching.watch_for_idleness());

        self.stop.wait();
        log::info!("daemon stopping");
        wake_accept_loop(&accept_wakeup);
        let _ = accept_thread.join();
        let _ = idle_thread.join();
        let _ = fs::remove_file(&self.options.socket_path);
        Ok(())
    }

    /// Replaces any socket left behind by a crashed daemon. This is only safe
    /// because the caller already holds the daemon lock, which proves no live
    /// daemon owns that path.
    fn listen(&self) -> io::Result<UnixListener> {
        let socket_path = &self.options.socket_path;
        proto::validate_socket_path(socket_path).map_err(io::Error::other)?;
        if let Some(directory) = socket_path.parent() {
            proto::ensure_private_directory(directory)?;
        }
        remove_stale_socket(socket_path)?;
        let listener = UnixListener::bind(socket_path)
            .map_err(|error| io::Error::new(error.kind(), format!("listen on {}: {error}", socket_path.display())))?;
        if let Err(error) = fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600)) {
            let _ = fs::remove_file(socket_path);
            return Err(error);
        }
        Ok(listener)
    }

    fn accept_connections(self: Arc<Self>, listener: UnixListener) {
        for connection in listener.incoming() {
            if self.stop.is_stopped() {
                return;
            }
            let Ok(connection) = connection else {
                self.stop.stop();
                return;
            };
            let handling = Arc::clone(&self);
            thread::spawn(move || handling.handle_connection(connection));
        }
    }

    fn handle_connection(&self, connection: UnixStream) {
        if !self.accept_peer(&connection) {
            return;
        }
        let _active = ActiveConnection::open(self);
        let Ok(mut writer) = connection.try_clone() else {
            return;
        };
        let mut reader = io::BufReader::new(connection);
        let mut session = ConnectionSession::default();
        while self.serve_request(&mut reader, &mut writer, &mut session) {}
    }

    fn accept_peer(&self, connection: &UnixStream) -> bool {
        match peer::peer_is_same_user(connection) {
            Ok(true) => true,
            Ok(false) => {
                log::warn!("refused connection from another user");
                false
            }
            Err(error) => {
                log::warn!("cannot verify peer, refusing connection error={error}");
                false
            }
        }
    }

    /// Handles one request and reports whether the connection stays open.
    fn serve_request(
        &self,
        reader: &mut impl io::Read,
        writer: &mut impl io::Write,
        session: &mut ConnectionSession,
    ) -> bool {
        let Ok(payload) = proto::read_message(reader) else {
            return false;
        };
        self.update_state(|state| state.last_activity = Instant::now());
        let response = self.build_response(&payload, session);
        let written = proto::write_message(writer, &response).is_ok();
        if session.stop_after_response {
            self.stop.stop();
        }
        written && !self.stop.is_stopped()
    }

    fn build_response(&self, payload: &[u8], session: &mut ConnectionSession) -> Vec<u8> {
        let request: proto::Request = match serde_json::from_slice(payload) {
            Ok(request) => request,
            Err(error) => {
                let failure = ProtocolError::new(proto::CODE_INVALID_PARAMS, error.to_string());
                return encode_response(&proto::Response { error: Some(failure), ..proto::Response::default() });
            }
        };
        let id = request.id;
        match self.dispatch(request, session) {
            Ok(result) => encode_response(&proto::Response { id, result: Some(result), error: None }),
            Err(failure) => encode_response(&proto::Response { id, result: None, error: Some(failure) }),
        }
    }

    fn update_state(&self, update: impl FnOnce(&mut ServerState)) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        update(&mut state);
    }

    fn watch_for_idleness(&self) {
        if self.options.idle_timeout.is_zero() {
            return;
        }
        while !self.stop.wait_timeout(IDLE_CHECK_INTERVAL) {
            self.stop_if_idle();
        }
    }

    /// Exits only when nothing is connected and no scan is in flight, so a
    /// first-run crawl is never abandoned halfway.
    fn stop_if_idle(&self) {
        if !self.is_idle() || self.is_scanning() {
            return;
        }
        log::info!("idle timeout reached timeout={:?}", self.options.idle_timeout);
        self.stop.stop();
    }

    fn is_idle(&self) -> bool {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.active_connections == 0 && state.last_activity.elapsed() > self.options.idle_timeout
    }

    fn is_scanning(&self) -> bool {
        self.indexer.status().map(|status| status.scanning).unwrap_or(false)
    }
}

impl From<IndexError> for ProtocolError {
    fn from(error: IndexError) -> Self {
        if is_not_found(&error) {
            return ProtocolError::new(proto::CODE_NOT_FOUND, error.to_string());
        }
        ProtocolError::new(proto::CODE_INTERNAL, error.to_string())
    }
}

fn is_not_found(error: &IndexError) -> bool {
    match error {
        IndexError::RootNotFound => true,
        IndexError::Io(io_error) => io_error.kind() == io::ErrorKind::NotFound,
        _ => false,
    }
}

/// Encoding cannot fail in practice: every field is already valid JSON.
fn encode_response(response: &proto::Response) -> Vec<u8> {
    serde_json::to_vec(response).unwrap_or_else(|_| {
        br#"{"id":0,"error":{"code":"internal","message":"response encoding failed"}}"#.to_vec()
    })
}

/// Shutting down a listening socket makes a blocked `accept` fail, which ends
/// the accept loop without needing a connection to the (possibly removed) path.
fn wake_accept_loop(listener: &UnixListener) {
    unsafe { libc::shutdown(listener.as_raw_fd(), libc::SHUT_RDWR) };
}

fn remove_stale_socket(socket_path: &std::path::Path) -> io::Result<()> {
    match fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io::Error::new(error.kind(), format!("remove stale socket: {error}"))),
    }
}
