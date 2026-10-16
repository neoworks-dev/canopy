//! Work over a whole tree that reports as it goes: measuring a folder and
//! finding duplicate files. Each start answers with the id its events carry.

use std::sync::Arc;

use canopy_files::filesystem;
use serde::Deserialize;
use svelte_gpui::{CommandError, Commands};

use super::typed::{failure, typed};
use crate::services::Services;

#[derive(Deserialize)]
struct PathArguments {
    path: String,
}

#[derive(Deserialize)]
struct RootArguments {
    root: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeasurementArguments {
    measurement_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchArguments {
    search_id: String,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let measuring = services.clone();
    let cancelling_measurement = services.clone();
    let searching = services.clone();
    let cancelling_search = services.clone();
    commands
        .command("MeasureFolder", typed(move |arguments: PathArguments| measure_folder(&measuring, &arguments.path)))
        .command(
            "CancelMeasurement",
            typed(move |arguments: MeasurementArguments| {
                cancelling_measurement.folder_sizes.cancel(&arguments.measurement_id).map_err(failure)
            }),
        )
        .command("FindDuplicates", typed(move |arguments: RootArguments| find_duplicates(&searching, &arguments.root)))
        .command(
            "CancelDuplicateSearch",
            typed(move |arguments: SearchArguments| {
                cancelling_search.duplicates.cancel(&arguments.search_id).map_err(failure)
            }),
        )
}

fn measure_folder(services: &Services, path: &str) -> Result<String, CommandError> {
    let resolved = filesystem::resolve_path(path).map_err(failure)?;
    services.folder_sizes.start(resolved).map_err(failure)
}

fn find_duplicates(services: &Services, root: &str) -> Result<String, CommandError> {
    let resolved = filesystem::resolve_path(root).map_err(failure)?;
    services.duplicates.start(resolved).map_err(failure)
}
