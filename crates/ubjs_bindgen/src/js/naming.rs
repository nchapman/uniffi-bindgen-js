// ---------------------------------------------------------------------------
// Identifier helpers
// ---------------------------------------------------------------------------

pub(super) fn camel_case(input: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = false;
    for ch in input.chars() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            out.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

pub(super) fn is_js_reserved(word: &str) -> bool {
    matches!(
        word,
        // ECMAScript reserved words
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "undefined"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            // Strict-mode and TypeScript contextual keywords
            | "async"
            | "await"
            | "implements"
            | "interface"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "type"
    )
}

pub(super) fn safe_js_identifier(input: &str) -> String {
    if is_js_reserved(input) {
        format!("{input}_")
    } else {
        input.to_string()
    }
}

pub(super) fn pascal_case(input: &str) -> String {
    let mut out = String::new();
    for part in input.split(['_', '-']) {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        "UniffiBindings".to_string()
    } else {
        out
    }
}

pub(super) fn snake_case(input: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            let prev_upper = chars[i - 1].is_ascii_uppercase();
            let next_lower = chars.get(i + 1).is_some_and(|c| c.is_ascii_lowercase());
            // Insert underscore before: a lone uppercase after lowercase, OR
            // the last letter of an acronym run when the next char is lowercase.
            if !prev_upper || next_lower {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}
