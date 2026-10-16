use crate::file_uri;

/// Splits an Exec line on unquoted spaces, the only quoting rule the desktop
/// entry spec needs here.
pub(crate) fn split_exec(exec_line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in exec_line.chars() {
        if character == '"' {
            quoted = !quoted;
        } else if character == ' ' && !quoted {
            flush(&mut fields, &mut current);
        } else {
            current.push(character);
        }
    }
    flush(&mut fields, &mut current);
    fields
}

fn flush(fields: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    fields.push(std::mem::take(current));
}

/// Expands the desktop entry field codes. An entry that names no placeholder
/// still gets the path appended, which is what launchers do.
pub(crate) fn build_command(exec_line: &str, path: &str) -> Vec<String> {
    let mut command = Vec::new();
    let mut substituted = false;
    for field in split_exec(exec_line) {
        if let Some(argument) = path_argument(&field, path) {
            command.push(argument);
            substituted = true;
        } else if !field.starts_with('%') {
            command.push(field);
        }
    }
    if !substituted {
        command.push(path.to_string());
    }
    command
}

fn path_argument(field: &str, path: &str) -> Option<String> {
    match field {
        "%f" | "%F" => Some(path.to_string()),
        "%u" | "%U" => Some(file_uri::from_path(path)),
        _ => None,
    }
}

/// The command line with every field code dropped, which starts the
/// application with nothing to open.
pub(crate) fn without_field_codes(fields: Vec<String>) -> Vec<String> {
    fields.into_iter().filter(|field| !field.starts_with('%')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_exec_keeps_quoted_arguments() {
        let fields = split_exec(r#"/usr/bin/app --flag "one argument" %f"#);

        assert_eq!(fields, ["/usr/bin/app", "--flag", "one argument", "%f"]);
    }

    #[test]
    fn build_command_substitutes_the_file_code() {
        let command = build_command("viewer %f --quiet", "/home/user/a b.png");

        assert_eq!(command, ["viewer", "/home/user/a b.png", "--quiet"]);
    }

    #[test]
    fn build_command_passes_a_uri_for_the_uri_code() {
        let command = build_command("browser %u", "/home/user/a b.html");

        assert_eq!(command, ["browser", "file:///home/user/a%20b.html"]);
    }

    #[test]
    fn build_command_drops_unused_field_codes_and_appends_the_path() {
        let command = build_command("editor %i %c", "/home/user/note.txt");

        assert_eq!(command, ["editor", "/home/user/note.txt"]);
    }
}
