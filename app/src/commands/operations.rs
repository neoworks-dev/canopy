//! Changing the filesystem: queued operations and their conflicts, undo and
//! redo, renaming, creating entries and links, and the trash.

use std::path::Path;
use std::sync::Arc;

use canopy_files::fileops::{self, Kind, LinkKind, Resolution, TrashedItem};
use serde::Deserialize;
use svelte_gpui::{CommandError, Commands};

use super::typed::{NoArguments, failure, typed};
use crate::services::Services;

#[derive(Deserialize)]
struct StartArguments {
    kind: String,
    sources: Vec<String>,
    destination: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveArguments {
    operation_id: String,
    resolution: String,
    apply_to_all: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationArguments {
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameArguments {
    path: String,
    new_name: String,
}

#[derive(Deserialize)]
struct CreateArguments {
    directory: String,
    name: String,
}

#[derive(Deserialize)]
struct LinkArguments {
    sources: Vec<String>,
    directory: String,
    kind: String,
}

#[derive(Deserialize)]
struct NameArguments {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrashArguments {
    trash_path: String,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let commands = register_queue(commands, services);
    let commands = register_history(commands, services);
    let commands = register_entries(commands, services);
    let commands = commands.command("CreateLinks", typed(create_links));
    register_trash(commands, services)
}

fn register_queue(commands: Commands, services: &Arc<Services>) -> Commands {
    let starting = services.clone();
    let resolving = services.clone();
    let cancelling = services.clone();
    let pausing = services.clone();
    let resuming = services.clone();
    commands
        .command("StartOperation", typed(move |arguments: StartArguments| start_operation(&starting, arguments)))
        .command("ResolveConflict", typed(move |arguments: ResolveArguments| resolve_conflict(&resolving, arguments)))
        .command(
            "CancelOperation",
            typed(move |arguments: OperationArguments| cancelling.engine.cancel(&arguments.operation_id).map_err(failure)),
        )
        .command(
            "PauseOperation",
            typed(move |arguments: OperationArguments| pausing.engine.pause(&arguments.operation_id).map_err(failure)),
        )
        .command(
            "ResumeOperation",
            typed(move |arguments: OperationArguments| resuming.engine.resume(&arguments.operation_id).map_err(failure)),
        )
}

fn register_history(commands: Commands, services: &Arc<Services>) -> Commands {
    let undoing = services.clone();
    let redoing = services.clone();
    let undo_labelling = services.clone();
    let redo_labelling = services.clone();
    commands
        .command("Undo", typed(move |_: NoArguments| undoing.engine.undo().map_err(failure)))
        .command("Redo", typed(move |_: NoArguments| redoing.engine.redo().map_err(failure)))
        .command("UndoLabel", typed(move |_: NoArguments| Ok(undo_labelling.engine.undo_label())))
        .command("RedoLabel", typed(move |_: NoArguments| Ok(redo_labelling.engine.redo_label())))
}

fn register_entries(commands: Commands, services: &Arc<Services>) -> Commands {
    let renaming = services.clone();
    let creating_folders = services.clone();
    let creating_files = services.clone();
    commands
        .command(
            "RenameEntry",
            typed(move |arguments: RenameArguments| {
                renaming.engine.rename(&arguments.path, &arguments.new_name).map_err(failure)
            }),
        )
        .command(
            "CreateFolder",
            typed(move |arguments: CreateArguments| {
                creating_folders.engine.create_folder(&arguments.directory, &arguments.name).map_err(failure)
            }),
        )
        .command(
            "CreateFile",
            typed(move |arguments: CreateArguments| {
                creating_files.engine.create_file(&arguments.directory, &arguments.name).map_err(failure)
            }),
        )
        .command("ValidateEntryName", typed(|arguments: NameArguments| fileops::validate_name(&arguments.name).map_err(failure)))
        .command(
            "SuggestName",
            typed(|arguments: CreateArguments| Ok(fileops::suggest_name(Path::new(&arguments.directory), &arguments.name))),
        )
}

fn register_trash(commands: Commands, services: &Arc<Services>) -> Commands {
    let restoring = services.clone();
    commands
        .command("ListTrash", typed(|_: NoArguments| list_trash()))
        .command(
            "RestoreFromTrash",
            typed(move |arguments: TrashArguments| restoring.engine.restore_trashed(&arguments.trash_path).map_err(failure)),
        )
        .command("EmptyTrash", typed(|_: NoArguments| fileops::empty_trash().map_err(failure)))
}

/// Queues the operation and answers with the id its progress events carry.
fn start_operation(services: &Services, arguments: StartArguments) -> Result<String, CommandError> {
    let kind: Kind = arguments.kind.parse().map_err(failure)?;
    services.engine.enqueue(kind, &arguments.sources, &arguments.destination).map_err(failure)
}

fn resolve_conflict(services: &Services, arguments: ResolveArguments) -> Result<(), CommandError> {
    let resolution = Resolution::from_name(&arguments.resolution);
    services.engine.resolve(&arguments.operation_id, resolution, arguments.apply_to_all).map_err(failure)
}

fn create_links(arguments: LinkArguments) -> Result<Vec<String>, CommandError> {
    let kind = LinkKind::from_name(&arguments.kind).map_err(failure)?;
    fileops::create_links(&arguments.sources, &arguments.directory, kind).map_err(failure)
}

fn list_trash() ->Result<Vec<TrashedItem>, CommandError> {
    fileops::list_trashed().map_err(failure)
}
