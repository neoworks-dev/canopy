/// The last element of a slash-separated path: trailing slashes are ignored, an empty path is ".", and a path of
/// only slashes is "/".
pub(crate) fn base_name(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rfind('/') {
        Some(separator) => trimmed[separator + 1..].to_string(),
        None => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_name_follows_go_rules() {
        assert_eq!(base_name("/home/user/notes.txt"), "notes.txt");
        assert_eq!(base_name("/home/user/"), "user");
        assert_eq!(base_name("/"), "/");
        assert_eq!(base_name(""), ".");
        assert_eq!(base_name("plain"), "plain");
    }
}
