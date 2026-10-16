//! The commands the UI calls, one module per area. Each command is named after
//! the function in `ui/backend/` that calls it, and takes the same arguments.

mod launching;
mod listing;
mod operations;
mod places;
mod preferences;
mod scans;
mod search;
mod typed;

use std::sync::Arc;

use svelte_gpui::Commands;

use crate::services::Services;

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let commands = listing::register(commands, services);
    let commands = operations::register(commands, services);
    let commands = scans::register(commands, services);
    let commands = places::register(commands, services);
    let commands = launching::register(commands, services);
    let commands = preferences::register(commands, services);
    search::register(commands, services)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every `invoke("Name"` in the UI's backend modules.
    fn names_the_ui_calls() -> BTreeSet<String> {
        let backend = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/backend");
        let mut names = BTreeSet::new();
        for file in std::fs::read_dir(backend).unwrap() {
            let source = std::fs::read_to_string(file.unwrap().path()).unwrap();
            names.extend(source.split("invoke<").skip(1).filter_map(command_name));
        }
        names
    }

    fn command_name(call: &str) -> Option<String> {
        let quoted = call.split_once("(\"")?.1;
        Some(quoted.split_once('"')?.0.to_string())
    }

    #[test]
    fn every_command_the_ui_calls_is_registered() {
        let sandbox = tempfile::tempdir().unwrap();
        // SAFETY: set before any thread reads the environment.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", sandbox.path().join("config"));
            std::env::set_var("XDG_DATA_HOME", sandbox.path().join("data"));
        }
        let commands = Commands::new();
        let services = Services::start(commands.emitter());
        let registered: BTreeSet<String> = register(commands, &services).names().into_iter().collect();
        let called = names_the_ui_calls();
        assert!(called.len() > 50, "found only {} commands in ui/backend", called.len());
        let missing: Vec<&String> = called.difference(&registered).collect();
        assert!(missing.is_empty(), "commands the UI calls but the app does not register: {missing:?}");
    }
}
