//! The settings file: what the UI saves, and the window size kept beside it.

use std::sync::Arc;

use canopy_desktop::settings::{Settings, Window};
use serde::Deserialize;
use svelte_gpui::Commands;

use super::typed::{NoArguments, typed};
use crate::services::Services;

#[derive(Deserialize)]
struct SaveArguments {
    next: Settings,
}

#[derive(Deserialize)]
struct WindowSizeArguments {
    width: i64,
    height: i64,
}

pub fn register(commands: Commands, services: &Arc<Services>) -> Commands {
    let loading = services.clone();
    let saving = services.clone();
    let sizing = services.clone();
    commands
        .command("LoadSettings", typed(move |_: NoArguments| Ok(current(&loading))))
        .command("SaveSettings", typed(move |arguments: SaveArguments| Ok(save(&saving, arguments.next))))
        .command(
            "RememberWindowSize",
            typed(move |arguments: WindowSizeArguments| Ok(remember_window_size(&sizing, arguments))),
        )
}

fn current(services: &Services) -> Settings {
    services.current_settings.lock().unwrap().clone()
}

/// The UI does not know the window's geometry, so the saved settings keep the
/// size last recorded.
fn save(services: &Services, mut next: Settings) {
    let mut current = services.current_settings.lock().unwrap();
    next.window = current.window.clone();
    *current = next.clone();
    services.settings.save(next);
}

/// The compositor decides where the window goes, so only its size is kept.
fn remember_window_size(services: &Services, size: WindowSizeArguments) {
    let mut current = services.current_settings.lock().unwrap();
    current.window = Window { width: size.width, height: size.height, x: 0, y: 0 };
    services.settings.save(current.clone());
}
