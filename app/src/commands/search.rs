//! Finding things: the shared search index, saved searches and zoxide's ranked
//! directories.

use std::sync::Arc;

use canopy_desktop::searches::Search;
use serde::Deserialize;
use svelte_gpui::Commands;

use super::typed::{NoArguments, failure, typed};
use crate::services::Services;

/// Enough ranked directories to pick from without turning the prompt into a
/// list to read.
const ZOXIDE_RESULT_LIMIT: usize = 12;

#[derive(Deserialize)]
struct QueryArguments {
    query: String,
    scope: String,
}

#[derive(Deserialize)]
struct ZoxideArguments {
    query: String,
    exclude: String,
}

#[derive(Deserialize)]
struct SaveSearchArguments {
    name: String,
    query: String,
    scope: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameSearchArguments {
    name: String,
    new_name: String,
}

#[derive(Deserialize)]
struct NameArguments {
    name: String,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let commands = register_index(commands, services);
    let commands = register_saved_searches(commands, services);
    register_zoxide(commands, services)
}

fn register_index(commands: Commands, services: &Arc<Services>) -> Commands {
    let searching = services.clone();
    let describing = services.clone();
    let reporting = services.clone();
    commands
        .command(
            "Search",
            typed(move |arguments: QueryArguments| {
                searching.index.search(&arguments.query, &arguments.scope).map_err(failure)
            }),
        )
        .command(
            "SearchEntries",
            typed(move |arguments: QueryArguments| {
                describing.index.search_entries(&arguments.query, &arguments.scope).map_err(failure)
            }),
        )
        .command("IndexStatus", typed(move |_: NoArguments| reporting.index.status().map_err(failure)))
}

fn register_saved_searches(commands: Commands, services: &Arc<Services>) -> Commands {
    let listing = services.clone();
    let saving = services.clone();
    let renaming = services.clone();
    let forgetting = services.clone();
    commands
        .command("SavedSearches", typed(move |_: NoArguments| Ok(listing.searches.list())))
        .command(
            "SaveSearch",
            typed(move |arguments: SaveSearchArguments| {
                let search = Search { name: arguments.name, query: arguments.query, scope: arguments.scope };
                saving.searches.save(search).map_err(failure)
            }),
        )
        .command(
            "RenameSearch",
            typed(move |arguments: RenameSearchArguments| {
                renaming.searches.rename(&arguments.name, &arguments.new_name).map_err(failure)
            }),
        )
        .command(
            "ForgetSearch",
            typed(move |arguments: NameArguments| forgetting.searches.forget(&arguments.name).map_err(failure)),
        )
}

fn register_zoxide(commands: Commands, services: &Arc<Services>) -> Commands {
    let querying = services.clone();
    let checking = services.clone();
    commands
        .command(
            "ZoxideQuery",
            typed(move |arguments: ZoxideArguments| {
                Ok(querying.zoxide.query(&arguments.query, &arguments.exclude, ZOXIDE_RESULT_LIMIT))
            }),
        )
        .command("ZoxideAvailable", typed(move |_: NoArguments| Ok(checking.zoxide.available())))
}
