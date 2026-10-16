//! The `canopyd` process: argument parsing, the daemon lock, logging, and the
//! wiring of indexer and server.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use log::LevelFilter;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::index::{Indexer, IndexerOptions};
use crate::lockfile::{self, Lock, LockError};
use crate::proto;
use crate::server::{Server, ServerOptions};
use crate::stop::StopSignal;

/// Compared by clients during the handshake and decides which bundled copy wins
/// when two applications ship different builds.
pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct Configuration {
    pub socket_path: PathBuf,
    pub database_path: PathBuf,
    /// Zero disables the idle exit.
    pub idle_timeout: Duration,
    pub rescan_interval: Duration,
    /// Zero uses the built-in default.
    pub maximum_watches: usize,
    pub log_level: LevelFilter,
    pub print_version: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            socket_path: proto::socket_path(),
            database_path: proto::index_database_path(),
            idle_timeout: Duration::from_secs(10 * 60),
            rescan_interval: Duration::from_secs(30 * 60),
            maximum_watches: 0,
            log_level: LevelFilter::Info,
            print_version: false,
        }
    }
}

impl Configuration {
    /// Parses `-name value`, `--name value` and `-name=value` flags:
    /// `socket`, `database`, `idle-timeout`, `rescan-interval`, `max-watches`,
    /// `log-level` and `version`.
    pub fn from_arguments(arguments: impl IntoIterator<Item = String>) -> Result<Configuration, String> {
        let mut configuration = Configuration::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            let (name, inline_value) = split_flag(&argument)?;
            if name == "version" {
                configuration.print_version = true;
                continue;
            }
            let value = match inline_value {
                Some(value) => value,
                None => arguments.next().ok_or_else(|| format!("flag needs an argument: -{name}"))?,
            };
            configuration.apply_flag(name, &value)?;
        }
        Ok(configuration)
    }

    fn apply_flag(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "socket" => self.socket_path = PathBuf::from(value),
            "database" => self.database_path = PathBuf::from(value),
            "idle-timeout" => self.idle_timeout = parse_duration(value)?,
            "rescan-interval" => self.rescan_interval = parse_duration(value)?,
            "max-watches" => self.maximum_watches = parse_count(name, value)?,
            "log-level" => self.log_level = parse_log_level(value),
            _ => return Err(format!("flag provided but not defined: -{name}")),
        }
        Ok(())
    }
}

fn split_flag(argument: &str) -> Result<(&str, Option<String>), String> {
    let Some(flag) = argument.strip_prefix("--").or_else(|| argument.strip_prefix('-')) else {
        return Err(format!("unexpected argument: {argument}"));
    };
    match flag.split_once('=') {
        Some((name, value)) => Ok((name, Some(value.to_string()))),
        None => Ok((flag, None)),
    }
}

fn parse_count(name: &str, value: &str) -> Result<usize, String> {
    value.parse().map_err(|_| format!("invalid value {value:?} for flag -{name}"))
}

/// Unknown levels fall back to info.
fn parse_log_level(value: &str) -> LevelFilter {
    match value.to_ascii_lowercase().as_str() {
        "debug" => LevelFilter::Debug,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    }
}

const DURATION_UNITS: &[(&str, f64)] = &[
    ("ns", 1e-9),
    ("us", 1e-6),
    ("\u{b5}s", 1e-6),
    ("ms", 1e-3),
    ("s", 1.0),
    ("m", 60.0),
    ("h", 3600.0),
];

/// Durations in Go's syntax: "10m", "1h30m", "250ms", or a bare "0".
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    let invalid = || format!("invalid duration {text:?}");
    if text == "0" {
        return Ok(Duration::ZERO);
    }
    let mut remaining = text;
    let mut total_seconds = 0.0;
    while !remaining.is_empty() {
        let number_length = remaining.find(|character: char| !character.is_ascii_digit() && character != '.');
        let (number, after_number) = remaining.split_at(number_length.ok_or_else(invalid)?);
        let (unit_seconds, after_unit) = strip_unit(after_number).ok_or_else(invalid)?;
        total_seconds += number.parse::<f64>().map_err(|_| invalid())? * unit_seconds;
        remaining = after_unit;
    }
    Ok(Duration::from_secs_f64(total_seconds))
}

fn strip_unit(text: &str) -> Option<(f64, &str)> {
    DURATION_UNITS
        .iter()
        .filter_map(|(unit, seconds)| text.strip_prefix(unit).map(|rest| (*seconds, rest, unit.len())))
        .max_by_key(|(_, _, unit_length)| *unit_length)
        .map(|(seconds, rest, _)| (seconds, rest))
}

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let time = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
        eprintln!("time={time} level={} msg={}", record.level(), record.args());
    }

    fn flush(&self) {}
}

fn install_logger(level: LevelFilter) {
    if log::set_logger(&StderrLogger).is_ok() {
        log::set_max_level(level);
    }
}

pub fn run(configuration: Configuration) -> Result<(), String> {
    if configuration.print_version {
        println!("{DAEMON_VERSION}");
        return Ok(());
    }
    install_logger(configuration.log_level);
    let Some(_lock) = claim_daemon_lock()? else {
        log::info!("another daemon already holds the lock, exiting");
        return Ok(());
    };
    serve(&configuration)
}

fn serve(configuration: &Configuration) -> Result<(), String> {
    let stop = StopSignal::new();
    stop_on_signal(stop.clone())?;
    let indexer = Arc::new(start_indexer(configuration, &stop)?);
    log::info!(
        "daemon ready version={DAEMON_VERSION} socket={} database={}",
        configuration.socket_path.display(),
        configuration.database_path.display()
    );
    let options = ServerOptions {
        socket_path: configuration.socket_path.clone(),
        idle_timeout: configuration.idle_timeout,
        daemon_version: DAEMON_VERSION.to_string(),
        index_path: configuration.database_path.to_string_lossy().into_owned(),
    };
    let outcome = Server::new(Arc::clone(&indexer), options, stop).serve();
    indexer.close();
    outcome.map_err(|error| error.to_string())
}

fn stop_on_signal(stop: StopSignal) -> Result<(), String> {
    let mut signals = Signals::new([SIGINT, SIGTERM]).map_err(|error| error.to_string())?;
    thread::spawn(move || {
        if signals.forever().next().is_some() {
            stop.stop();
        }
    });
    Ok(())
}

fn start_indexer(configuration: &Configuration, stop: &StopSignal) -> Result<Indexer, String> {
    if let Some(directory) = configuration.database_path.parent() {
        proto::ensure_private_directory(directory).map_err(|error| error.to_string())?;
    }
    let options = IndexerOptions {
        database_path: configuration.database_path.clone(),
        maximum_watches: configuration.maximum_watches,
        rescan_interval: configuration.rescan_interval,
        ..IndexerOptions::default()
    };
    let indexer = Indexer::new(stop.clone(), options).map_err(|error| error.to_string())?;
    indexer.start();
    Ok(indexer)
}

/// Losing the race is the expected outcome when two applications start at once,
/// so a held lock is `Ok(None)` rather than an error.
fn claim_daemon_lock() -> Result<Option<Lock>, String> {
    proto::ensure_private_directory(&proto::runtime_directory()).map_err(|error| error.to_string())?;
    match lockfile::try_acquire(&proto::daemon_lock_path()) {
        Ok(lock) => Ok(Some(lock)),
        Err(LockError::Busy) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Configuration, String> {
        Configuration::from_arguments(arguments.iter().map(|argument| argument.to_string()))
    }

    #[test]
    fn parses_go_style_flags() {
        let configuration =
            parse(&["-socket", "/tmp/a.sock", "--idle-timeout=0", "-max-watches", "7", "-log-level=debug"]).unwrap();
        assert_eq!(configuration.socket_path, PathBuf::from("/tmp/a.sock"));
        assert_eq!(configuration.idle_timeout, Duration::ZERO);
        assert_eq!(configuration.maximum_watches, 7);
        assert_eq!(configuration.log_level, LevelFilter::Debug);
        assert!(!configuration.print_version);
    }

    #[test]
    fn rejects_unknown_flags_and_missing_values() {
        assert!(parse(&["-nope", "1"]).is_err());
        assert!(parse(&["-socket"]).is_err());
        assert!(parse(&["stray"]).is_err());
    }

    #[test]
    fn parses_version_flag() {
        assert!(parse(&["-version"]).unwrap().print_version);
    }

    #[test]
    fn parses_durations_in_go_syntax() {
        assert_eq!(parse_duration("10m").unwrap(), Duration::from_secs(600));
        assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse_duration("250ms").unwrap(), Duration::from_millis(250));
        assert_eq!(parse_duration("0").unwrap(), Duration::ZERO);
        assert!(parse_duration("soon").is_err());
        assert!(parse_duration("5").is_err());
    }
}
