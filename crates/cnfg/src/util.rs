/// Leak a string into a `'static` lifetime. Used to build path literals at runtime.
pub fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Format a block of documentation for CLI help.
pub fn format_doc(doc: Option<&str>) -> Option<String> {
    doc.map(|d| {
        d.split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// Render a CLI flag with padding suitable for help output.
pub fn format_flag(flag: &str, takes_value: bool) -> String {
    if takes_value {
        format!("--{} <value>", flag)
    } else {
        format!("--{}", flag)
    }
}
