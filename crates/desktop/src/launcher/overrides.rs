use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;
use std::sync::Mutex;

use super::environment::Environment;
use super::mime::mime_type_of;

/// Serialises the read-modify-write of the override file so two quick "always
/// open with" choices cannot lose one another.
static OVERRIDES_LOCK: Mutex<()> = Mutex::new(());

/// Remembers an application for the MIME type of `path`. The choice is kept in
/// this application's own config rather than rewritten into the user's
/// `mimeapps.list`.
pub fn set_default(path: &str, application_id: &str) -> io::Result<()> {
    Environment::from_process().set_default(path, application_id)
}

/// Drops the remembered application for the MIME type of `path`.
pub fn clear_default(path: &str) -> io::Result<()> {
    Environment::from_process().clear_default(path)
}

impl Environment {
    pub(crate) fn set_default(&self, path: &str, application_id: &str) -> io::Result<()> {
        self.load_application(application_id)?;
        let _guard = lock_overrides();
        let mut overrides = self.read_overrides();
        overrides.insert(mime_type_of(path), application_id.to_string());
        self.write_overrides(&overrides)
    }

    pub(crate) fn clear_default(&self, path: &str) -> io::Result<()> {
        let _guard = lock_overrides();
        let mut overrides = self.read_overrides();
        overrides.remove(&mime_type_of(path));
        self.write_overrides(&overrides)
    }

    /// The remembered application for a MIME type, empty when there is none.
    pub(crate) fn override_for(&self, mime_type: &str) -> String {
        let _guard = lock_overrides();
        self.read_overrides().remove(mime_type).unwrap_or_default()
    }

    fn read_overrides(&self) -> BTreeMap<String, String> {
        let Some(path) = &self.overrides_path else {
            return BTreeMap::new();
        };
        let Ok(contents) = fs::read(path) else {
            return BTreeMap::new();
        };
        serde_json::from_slice(&contents).unwrap_or_default()
    }

    fn write_overrides(&self, overrides: &BTreeMap<String, String>) -> io::Result<()> {
        let Some(path) = &self.overrides_path else {
            return Err(io::Error::other("the config directory cannot be determined"));
        };
        let contents = serde_json::to_vec_pretty(overrides)?;
        write_private_file(path, &contents)
    }
}

fn lock_overrides() -> std::sync::MutexGuard<'static, ()> {
    match OVERRIDES_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    use std::io::Write;
    if let Some(directory) = path.parent() {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)?;
    }
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(contents)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn environment_with_viewer(root: &Path) -> Environment {
        let applications = root.join("data").join("applications");
        fs::create_dir_all(&applications).unwrap();
        fs::write(applications.join("viewer.desktop"), "[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer %f\n")
            .unwrap();
        Environment {
            data_directories: vec![root.join("data")],
            overrides_path: Some(root.join("config").join("canopy").join("open-with.json")),
            ..Environment::default()
        }
    }

    #[test]
    fn overrides_round_trip_through_the_config_file() {
        let root = tempfile::tempdir().unwrap();
        let environment = environment_with_viewer(root.path());
        let image: PathBuf = root.path().join("image.png");
        fs::write(&image, "x").unwrap();
        let image = image.to_str().unwrap();

        environment.set_default(image, "viewer.desktop").unwrap();
        assert_eq!(environment.override_for("image/png"), "viewer.desktop");

        environment.clear_default(image).unwrap();
        assert_eq!(environment.override_for("image/png"), "");
    }

    #[test]
    fn set_default_refuses_an_unknown_application() {
        let root = tempfile::tempdir().unwrap();
        let environment = Environment {
            data_directories: vec![root.path().join("data")],
            overrides_path: Some(root.path().join("open-with.json")),
            ..Environment::default()
        };
        let image = root.path().join("image.png");
        fs::write(&image, "x").unwrap();

        assert!(environment.set_default(image.to_str().unwrap(), "missing.desktop").is_err());
    }
}
