pub(super) const VARIABLES: [&str; 4] = [
    "long_term_memory",
    "short_term_memory",
    "input_context",
    "transcript",
];

/// Validate the deliberately small placeholder language. Values will be
/// rendered in one pass and must never be parsed as another template.
pub(super) fn validate(template: &str) -> Result<(), String> {
    if template.trim().is_empty() || template.chars().count() > 32_000 {
        return Err("A prompt must contain between 1 and 32000 characters".into());
    }
    let mut rest = template;
    loop {
        let open = rest.find("{{");
        let close = rest.find("}}");
        match (open, close) {
            (None, None) => return Ok(()),
            (Some(start), Some(end)) if start < end => {
                if !VARIABLES.contains(&rest[start + 2..end].trim()) {
                    return Err("Unknown prompt variable or unsupported template expression".into());
                }
                rest = &rest[end + 2..];
            }
            _ => return Err("Unclosed or unmatched prompt variable".into()),
        }
    }
}

/// Templates refer to named fields in the user-message data envelope. Captured
/// content is never promoted into the system role or evaluated as a template.
pub(super) fn render(template: &str) -> Result<String, String> {
    validate(template)?;
    let mut rendered = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let end = rest.find("}}").ok_or("Unclosed prompt variable")?;
        rendered.push_str(&rest[..start]);
        rendered.push_str("[user data field: ");
        rendered.push_str(rest[start + 2..end].trim());
        rendered.push(']');
        rest = &rest[end + 2..];
    }
    rendered.push_str(rest);
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_templates_and_helpers() {
        for prompt in [
            "",
            "{{unknown}}",
            "{{transcript",
            "text }}",
            "{{#if transcript}}",
            "{{{transcript}}}",
        ] {
            assert!(validate(prompt).is_err(), "{prompt}");
        }
        assert!(validate("Rewrite {{ transcript }} using {{long_term_memory}}.").is_ok());
        assert!(validate("Rewrite carefully.").is_ok());
    }
}
