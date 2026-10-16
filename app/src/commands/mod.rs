//! The commands the UI calls, one module per area. Each command is named after
//! the function in `ui/backend/` that calls it, and takes the same arguments.

mod launching;
mod places;
mod preferences;
mod search;
mod typed;

use std::sync::Arc;

use svelte_gpui::Commands;

use crate::services::Services;

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let commands = places::register(commands, services);
    let commands = launching::register(commands, services);
    let commands = preferences::register(commands, services);
    search::register(commands, services)
}
