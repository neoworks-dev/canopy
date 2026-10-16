//! Canopy's executable: the svelte-gpui host running the UI bundle, with the
//! commands the UI calls registered from `commands`.

mod commands;
mod services;
mod startup;

use svelte_gpui::{Commands, Options};

fn main() {
    let commands = Commands::new();
    let services = services::Services::start(commands.emitter());
    let commands = commands::register(commands, &services);
    let mut options = options().commands(commands);
    startup::open_folder_argument(&mut services.current_settings.lock().unwrap());
    startup::apply_window_size(&mut options, &services.current_settings.lock().unwrap());
    svelte_gpui::run(options);
    services.shut_down();
}

#[cfg(feature = "package")]
fn options() -> Options {
    Options::embedded(svelte_gpui::embed!())
}

#[cfg(not(feature = "package"))]
fn options() -> Options {
    Options::from_command_line().unwrap_or_else(|| {
        eprintln!("usage: canopy --bundle dist/canopy.js");
        std::process::exit(2);
    })
}
