//! Placeholder rendering for prompt `.txt` bodies.

/// Replace `{{key}}` placeholders. Unknown keys are left unchanged.
pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        let needle = format!("{{{{{key}}}}}");
        out = out.replace(&needle, value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_known_placeholders() {
        let out = render(
            "Hi {{lang}} — {{name}}",
            &[("lang", "vi"), ("name", "Nhan")],
        );
        assert_eq!(out, "Hi vi — Nhan");
    }

    #[test]
    fn leaves_unknown_placeholders() {
        let out = render("{{a}} {{b}}", &[("a", "1")]);
        assert_eq!(out, "1 {{b}}");
    }
}
