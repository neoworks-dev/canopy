//! The sidebar's lists: pinned folders, recent entries and tags.

use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use svelte_gpui::Commands;

use super::typed::{NoArguments, failure, typed};
use crate::services::Services;
use canopy_desktop::{bookmarks, tags};

#[derive(Deserialize)]
struct PathArguments {
    path: String,
}

#[derive(Deserialize)]
struct NameArguments {
    name: String,
}

#[derive(Deserialize)]
struct TagArguments {
    paths: Vec<String>,
    name: String,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let commands = register_bookmarks(commands);
    let commands = register_recents(commands, services);
    register_tags(commands, services)
}

fn register_bookmarks(commands: Commands) -> Commands {
    commands
        .command("Bookmarks", typed(|_: NoArguments| Ok(bookmarks::list())))
        .command(
            "AddBookmark",
            typed(|arguments: PathArguments| bookmarks::add(&arguments.path, &base_name(&arguments.path)).map_err(failure)),
        )
        .command("RemoveBookmark", typed(|arguments: PathArguments| bookmarks::remove(&arguments.path).map_err(failure)))
}

fn register_recents(commands: Commands, services: &Arc<Services>) -> Commands {
    let listing = services.clone();
    let clearing = services.clone();
    commands
        .command("RecentEntries", typed(move |_: NoArguments| Ok(listing.recents.list())))
        .command("ClearRecents", typed(move |_: NoArguments| Ok(clearing.recents.clear())))
}

fn register_tags(commands: Commands, services: &Arc<Services>) -> Commands {
    let adding = services.clone();
    let naming = services.clone();
    let forgetting = services.clone();
    commands
        .command("TagsOf", typed(|arguments: PathArguments| Ok(tags::of(&arguments.path))))
        .command("AddTag", typed(move |arguments: TagArguments| add_tag(&adding, arguments)))
        .command("RemoveTag", typed(|arguments: TagArguments| remove_tag(arguments)))
        .command("KnownTags", typed(move |_: NoArguments| Ok(naming.tag_names.names())))
        .command("ForgetTag", typed(move |arguments: NameArguments| Ok(forgetting.tag_names.forget(&arguments.name))))
        .command("FilesWithTag", typed(|arguments: NameArguments| Ok(files_with_tag(&arguments.name))))
}

/// The first failure stops the run, so a selection is not left half labelled
/// without anybody being told.
fn add_tag(services: &Services, arguments: TagArguments) -> Result<(), svelte_gpui::CommandError> {
    for path in &arguments.paths {
        tags::add(path, &arguments.name).map_err(failure)?;
    }
    services.tag_names.remember(&[arguments.name]);
    Ok(())
}

fn remove_tag(arguments: TagArguments) -> Result<(), svelte_gpui::CommandError> {
    for path in &arguments.paths {
        tags::remove(path, &arguments.name).map_err(failure)?;
    }
    Ok(())
}

/// Tags live on the files themselves, so finding them means walking the home directory.
fn files_with_tag(name: &str) -> Vec<canopy_model::Entry> {
    let Some(home) = std::env::home_dir() else {
        return Vec::new();
    };
    tags::search(&home.to_string_lossy(), name)
}

fn base_name(path: &str) -> String {
    match Path::new(path).file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.to_string(),
    }
}
