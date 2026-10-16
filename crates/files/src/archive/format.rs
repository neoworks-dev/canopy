/// How an archive is packed, worked out from the name rather than the
/// contents: a file manager labels an entry before it has opened it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    Tar,
    TarGzip,
    TarBzip2,
    Unknown,
}

/// The longer suffixes come first, so a `.tar.gz` is not read as a `.gz`.
const SUFFIX_FORMATS: [(&str, Format); 6] = [
    (".tar.gz", Format::TarGzip),
    (".tgz", Format::TarGzip),
    (".tar.bz2", Format::TarBzip2),
    (".tbz2", Format::TarBzip2),
    (".tar", Format::Tar),
    (".zip", Format::Zip),
];

/// How a file is packed, or `Format::Unknown` when the name says it is not an
/// archive at all.
pub fn format_of(name: &str) -> Format {
    let lowered = name.to_lowercase();
    for (suffix, format) in SUFFIX_FORMATS {
        if lowered.ends_with(suffix) {
            return format;
        }
    }
    Format::Unknown
}

/// Whether the name says the file is an archive the explorer can open.
pub fn is_archive(name: &str) -> bool {
    format_of(name) != Format::Unknown
}

/// An archive's name without the suffix that says how it is packed, which is
/// what a folder unpacked from it is called.
pub fn base_name(name: &str) -> String {
    let lowered = name.to_lowercase();
    for (suffix, _) in SUFFIX_FORMATS {
        if lowered.ends_with(suffix) {
            return name[..name.len() - suffix.len()].to_string();
        }
    }
    name.to_string()
}

/// What an archive holding the given entry should be called.
pub fn suggested_name(source_name: &str) -> String {
    format!("{source_name}.zip")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_is_read_from_the_name() {
        let cases = [
            ("photos.zip", Format::Zip),
            ("PHOTOS.ZIP", Format::Zip),
            ("backup.tar", Format::Tar),
            ("backup.tar.gz", Format::TarGzip),
            ("backup.tgz", Format::TarGzip),
            ("backup.tar.bz2", Format::TarBzip2),
            ("notes.txt", Format::Unknown),
            ("archive.zip.part", Format::Unknown),
        ];
        for (name, wanted) in cases {
            assert_eq!(format_of(name), wanted, "{name}");
        }
    }

    #[test]
    fn the_base_name_drops_the_whole_suffix() {
        assert_eq!(base_name("photos.tar.gz"), "photos");
        assert_eq!(base_name("Photos.ZIP"), "Photos");
        assert_eq!(base_name("notes.txt"), "notes.txt");
    }
}
