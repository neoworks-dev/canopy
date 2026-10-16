//! Canopy's executable: the svelte-gpui host running the UI bundle, with the
//! commands the UI calls registered in `commands`.

use svelte_gpui::{Commands, Options};

fn main() {
    let commands = Commands::new();
    svelte_gpui::run(options().commands(commands));
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
