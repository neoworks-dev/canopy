//! Splits the text of a previewed file into coloured runs.
//!
//! syntect (with the extra syntaxes from `two-face`) owns the languages: its
//! grammars decide what a token is, so nothing here lists a syntax. A token
//! carries a short class name rather than a colour. The class names are the
//! ones `ui/lib/highlightColors.ts` colours (`hl-kn`, `hl-s2`, `hl-c1`, ...),
//! so a preview is coloured like the rest of the window whatever theme it is in.
//! The scope-to-class table lives in `classes.rs`.

mod classes;

use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use syntect::parsing::{ParseState, ScopeStack, ScopeStackOp, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use classes::ScopeClassifier;

/// Guards the short class names against everything else a stylesheet holds.
pub const CLASS_PREFIX: &str = "hl-";

/// One run of text drawn in one colour. `class` is the CSS class that colour
/// belongs to, empty for text that takes the pane's own colour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub class: String,
    pub text: String,
}

/// Splits `source` into coloured runs, taking the language from the file name.
/// It returns nothing for a file type no grammar covers, which leaves the
/// caller with the plain text it already had.
pub fn tokens(file_name: &str, source: &str) -> Option<Vec<Token>> {
    let syntax_set = syntax_set();
    let syntax = syntax_for(syntax_set, file_name)?;
    let tokens = tokenise(syntax_set, syntax, source)?;
    if tokens.is_empty() {
        return None;
    }
    Some(tokens)
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

/// The grammar for a file name: the whole name first (`Makefile`), then its
/// extension. Plain text has nothing to colour.
fn syntax_for<'a>(syntax_set: &'a SyntaxSet, file_name: &str) -> Option<&'a SyntaxReference> {
    let by_name = syntax_set.find_syntax_by_extension(file_name);
    let found = match by_name {
        Some(syntax) => Some(syntax),
        None => extension_of(file_name).and_then(|extension| syntax_set.find_syntax_by_extension(extension)),
    };
    found.filter(|syntax| syntax.name != "Plain Text")
}

fn extension_of(file_name: &str) -> Option<&str> {
    Path::new(file_name).extension().and_then(|extension| extension.to_str())
}

fn tokenise(syntax_set: &SyntaxSet, syntax: &SyntaxReference, source: &str) -> Option<Vec<Token>> {
    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut classifier = ScopeClassifier::default();
    let mut tokens: Vec<Token> = Vec::new();
    for line in LinesWithEndings::from(source) {
        let operations = parse_state.parse_line(line, syntax_set).ok()?;
        let line_runs = line_runs(line, &operations, &mut scope_stack, &mut classifier)?;
        for (class, text) in line_runs {
            push_coalesced(&mut tokens, class, text);
        }
    }
    Some(tokens)
}

/// Cuts one line at every scope change and names the class each piece takes.
fn line_runs<'a>(
    line: &'a str,
    operations: &[(usize, ScopeStackOp)],
    scope_stack: &mut ScopeStack,
    classifier: &mut ScopeClassifier,
) -> Option<Vec<(&'static str, &'a str)>> {
    let mut runs = Vec::new();
    let mut position = 0;
    for (index, operation) in operations {
        if *index > position {
            runs.push((classifier.class_of(scope_stack), &line[position..*index]));
            position = *index;
        }
        scope_stack.apply(operation).ok()?;
    }
    if position < line.len() {
        runs.push((classifier.class_of(scope_stack), &line[position..]));
    }
    Some(runs)
}

/// Adjacent runs of one class become one token.
fn push_coalesced(tokens: &mut Vec<Token>, class: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    let prefixed = prefixed(class);
    if let Some(last) = tokens.last_mut()
        && last.class == prefixed
    {
        last.text.push_str(text);
        return;
    }
    tokens.push(Token { class: prefixed, text: text.to_string() });
}

fn prefixed(class: &str) -> String {
    if class.is_empty() {
        return String::new();
    }
    format!("{CLASS_PREFIX}{class}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spelled(tokens: &[Token]) -> String {
        tokens.iter().map(|token| token.text.as_str()).collect()
    }

    #[test]
    fn a_source_file_is_split_and_spells_the_source_back() {
        let found = tokens("main.go", "package main\n").expect("go is highlighted");
        assert_eq!(spelled(&found), "package main\n");
        assert!(found.iter().any(|token| token.class.starts_with("hl-k")), "{found:?}");
    }

    #[test]
    fn the_language_comes_from_the_file_name() {
        for file_name in ["script.py", "styles.css", "Makefile", "notes.md"] {
            assert!(tokens(file_name, "a = 1\n").is_some(), "{file_name} should be highlighted");
        }
    }

    #[test]
    fn an_unknown_file_type_is_left_alone() {
        assert!(tokens("archive.qqq", "some words\n").is_none());
    }

    #[test]
    fn plain_text_is_left_alone() {
        assert!(tokens("notes.txt", "some words\n").is_none());
    }

    #[test]
    fn every_class_is_prefixed_or_empty() {
        let found = tokens("script.py", "def f(x):\n\treturn \"a\" + 1  # note\n").unwrap();
        for token in found {
            assert!(token.class.is_empty() || token.class.starts_with(CLASS_PREFIX), "{}", token.class);
        }
    }

    #[test]
    fn strings_comments_and_numbers_take_their_classes() {
        let found = tokens("script.py", "x = \"a\" + 1  # note\n").unwrap();
        let class_of = |text: &str| found.iter().find(|token| token.text.contains(text)).unwrap().class.clone();
        assert_eq!(class_of("\"a\""), "hl-s2");
        assert_eq!(class_of("1"), "hl-mi");
        assert!(class_of("# note").starts_with("hl-c"));
    }

    #[test]
    fn source_without_a_trailing_newline_is_kept_whole() {
        let found = tokens("main.rs", "fn main() {}").unwrap();
        assert_eq!(spelled(&found), "fn main() {}");
    }
}
