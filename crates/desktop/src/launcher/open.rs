use std::io;

use super::desktop_entry::{Application, DesktopEntry};
use super::environment::Environment;
use super::exec_line::build_command;
use super::mime::mime_type_of;

/// Launches a path with the remembered override for its type, or with the
/// system default when there is none.
pub fn open(path: &str) -> io::Result<()> {
    Environment::from_process().open(path)
}

/// Launches a path with one specific desktop entry.
pub fn open_with(path: &str, application_id: &str) -> io::Result<()> {
    Environment::from_process().open_with(path, application_id)
}

/// The desktop entries registered for a path's MIME type.
pub fn applications_for(path: &str) -> Vec<Application> {
    Environment::from_process().applications_for(path)
}

impl Environment {
    pub(crate) fn open(&self, path: &str) -> io::Result<()> {
        let application_id = self.override_for(&mime_type_of(path));
        if !application_id.is_empty() {
            return self.open_with(path, &application_id);
        }
        self.run_detached(None, "xdg-open", &[path.to_string()])
    }

    pub(crate) fn open_with(&self, path: &str, application_id: &str) -> io::Result<()> {
        let entry = self.load_application(application_id)?;
        if entry.terminal {
            let message = format!("{} needs a terminal, which this explorer cannot provide yet", entry.name);
            return Err(io::Error::other(message));
        }
        let command = build_command(&entry.exec, path);
        let Some((program, arguments)) = command.split_first() else {
            return Err(io::Error::other(format!("{application_id} has no usable command line")));
        };
        self.run_detached(None, program, arguments)
    }

    pub(crate) fn applications_for(&self, path: &str) -> Vec<Application> {
        let mut applications = Vec::new();
        for desktop_id in self.desktop_ids_for(&mime_type_of(path)) {
            if let Some(application) = self.application_for(&desktop_id) {
                applications.push(application);
            }
        }
        applications
    }

    fn application_for(&self, desktop_id: &str) -> Option<Application> {
        let entry = self.load_entry(desktop_id)?;
        if entry.no_display {
            return None;
        }
        Some(Application {
            id: desktop_id.to_string(),
            name: display_name(&entry, desktop_id),
            icon: entry.icon.clone(),
            exec: entry.exec.clone(),
        })
    }

    pub(crate) fn load_application(&self, desktop_id: &str) -> io::Result<DesktopEntry> {
        match self.load_entry(desktop_id) {
            Some(entry) => Ok(entry),
            None => Err(io::Error::other(format!("no launchable desktop entry named {desktop_id}"))),
        }
    }
}

fn display_name(entry: &DesktopEntry, desktop_id: &str) -> String {
    if !entry.name.is_empty() {
        return entry.name.clone();
    }
    desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id).to_string()
}
