//! Looking at folders and files: listings, previews, thumbnails, git state,
//! archives, the places and devices in the sidebar, and watching for changes.

use std::sync::Arc;

use canopy_desktop::git;
use canopy_files::{archive, fileops, filesystem, thumbnail};
use canopy_model::Order;
use serde::Deserialize;
use svelte_gpui::{CommandError, Commands};

use super::typed::{NoArguments, failure, typed};
use crate::services::Services;

#[derive(Deserialize)]
struct PathArguments {
    path: String,
}

#[derive(Deserialize)]
struct ListArguments {
    path: String,
    order: Order,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveArguments {
    archive_path: String,
}

#[derive(Deserialize)]
struct PathsArguments {
    paths: Vec<String>,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let commands = register_folders(commands, services);
    let commands = register_files(commands);
    register_watching(commands, services)
}

fn register_folders(commands: Commands, services: &Arc<Services>) -> Commands {
    let listing = services.clone();
    commands
        .command("ListDirectory", typed(move |arguments: ListArguments| list_directory(&listing, arguments)))
        .command("HomeDirectory", typed(|_: NoArguments| home_directory()))
        .command("GitReport", typed(|arguments: PathArguments| Ok(git_report(&arguments.path))))
        .command("Places", typed(|_: NoArguments| Ok(filesystem::places())))
        .command("Devices", typed(|_: NoArguments| Ok(filesystem::devices())))
}

fn register_files(commands: Commands) -> Commands {
    commands
        .command("PreviewFile", typed(|arguments: PathArguments| filesystem::read_preview(&arguments.path).map_err(failure)))
        .command("ThumbnailFor", typed(|arguments: PathArguments| Ok(thumbnail::thumbnail_for(&arguments.path))))
        .command("ListArchive", typed(|arguments: ArchiveArguments| list_archive(&arguments.archive_path)))
        .command("ExtractionFolder", typed(|arguments: ArchiveArguments| extraction_folder(&arguments.archive_path)))
}

fn register_watching(commands: Commands, services: &Arc<Services>) -> Commands {
    let watching = services.clone();
    let stopping = services.clone();
    commands
        .command(
            "WatchDirectories",
            typed(move |arguments: PathsArguments| Ok(watching.watcher.watch(&resolve_all(&arguments.paths)))),
        )
        .command("StopWatching", typed(move |_: NoArguments| Ok(stopping.watcher.stop())))
}

/// A folder listed counts as visited, which is what the sidebar's recents show.
fn list_directory(services: &Services, arguments: ListArguments) -> Result<canopy_model::Listing, CommandError> {
    let listing = filesystem::read_directory(&arguments.path, &arguments.order).map_err(failure)?;
    services.recents.record(&listing.path, true);
    Ok(listing)
}

fn home_directory() -> Result<String, CommandError> {
    let home = std::env::home_dir().ok_or("no home directory is set")?;
    Ok(home.to_string_lossy().into_owned())
}

/// A folder outside a working tree reports no repository, which is not a failure.
fn git_report(path: &str) -> git::Report {
    let Ok(resolved) = filesystem::resolve_path(path) else {
        return git::Report::default();
    };
    git::describe(&resolved.to_string_lossy())
}

fn list_archive(archive_path: &str) -> Result<Vec<archive::Member>, CommandError> {
    let resolved = filesystem::resolve_path(archive_path).map_err(failure)?;
    archive::list(&resolved).map_err(failure)
}

/// Where an archive unpacks when nobody chose a folder: beside it, under a free name.
fn extraction_folder(archive_path: &str) -> Result<String, CommandError> {
    let resolved = filesystem::resolve_path(archive_path).map_err(failure)?;
    Ok(fileops::extraction_folder_for(&resolved).to_string_lossy().into_owned())
}

/// A path that cannot be resolved is dropped: the pane showing it has already
/// said why it could not be listed.
fn resolve_all(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| filesystem::resolve_path(path).ok())
        .map(|resolved| resolved.to_string_lossy().into_owned())
        .collect()
}
