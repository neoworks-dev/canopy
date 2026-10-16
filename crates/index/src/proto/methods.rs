use serde::{Deserialize, Serialize};

/// The first message on every connection.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HandshakeParams {
    pub client_name: String,
    pub protocol_version: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HandshakeResult {
    pub daemon_version: String,
    pub protocol_version: i64,
    pub index_path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AddRootParams {
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoveRootParams {
    pub path: String,
}

/// One indexed directory tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Root {
    pub path: String,
    pub file_count: i64,
    pub last_scan_unix: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RootsResult {
    pub roots: Vec<Root>,
}

/// An empty `path` rescans every root.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RescanParams {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub path: String,
}

pub const STATE_IDLE: &str = "idle";
pub const STATE_SCANNING: &str = "scanning";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusResult {
    pub state: String,
    pub files_indexed: i64,
    pub scanning_path: String,
    pub watched_directories: i64,
    pub watch_limit_reached: bool,
    pub last_scan_unix: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchParams {
    pub text: String,
    pub limit: i64,
    pub directories_only: bool,
    /// Limits results to a directory and everything under it. Empty searches
    /// every indexed root.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scope: String,
}

/// One search result. Times are Unix seconds so the wire format stays free of
/// timezone ambiguity.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Hit {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: i64,
    pub mod_time_unix: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchResult {
    pub hits: Vec<Hit>,
    /// The index held more matches than `limit` allowed.
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmptyResult {}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn search_result_serialises_with_the_field_names_the_frontend_reads() {
        let result = SearchResult {
            hits: vec![Hit { path: "/a/b.txt".into(), name: "b.txt".into(), is_dir: false, size: 3, mod_time_unix: 9 }],
            truncated: true,
        };
        let expected = json!({
            "hits": [{"path": "/a/b.txt", "name": "b.txt", "isDir": false, "size": 3, "modTimeUnix": 9}],
            "truncated": true,
        });
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn status_result_serialises_with_the_field_names_the_frontend_reads() {
        let value = serde_json::to_value(StatusResult::default()).unwrap();
        let mut names: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
        names.sort_unstable();
        let expected = ["filesIndexed", "lastScanUnix", "scanningPath", "state", "watchLimitReached", "watchedDirectories"];
        assert_eq!(names, expected);
    }

    #[test]
    fn empty_scope_and_path_are_omitted_from_requests() {
        assert_eq!(serde_json::to_value(RescanParams::default()).unwrap(), json!({}));
        let params = SearchParams { text: "x".into(), limit: 5, ..SearchParams::default() };
        assert_eq!(serde_json::to_value(params).unwrap(), json!({"text": "x", "limit": 5, "directoriesOnly": false}));
    }
}
