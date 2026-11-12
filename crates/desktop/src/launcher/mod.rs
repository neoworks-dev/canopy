//! Opens files and folders with the applications the desktop knows about:
//! `.desktop` entries, the per-type default this explorer remembers, and the
//! system terminal.

mod desktop_entry;
mod environment;
mod exec_line;
mod mime;
mod open;
mod overrides;
mod process;
mod shell;
mod terminal;

pub use desktop_entry::Application;
pub use mime::mime_type_of;
pub use open::{applications_for, open, open_with};
pub use overrides::{clear_default, set_default};
pub use shell::run_shell;
pub use terminal::open_terminal;
