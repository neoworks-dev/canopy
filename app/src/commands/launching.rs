//! Handing entries to other programs: opening them, a terminal, a chosen
//! application, and the system clipboard's file list.

use std::sync::Arc;

use serde::Deserialize;
use svelte_gpui::{CommandError, Commands};

use super::typed::{failure, typed};
use crate::services::Services;
use canopy_desktop::{clipboard, launcher};

#[derive(Deserialize)]
struct PathArguments {
    path: String,
}

#[derive(Deserialize)]
struct DirectoryArguments {
    directory: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationArguments {
    path: String,
    application_id: String,
}

#[derive(Deserialize)]
struct PathsArguments {
    paths: Vec<String>,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let opening = services.clone();
    commands
        .command("OpenPath", typed(move |arguments: PathArguments| open_path(&opening, &arguments.path)))
        .command(
            "OpenTerminal",
            typed(|arguments: DirectoryArguments| launcher::open_terminal(&arguments.directory).map_err(failure)),
        )
        .command("ApplicationsFor", typed(|arguments: PathArguments| Ok(launcher::applications_for(&arguments.path))))
        .command(
            "OpenWith",
            typed(|arguments: ApplicationArguments| {
                launcher::open_with(&arguments.path, &arguments.application_id).map_err(failure)
            }),
        )
        .command(
            "SetDefaultApplication",
            typed(|arguments: ApplicationArguments| {
                launcher::set_default(&arguments.path, &arguments.application_id).map_err(failure)
            }),
        )
        .command(
            "CopyPathsToClipboard",
            typed(|arguments: PathsArguments| clipboard::write_paths(&arguments.paths).map_err(failure)),
        )
        .command("ReadClipboardPaths", typed(|_: super::typed::NoArguments| clipboard::read_paths().map_err(failure)))
}

/// A file opened from here counts as a recent entry, as a folder listed does.
fn open_path(services: &Services, path: &str) -> Result<(), CommandError> {
    launcher::open(path).map_err(failure)?;
    services.recents.record(path, false);
    Ok(())
}
