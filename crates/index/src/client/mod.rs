//! Connects an application to the shared Neoworks daemon. Callers never manage
//! the daemon's lifetime: `Client::connect` starts one if none is running, and
//! the daemon exits by itself once the last application disconnects.

mod spawn;

use std::io::{self, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::proto::{self, ProtocolError};

pub use spawn::BINARY_PATH_ENVIRONMENT_VARIABLE;

const DEFAULT_SPAWN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The daemon answered with an error.
    #[error(transparent)]
    Daemon(#[from] ProtocolError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("decode response: {0}")]
    Decode(#[from] serde_json::Error),
    /// The daemon could not be found, started, or reached.
    #[error("{0}")]
    Startup(String),
}

pub type ClientResult<Value> = Result<Value, ClientError>;

/// Empty fields select the defaults, which follow the XDG directories the
/// daemon also uses.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Identifies the application in daemon logs.
    pub client_name: String,
    pub socket_path: Option<PathBuf>,
    pub binary_path: Option<PathBuf>,
    /// Zero selects ten seconds.
    pub spawn_timeout: Duration,
}

impl Options {
    fn with_defaults(mut self) -> Self {
        if self.client_name.is_empty() {
            self.client_name = "unknown".to_string();
        }
        if self.socket_path.is_none() {
            self.socket_path = Some(proto::socket_path());
        }
        if self.spawn_timeout.is_zero() {
            self.spawn_timeout = DEFAULT_SPAWN_TIMEOUT;
        }
        self
    }

    fn socket_path(&self) -> &std::path::Path {
        self.socket_path.as_deref().unwrap_or(std::path::Path::new(""))
    }
}

struct Connection {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

#[derive(Default)]
struct ClientState {
    connection: Option<Connection>,
    next_request_id: u64,
    daemon_version: String,
}

/// A connection to the daemon. Calls are serialized, so one `Client` is safe to
/// share across an application's threads.
pub struct Client {
    options: Options,
    state: Mutex<ClientState>,
}

impl Client {
    /// Returns a connected client, spawning the daemon when necessary.
    pub fn connect(options: Options) -> ClientResult<Client> {
        let client = Client { options: options.with_defaults(), state: Mutex::new(ClientState::default()) };
        client.establish_connection(&mut client.lock_state())?;
        Ok(client)
    }

    /// The version reported by the handshake.
    pub fn daemon_version(&self) -> String {
        self.lock_state().daemon_version.clone()
    }

    pub fn close(&self) {
        self.lock_state().connection = None;
    }

    /// Registers a directory tree for indexing. It returns as soon as the
    /// daemon accepts the root; the crawl continues in the background.
    pub fn add_root(&self, path: &str) -> ClientResult<()> {
        self.call::<_, Value>(proto::METHOD_ADD_ROOT, Some(&proto::AddRootParams { path: path.to_string() }))?;
        Ok(())
    }

    pub fn remove_root(&self, path: &str) -> ClientResult<()> {
        self.call::<_, Value>(proto::METHOD_REMOVE_ROOT, Some(&proto::RemoveRootParams { path: path.to_string() }))?;
        Ok(())
    }

    pub fn roots(&self) -> ClientResult<Vec<proto::Root>> {
        let result: proto::RootsResult = self.call(proto::METHOD_ROOTS, None::<&()>)?;
        Ok(result.roots)
    }

    pub fn status(&self) -> ClientResult<proto::StatusResult> {
        self.call(proto::METHOD_STATUS, None::<&()>)
    }

    /// Reindexes one root, or every root when `path` is empty.
    pub fn rescan(&self, path: &str) -> ClientResult<()> {
        self.call::<_, Value>(proto::METHOD_RESCAN, Some(&proto::RescanParams { path: path.to_string() }))?;
        Ok(())
    }

    pub fn search(&self, params: &proto::SearchParams) -> ClientResult<proto::SearchResult> {
        self.call(proto::METHOD_SEARCH_QUERY, Some(params))
    }

    /// Asks the daemon to stop. Applications do not normally call this; it
    /// exists for tests and for replacing a daemon with a newer build.
    pub fn shutdown(&self) -> ClientResult<()> {
        self.call::<_, Value>(proto::METHOD_SHUTDOWN, None::<&()>)?;
        Ok(())
    }

    fn lock_state(&self) -> MutexGuard<'_, ClientState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Retries once on a broken connection, which covers the daemon having
    /// exited on its idle timeout between two user actions.
    fn call<Params: Serialize, Reply: DeserializeOwned + Default>(
        &self,
        method: &str,
        params: Option<&Params>,
    ) -> ClientResult<Reply> {
        let mut state = self.lock_state();
        self.ensure_connection(&mut state)?;
        let outcome = exchange(&mut state, method, params);
        if !is_connection_lost(&outcome) {
            return outcome;
        }
        state.connection = None;
        self.ensure_connection(&mut state)?;
        exchange(&mut state, method, params)
    }

    fn ensure_connection(&self, state: &mut ClientState) -> ClientResult<()> {
        if state.connection.is_some() {
            return Ok(());
        }
        self.establish_connection(state)
    }

    fn establish_connection(&self, state: &mut ClientState) -> ClientResult<()> {
        let stream = spawn::dial_or_spawn(&self.options)?;
        let reader = BufReader::new(stream.try_clone()?);
        state.connection = Some(Connection { writer: stream, reader });
        if let Err(error) = self.handshake(state) {
            state.connection = None;
            return Err(error);
        }
        Ok(())
    }

    fn handshake(&self, state: &mut ClientState) -> ClientResult<()> {
        let params = proto::HandshakeParams {
            client_name: self.options.client_name.clone(),
            protocol_version: proto::PROTOCOL_VERSION,
        };
        let result: proto::HandshakeResult = exchange(state, proto::METHOD_HANDSHAKE, Some(&params))?;
        state.daemon_version = result.daemon_version;
        Ok(())
    }
}

fn exchange<Params: Serialize, Reply: DeserializeOwned + Default>(
    state: &mut ClientState,
    method: &str,
    params: Option<&Params>,
) -> ClientResult<Reply> {
    state.next_request_id += 1;
    let payload = encode_request(state.next_request_id, method, params)?;
    let Some(connection) = state.connection.as_mut() else {
        return Err(io::Error::from(io::ErrorKind::NotConnected).into());
    };
    proto::write_message(&mut connection.writer, &payload)?;
    let response_payload = proto::read_message(&mut connection.reader)?;
    decode_response(&response_payload)
}

fn encode_request<Params: Serialize>(request_id: u64, method: &str, params: Option<&Params>) -> ClientResult<Vec<u8>> {
    let params = match params {
        Some(params) => Some(serde_json::to_value(params)?),
        None => None,
    };
    let request = proto::Request { id: request_id, method: method.to_string(), params };
    Ok(serde_json::to_vec(&request)?)
}

fn decode_response<Reply: DeserializeOwned + Default>(payload: &[u8]) -> ClientResult<Reply> {
    let response: proto::Response = serde_json::from_slice(payload)?;
    if let Some(failure) = response.error {
        return Err(failure.into());
    }
    match response.result {
        Some(result) => Ok(serde_json::from_value(result)?),
        None => Ok(Reply::default()),
    }
}

/// Distinguishes a dead socket, which is worth retrying, from a daemon-reported
/// error, which is not.
fn is_connection_lost<Reply>(outcome: &ClientResult<Reply>) -> bool {
    match outcome {
        Err(ClientError::Io(error)) => !proto::is_message_too_large(error),
        _ => false,
    }
}
