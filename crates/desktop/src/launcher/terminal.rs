use std::io;
use std::path::Path;

use super::environment::Environment;
use super::exec_line::{split_exec, without_field_codes};

/// The association a desktop declares its default terminal under.
const TERMINAL_SCHEME_TYPE: &str = "x-scheme-handler/terminal";

/// The last resort for a system that declares no default.
const KNOWN_TERMINALS: [&str; 12] = [
    "ghostty",
    "kitty",
    "alacritty",
    "wezterm",
    "foot",
    "konsole",
    "gnome-terminal",
    "xfce4-terminal",
    "tilix",
    "terminator",
    "urxvt",
    "xterm",
];

/// Starts a terminal in a directory. The terminal inherits its working
/// directory from the process that starts it, which avoids every emulator's own
/// spelling of a directory flag.
pub fn open_terminal(directory: &str) -> io::Result<()> {
    Environment::from_process().open_terminal(directory)
}

impl Environment {
    pub(crate) fn open_terminal(&self, directory: &str) -> io::Result<()> {
        require_directory(directory)?;
        let command = self.terminal_command();
        let Some((program, arguments)) = command.split_first() else {
            return Err(io::Error::other("no terminal emulator found"));
        };
        self.run_detached(Some(Path::new(directory)), program, arguments)
    }

    /// The terminal to start: the user's `$TERMINAL` first, then the desktop
    /// entry registered for the terminal scheme, then whichever known emulator
    /// is installed. Empty when there is none.
    pub(crate) fn terminal_command(&self) -> Vec<String> {
        if self.is_installed(&self.terminal) {
            return vec![self.terminal.clone()];
        }
        if let Some(from_desktop) = self.default_terminal_entry() {
            return from_desktop;
        }
        self.first_installed(&KNOWN_TERMINALS)
    }

    fn default_terminal_entry(&self) -> Option<Vec<String>> {
        self.desktop_ids_for(TERMINAL_SCHEME_TYPE)
            .iter()
            .find_map(|desktop_id| self.terminal_command_of(desktop_id))
    }

    fn terminal_command_of(&self, desktop_id: &str) -> Option<Vec<String>> {
        let entry = self.load_entry(desktop_id)?;
        let command = without_field_codes(split_exec(&entry.exec));
        if !self.is_installed(command.first()?) {
            return None;
        }
        Some(command)
    }

    pub(crate) fn first_installed(&self, names: &[&str]) -> Vec<String> {
        match names.iter().find(|name| self.is_installed(name)) {
            Some(name) => vec![name.to_string()],
            None => Vec::new(),
        }
    }
}

fn require_directory(directory: &str) -> io::Result<()> {
    let metadata = std::fs::metadata(directory)?;
    if !metadata.is_dir() {
        return Err(io::Error::other(format!("{directory} is not a directory")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use super::*;

    struct IsolatedTerminals {
        _root: tempfile::TempDir,
        binaries: PathBuf,
        applications: PathBuf,
        environment: Environment,
    }

    fn isolated_terminals() -> IsolatedTerminals {
        let root = tempfile::tempdir().unwrap();
        let binaries = root.path().join("bin");
        let applications = root.path().join("data").join("applications");
        fs::create_dir_all(&binaries).unwrap();
        fs::create_dir_all(&applications).unwrap();
        let environment = Environment {
            data_directories: vec![root.path().join("data")],
            executable_directories: vec![binaries.clone()],
            ..Environment::default()
        };
        IsolatedTerminals { _root: root, binaries, applications, environment }
    }

    impl IsolatedTerminals {
        fn install(&self, name: &str) {
            let path = self.binaries.join(name);
            fs::write(&path, "#!/bin/sh\n").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        fn declare_default(&self, desktop_id: &str, exec_line: &str) {
            let association = format!("[Default Applications]\n{TERMINAL_SCHEME_TYPE}={desktop_id};\n");
            fs::write(self.applications.join("mimeapps.list"), association).unwrap();
            let entry = format!("[Desktop Entry]\nType=Application\nName=Terminal\n{exec_line}\n");
            fs::write(self.applications.join(desktop_id), entry).unwrap();
        }
    }

    #[test]
    fn prefers_the_environment() {
        let mut terminals = isolated_terminals();
        terminals.install("faketerm");
        terminals.install("otherterm");
        terminals.declare_default("other.desktop", "Exec=otherterm %u");
        terminals.environment.terminal = "faketerm".to_string();

        assert_eq!(terminals.environment.terminal_command(), ["faketerm"]);
    }

    #[test]
    fn falls_back_to_the_desktop_default() {
        let mut terminals = isolated_terminals();
        terminals.install("otherterm");
        terminals.declare_default("other.desktop", "Exec=otherterm --login %u");
        terminals.environment.terminal = "not-installed".to_string();

        assert_eq!(terminals.environment.terminal_command(), ["otherterm", "--login"]);
    }

    #[test]
    fn ignores_a_default_that_is_not_installed() {
        let terminals = isolated_terminals();
        terminals.declare_default("missing.desktop", "Exec=missingterm %u");

        assert!(terminals.environment.terminal_command().is_empty());
    }

    #[test]
    fn first_installed_skips_missing_candidates() {
        let terminals = isolated_terminals();
        terminals.install("thirdterm");

        assert_eq!(terminals.environment.first_installed(&["missingterm", "thirdterm"]), ["thirdterm"]);
    }

    #[test]
    fn open_terminal_rejects_a_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.txt");
        fs::write(&path, "text").unwrap();

        assert!(open_terminal(path.to_str().unwrap()).is_err());
    }
}
