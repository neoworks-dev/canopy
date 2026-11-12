use std::collections::HashMap;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::sync::OnceLock;

const USERS_FILE: &str = "/etc/passwd";
const GROUPS_FILE: &str = "/etc/group";

static USER_NAMES: OnceLock<HashMap<u32, String>> = OnceLock::new();
static GROUP_NAMES: OnceLock<HashMap<u32, String>> = OnceLock::new();

/// `user:group` for an entry, by name where the local account files list one
/// and by number otherwise. The files are read once per process.
pub fn owner_string(metadata: &Metadata) -> String {
    let user_names = USER_NAMES.get_or_init(|| read_id_names(USERS_FILE));
    let group_names = GROUP_NAMES.get_or_init(|| read_id_names(GROUPS_FILE));
    format!("{}:{}", name_or_number(user_names, metadata.uid()), name_or_number(group_names, metadata.gid()))
}

fn name_or_number(names: &HashMap<u32, String>, id: u32) -> String {
    match names.get(&id) {
        Some(name) => name.clone(),
        None => id.to_string(),
    }
}

fn read_id_names(path: &str) -> HashMap<u32, String> {
    match std::fs::read_to_string(path) {
        Ok(contents) => parse_id_names(&contents),
        Err(_) => HashMap::new(),
    }
}

/// The first line naming an id wins, as it does for the C library's lookups.
fn parse_id_names(contents: &str) -> HashMap<u32, String> {
    let mut names = HashMap::new();
    for (id, name) in contents.lines().filter_map(parse_id_line) {
        names.entry(id).or_insert(name);
    }
    names
}

/// `/etc/passwd` and `/etc/group` both hold the name in the first field and the
/// numeric id in the third.
fn parse_id_line(line: &str) -> Option<(u32, String)> {
    let mut fields = line.split(':');
    let name = fields.next()?;
    let id = fields.nth(1)?.parse().ok()?;
    Some((id, name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_lines_map_ids_to_names() {
        let names = parse_id_names("root:x:0:0:root:/root:/bin/sh\n# comment\nmoritz:x:1000:1000::/home/moritz:/bin/fish\nshadow:x:1000:\n");

        assert_eq!(names[&0], "root");
        assert_eq!(names[&1000], "moritz");
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn an_unknown_id_falls_back_to_its_number() {
        assert_eq!(name_or_number(&HashMap::new(), 4242), "4242");
    }
}
