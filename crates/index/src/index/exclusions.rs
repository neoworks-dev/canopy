/// Trees that are large, machine-generated, and never what a user searches for.
/// Skipping them is the difference between an index of tens of thousands of
/// entries and one of millions.
const EXCLUDED_DIRECTORY_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".cache",
    ".npm",
    ".bun",
    ".gradle",
    ".venv",
    "venv",
    "node_modules",
    "__pycache__",
    "target",
    "vendor",
    "dist",
    "build",
    ".next",
    ".svelte-kit",
    ".terraform",
];

/// Kernel and runtime filesystems. Walking them is pointless and can block on
/// device nodes.
const EXCLUDED_PATH_PREFIXES: &[&str] = &["/proc", "/sys", "/dev", "/run", "/snap", "/var/lib/docker"];

/// Decides which directories a crawl refuses to descend into.
#[derive(Debug, Clone, Default)]
pub struct Exclusions;

impl Exclusions {
    /// Whether a directory should be left unindexed. Callers must not apply it
    /// to the root of a scan; a user who explicitly adds
    /// `~/projects/node_modules` as a root means it.
    pub fn skips_directory(&self, path: &str, name: &str) -> bool {
        if EXCLUDED_DIRECTORY_NAMES.contains(&name) {
            return true;
        }
        has_excluded_prefix(path)
    }
}

fn has_excluded_prefix(path: &str) -> bool {
    EXCLUDED_PATH_PREFIXES.iter().any(|prefix| {
        path == *prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_generated_directories_by_name() {
        assert!(Exclusions.skips_directory("/home/user/app/node_modules", "node_modules"));
        assert!(!Exclusions.skips_directory("/home/user/app/src", "src"));
    }

    #[test]
    fn skips_runtime_filesystems_by_prefix() {
        assert!(Exclusions.skips_directory("/proc", "proc"));
        assert!(Exclusions.skips_directory("/sys/devices", "devices"));
        assert!(!Exclusions.skips_directory("/procedures", "procedures"));
    }
}
