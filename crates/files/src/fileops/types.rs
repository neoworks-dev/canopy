use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// The family of a queued operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Copy,
    Move,
    Trash,
    Delete,
    Extract,
    Compress,
}

impl FromStr for Kind {
    type Err = Error;

    /// Reads the name the UI sends (`"copy"`, `"move"`, ...).
    fn from_str(name: &str) -> Result<Kind, Error> {
        match name {
            "copy" => Ok(Kind::Copy),
            "move" => Ok(Kind::Move),
            "trash" => Ok(Kind::Trash),
            "delete" => Ok(Kind::Delete),
            "extract" => Ok(Kind::Extract),
            "compress" => Ok(Kind::Compress),
            _ => Err(Error::UnknownRequest),
        }
    }
}

/// The caller's answer to a destination conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Resolution {
    Replace,
    Skip,
    KeepBoth,
    Cancel,
}

impl Resolution {
    /// Reads the name the UI sends (`"replace"`, `"skip"`, `"keepBoth"`,
    /// `"cancel"`). Any other answer cancels the operation.
    pub fn from_name(name: &str) -> Resolution {
        match name {
            "replace" => Resolution::Replace,
            "skip" => Resolution::Skip,
            "keepBoth" => Resolution::KeepBoth,
            _ => Resolution::Cancel,
        }
    }
}

/// The lifecycle position of one operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Queued,
    Running,
    Paused,
    Done,
    Cancelled,
}

/// Event names the engine emits. The UI subscribes to these.
pub const EVENT_PROGRESS: &str = "fileops:progress";
pub const EVENT_CONFLICT: &str = "fileops:conflict";
pub const EVENT_FINISHED: &str = "fileops:finished";

/// One item that could not be processed. A batch keeps going after a failure
/// and reports them all at the end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub path: String,
    pub message: String,
}

/// The snapshot the UI renders for one operation. The byte totals are zero for
/// kinds that move no data, where the item counter is the only honest measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub kind: Kind,
    pub status: Status,
    pub destination: String,
    pub total_items: usize,
    pub done_items: usize,
    pub total_bytes: i64,
    pub done_bytes: i64,
    pub current_path: String,
    pub failures: Vec<Failure>,
}

/// Asks the UI what to do about an occupied destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPrompt {
    pub operation_id: String,
    pub source_path: String,
    pub destination_path: String,
    pub source_is_dir: bool,
}

/// Something the engine tells its caller about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationEvent {
    /// An operation moved on: queued, started, paused, resumed, or copied more.
    Progress(Progress),
    /// An operation is blocked until `Engine::resolve` answers.
    Conflict(ConflictPrompt),
    /// An operation ended, whether done or cancelled.
    Finished(Progress),
}

impl OperationEvent {
    /// The event name the UI subscribes to (`fileops:progress`, ...).
    pub fn name(&self) -> &'static str {
        match self {
            OperationEvent::Progress(_) => EVENT_PROGRESS,
            OperationEvent::Conflict(_) => EVENT_CONFLICT,
            OperationEvent::Finished(_) => EVENT_FINISHED,
        }
    }

    /// The payload, serialised exactly as the UI expects it.
    pub fn payload(&self) -> serde_json::Value {
        let payload = match self {
            OperationEvent::Progress(progress) | OperationEvent::Finished(progress) => serde_json::to_value(progress),
            OperationEvent::Conflict(prompt) => serde_json::to_value(prompt),
        };
        payload.unwrap_or(serde_json::Value::Null)
    }
}

/// Delivers an event to the caller. It runs on the engine's worker thread, so
/// it should hand the event on quickly.
pub type OperationEmitter = Arc<dyn Fn(OperationEvent) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_serialises_with_the_go_field_names() {
        let progress = Progress {
            id: "op-1".to_string(),
            kind: Kind::Copy,
            status: Status::Running,
            destination: "/d".to_string(),
            total_items: 2,
            done_items: 1,
            total_bytes: 10,
            done_bytes: 5,
            current_path: "/s".to_string(),
            failures: vec![Failure { path: "/x".to_string(), message: "boom".to_string() }],
        };
        let value = OperationEvent::Progress(progress).payload();
        assert_eq!(
            value,
            serde_json::json!({
                "id": "op-1", "kind": "copy", "status": "running", "destination": "/d",
                "totalItems": 2, "doneItems": 1, "totalBytes": 10, "doneBytes": 5,
                "currentPath": "/s", "failures": [{"path": "/x", "message": "boom"}]
            })
        );
    }

    #[test]
    fn conflict_prompts_serialise_with_the_go_field_names() {
        let prompt = ConflictPrompt {
            operation_id: "op-1".to_string(),
            source_path: "/s".to_string(),
            destination_path: "/d".to_string(),
            source_is_dir: false,
        };
        assert_eq!(
            OperationEvent::Conflict(prompt).payload(),
            serde_json::json!({"operationId": "op-1", "sourcePath": "/s", "destinationPath": "/d", "sourceIsDir": false})
        );
    }

    #[test]
    fn names_from_the_ui_are_understood() {
        assert_eq!("extract".parse::<Kind>().unwrap(), Kind::Extract);
        assert!("explode".parse::<Kind>().is_err());
        assert_eq!(Resolution::from_name("keepBoth"), Resolution::KeepBoth);
        assert_eq!(Resolution::from_name("whatever"), Resolution::Cancel);
    }
}
