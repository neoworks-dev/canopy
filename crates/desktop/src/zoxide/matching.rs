/// Applies zoxide's own rule, so a query that works in the shell works here:
/// the keywords have to appear in the path in order, and the last one has to
/// appear in the final path component. That last part is what makes "z dow"
/// land on Downloads rather than on the first thing inside it.
pub(super) fn matches_keywords(path: &str, keywords: &[String]) -> bool {
    let Some((last_keyword, leading_keywords)) = keywords.split_last() else {
        return true;
    };
    let lowered = path.to_lowercase();
    let Some(cursor) = consume_leading_keywords(&lowered, leading_keywords) else {
        return false;
    };
    last_keyword_is_in_the_name(&lowered, last_keyword, cursor)
}

/// Walks every keyword but the last, reporting how far along the path they took
/// us.
fn consume_leading_keywords(lowered: &str, keywords: &[String]) -> Option<usize> {
    let mut cursor = 0;
    for keyword in keywords {
        let offset = lowered[cursor..].find(keyword.as_str())?;
        cursor += offset + keyword.len();
    }
    Some(cursor)
}

fn last_keyword_is_in_the_name(lowered: &str, keyword: &str, cursor: usize) -> bool {
    let Some(offset) = lowered[cursor..].find(keyword) else {
        return false;
    };
    match lowered.rfind('/') {
        Some(last_separator) => cursor + offset > last_separator,
        None => true,
    }
}

pub(super) fn split_keywords(text: &str) -> Vec<String> {
    text.to_lowercase().split_whitespace().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_zoxide_rules() {
        let path = "/home/moritz/Documents/file-explorer-neoworks";
        let cases = [
            ("", true),
            ("neo", true),
            ("NEO", true),
            ("doc neo", true),
            ("neo doc", false),
            ("moritz explorer", true),
            // The last keyword has to land in the final component, which is what
            // makes a query select a directory rather than its parent.
            ("documents", false),
            ("nothing", false),
        ];

        for (text, expected) in cases {
            assert_eq!(matches_keywords(path, &split_keywords(text)), expected, "{text:?}");
        }
    }

    #[test]
    fn consumes_each_keyword_in_turn() {
        assert!(!matches_keywords("/home/moritz/reports", &split_keywords("reports reports")));
        assert!(matches_keywords("/home/reports/reports", &split_keywords("reports reports")));
    }
}
