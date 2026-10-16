//! Maps TextMate scopes (`string.quoted.double.python`) to the short class
//! names `frontend/src/highlight.css` colours. A class the stylesheet does not
//! colour is left empty, so those runs take the pane's own colour.

use std::collections::HashMap;

use syntect::parsing::{Scope, ScopeStack};

/// First match wins, so the longer prefixes come before the shorter ones they
/// extend. A prefix matches whole dot-separated atoms.
const SCOPE_CLASSES: &[(&str, &str)] = &[
    ("comment.block.documentation", "cs"),
    ("comment.line", "c1"),
    ("comment.block", "cm"),
    ("comment", "c"),
    ("meta.preprocessor", "cp"),
    ("keyword.control.import.include", "cp"),
    ("keyword.control.import.define", "cp"),
    ("keyword.control.directive", "cp"),
    ("punctuation.definition.directive", "cp"),
    ("string.quoted.other.lt-gt.include", "cpf"),
    ("string.regexp", "sr"),
    ("string.interpolated", "si"),
    ("string.quoted.docstring", "sd"),
    ("string.quoted.triple", "sd"),
    ("string.quoted.single", "s1"),
    ("string.quoted.double", "s2"),
    ("string.quoted.backtick", "sb"),
    ("string", "s"),
    ("meta.interpolation", "si"),
    ("punctuation.section.interpolation", "si"),
    ("constant.character.escape", "se"),
    ("constant.character", "sc"),
    ("constant.numeric.integer.hexadecimal", "mh"),
    ("constant.numeric.hex", "mh"),
    ("constant.numeric.octal", "mo"),
    ("constant.numeric.binary", "mb"),
    ("constant.numeric.float", "mf"),
    ("constant.numeric.integer", "mi"),
    ("constant.numeric", "m"),
    ("constant.language", "kc"),
    ("constant.other.placeholder", "si"),
    ("constant", "no"),
    ("variable.language", "bp"),
    ("variable.function", "nf"),
    ("entity.name.function.decorator", "nd"),
    ("entity.name.function", "nf"),
    ("entity.name.class", "nc"),
    ("entity.name.struct", "nc"),
    ("entity.name.enum", "nc"),
    ("entity.name.union", "nc"),
    ("entity.name.trait", "nc"),
    ("entity.name.interface", "nc"),
    ("entity.name.type", "nc"),
    ("entity.other.inherited-class", "nc"),
    ("entity.name.namespace", "nn"),
    ("entity.name.module", "nn"),
    ("entity.name.exception", "ne"),
    ("entity.name.tag", "nt"),
    ("entity.name.label", "nl"),
    ("entity.name.constant", "no"),
    ("support.function.magic", "fm"),
    ("support.function", "nb"),
    ("support.class", "nb"),
    ("support.type.exception", "ne"),
    ("support.type", "nb"),
    ("support.constant", "no"),
    ("support.variable", "bp"),
    ("meta.annotation", "nd"),
    ("meta.decorator", "nd"),
    ("storage.type.annotation", "nd"),
    ("punctuation.definition.annotation", "nd"),
    ("keyword.operator.word", "ow"),
    ("keyword.operator.logical.python", "ow"),
    ("keyword.operator", "o"),
    ("keyword.control.import", "kn"),
    ("keyword.control", "k"),
    ("keyword.declaration", "kd"),
    ("keyword.other", "k"),
    ("keyword", "k"),
    ("storage.type.function", "kd"),
    ("storage.type.class", "kd"),
    ("storage.type.struct", "kd"),
    ("storage.type.enum", "kd"),
    ("storage.type.union", "kd"),
    ("storage.type.trait", "kd"),
    ("storage.type.interface", "kd"),
    ("storage.type.impl", "kd"),
    ("storage.type.type", "kd"),
    ("storage.type.var", "kd"),
    ("storage.type.const", "kd"),
    ("storage.type.let", "kd"),
    ("storage.type.namespace", "kn"),
    ("storage.type.module", "kn"),
    ("storage.type", "kt"),
    ("storage.modifier", "kd"),
    ("storage", "kd"),
    ("invalid", "err"),
];

/// What one scope contributes to the class of the text under it.
#[derive(Clone, Copy)]
struct ScopeClass {
    class: Option<&'static str>,
    is_punctuation: bool,
}

/// Decides the class of a run from the scopes on the stack, remembering the
/// answer per scope because a file uses few distinct scopes many times over.
#[derive(Default)]
pub(super) struct ScopeClassifier {
    known: HashMap<Scope, ScopeClass>,
}

impl ScopeClassifier {
    /// The innermost scope that has a class decides. Punctuation inside a
    /// string or comment belongs to it; punctuation on its own is `p`.
    pub(super) fn class_of(&mut self, scope_stack: &ScopeStack) -> &'static str {
        let mut has_punctuation = false;
        for scope in scope_stack.as_slice().iter().rev() {
            let scope_class = self.scope_class(*scope);
            if let Some(class) = scope_class.class {
                return class;
            }
            has_punctuation = has_punctuation || scope_class.is_punctuation;
        }
        if has_punctuation {
            return "p";
        }
        ""
    }

    fn scope_class(&mut self, scope: Scope) -> ScopeClass {
        if let Some(known) = self.known.get(&scope) {
            return *known;
        }
        let name = scope.to_string();
        let scope_class = ScopeClass { class: class_for_name(&name), is_punctuation: is_punctuation(&name) };
        self.known.insert(scope, scope_class);
        scope_class
    }
}

fn class_for_name(name: &str) -> Option<&'static str> {
    for (prefix, class) in SCOPE_CLASSES {
        if has_scope_prefix(name, prefix) {
            return Some(class);
        }
    }
    None
}

fn has_scope_prefix(name: &str, prefix: &str) -> bool {
    match name.strip_prefix(prefix) {
        Some(rest) => rest.is_empty() || rest.starts_with('.'),
        None => false,
    }
}

fn is_punctuation(name: &str) -> bool {
    has_scope_prefix(name, "punctuation")
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn prefixes_match_whole_atoms() {
        assert_eq!(class_for_name("string.quoted.double.python"), Some("s2"));
        assert_eq!(class_for_name("stringy.thing"), None);
        assert_eq!(class_for_name("constant.numeric.integer.decimal.python"), Some("mi"));
    }

    #[test]
    fn scopes_without_a_colour_have_no_class() {
        assert_eq!(class_for_name("source.python"), None);
        assert_eq!(class_for_name("variable.parameter.python"), None);
    }

    #[test]
    fn punctuation_inside_a_string_takes_the_strings_class() {
        let mut classifier = ScopeClassifier::default();
        let stack = ScopeStack::from_str(
            "source.python string.quoted.double.python punctuation.definition.string.begin.python",
        )
        .unwrap();
        assert_eq!(classifier.class_of(&stack), "s2");
    }

    #[test]
    fn lone_punctuation_is_p() {
        let mut classifier = ScopeClassifier::default();
        let stack = ScopeStack::from_str("source.python meta.group punctuation.section.group.begin.python").unwrap();
        assert_eq!(classifier.class_of(&stack), "p");
    }
}
