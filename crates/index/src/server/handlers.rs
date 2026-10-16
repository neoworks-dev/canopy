use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{ConnectionSession, Server};
use crate::index::{Query, RootRecord, Status};
use crate::proto::{self, ProtocolError};

const KNOWN_METHODS: &[&str] = &[
    proto::METHOD_HANDSHAKE,
    proto::METHOD_ADD_ROOT,
    proto::METHOD_REMOVE_ROOT,
    proto::METHOD_ROOTS,
    proto::METHOD_STATUS,
    proto::METHOD_RESCAN,
    proto::METHOD_SEARCH_QUERY,
    proto::METHOD_SHUTDOWN,
];

type HandlerResult = Result<Value, ProtocolError>;

impl Server {
    pub(super) fn dispatch(&self, request: proto::Request, session: &mut ConnectionSession) -> HandlerResult {
        if !KNOWN_METHODS.contains(&request.method.as_str()) {
            return Err(ProtocolError::new(proto::CODE_UNKNOWN_METHOD, request.method));
        }
        require_handshake(&request.method, session)?;
        let params = request.params;
        match request.method.as_str() {
            proto::METHOD_HANDSHAKE => self.handle_handshake(params, session),
            proto::METHOD_ADD_ROOT => self.handle_add_root(params),
            proto::METHOD_REMOVE_ROOT => self.handle_remove_root(params),
            proto::METHOD_ROOTS => self.handle_roots(),
            proto::METHOD_STATUS => self.handle_status(),
            proto::METHOD_RESCAN => self.handle_rescan(params),
            proto::METHOD_SEARCH_QUERY => self.handle_search(params),
            proto::METHOD_SHUTDOWN => Ok(self.handle_shutdown(session)),
            unknown => Err(ProtocolError::new(proto::CODE_UNKNOWN_METHOD, unknown)),
        }
    }

    fn handle_handshake(&self, params: Option<Value>, session: &mut ConnectionSession) -> HandlerResult {
        let params: proto::HandshakeParams = decode_params(params)?;
        if params.protocol_version != proto::PROTOCOL_VERSION {
            let message = describe_version_mismatch(params.protocol_version);
            return Err(ProtocolError::new(proto::CODE_PROTOCOL_MISMATCH, message));
        }
        session.handshake_complete = true;
        log::info!("client connected client={}", params.client_name);
        session.client_name = params.client_name;
        encode_result(&proto::HandshakeResult {
            daemon_version: self.options.daemon_version.clone(),
            protocol_version: proto::PROTOCOL_VERSION,
            index_path: self.options.index_path.clone(),
        })
    }

    fn handle_add_root(&self, params: Option<Value>) -> HandlerResult {
        let params: proto::AddRootParams = decode_params(params)?;
        self.indexer.add_root(&params.path)?;
        encode_result(&proto::EmptyResult {})
    }

    fn handle_remove_root(&self, params: Option<Value>) -> HandlerResult {
        let params: proto::RemoveRootParams = decode_params(params)?;
        self.indexer.remove_root(&params.path)?;
        encode_result(&proto::EmptyResult {})
    }

    fn handle_roots(&self) -> HandlerResult {
        let roots = self.indexer.roots()?;
        encode_result(&proto::RootsResult { roots: roots.iter().map(to_protocol_root).collect() })
    }

    fn handle_status(&self) -> HandlerResult {
        encode_result(&to_protocol_status(&self.indexer.status()?))
    }

    fn handle_rescan(&self, params: Option<Value>) -> HandlerResult {
        let params: proto::RescanParams = decode_params(params)?;
        self.indexer.rescan(&params.path)?;
        encode_result(&proto::EmptyResult {})
    }

    fn handle_search(&self, params: Option<Value>) -> HandlerResult {
        let params: proto::SearchParams = decode_params(params)?;
        let outcome = self.indexer.search(&Query {
            text: params.text,
            limit: params.limit,
            directories_only: params.directories_only,
            scope: params.scope,
        })?;
        encode_result(&proto::SearchResult {
            hits: outcome.records.into_iter().map(to_protocol_hit).collect(),
            truncated: outcome.truncated,
        })
    }

    fn handle_shutdown(&self, session: &mut ConnectionSession) -> Value {
        log::info!("shutdown requested client={}", session.client_name);
        session.stop_after_response = true;
        serde_json::json!({})
    }
}

/// Absent params decode as the zero value, so methods without arguments need no
/// params field on the wire.
fn decode_params<Params: DeserializeOwned + Default>(raw: Option<Value>) -> Result<Params, ProtocolError> {
    let Some(raw) = raw else {
        return Ok(Params::default());
    };
    serde_json::from_value(raw).map_err(|error| ProtocolError::new(proto::CODE_INVALID_PARAMS, error.to_string()))
}

fn encode_result(result: &impl Serialize) -> HandlerResult {
    serde_json::to_value(result).map_err(|error| ProtocolError::new(proto::CODE_INTERNAL, error.to_string()))
}

/// Gates every method but the handshake itself.
fn require_handshake(method: &str, session: &ConnectionSession) -> Result<(), ProtocolError> {
    if method == proto::METHOD_HANDSHAKE || session.handshake_complete {
        return Ok(());
    }
    Err(ProtocolError::new(proto::CODE_PROTOCOL_MISMATCH, format!("handshake required before {method}")))
}

fn to_protocol_root(root: &RootRecord) -> proto::Root {
    proto::Root { path: root.path.clone(), file_count: root.file_count, last_scan_unix: root.last_scan_unix }
}

fn to_protocol_hit(record: crate::index::FileRecord) -> proto::Hit {
    proto::Hit {
        path: record.path,
        name: record.name,
        is_dir: record.is_dir,
        size: record.size,
        mod_time_unix: record.mod_time_unix,
    }
}

fn to_protocol_status(status: &Status) -> proto::StatusResult {
    proto::StatusResult {
        state: describe_state(status.scanning).to_string(),
        files_indexed: status.files_indexed,
        scanning_path: status.scanning_path.clone(),
        watched_directories: status.watched_directories,
        watch_limit_reached: status.watch_limit_reached,
        last_scan_unix: status.last_scan_unix,
    }
}

fn describe_state(scanning: bool) -> &'static str {
    if scanning {
        return proto::STATE_SCANNING;
    }
    proto::STATE_IDLE
}

fn describe_version_mismatch(client_version: i64) -> String {
    format!("client speaks protocol {client_version}, daemon speaks {}", proto::PROTOCOL_VERSION)
}
