pub fn show_version() {
    println!("{}", version_display());
}

pub(crate) fn package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub(crate) fn build_commit() -> &'static str {
    option_env!("SKM_BUILD_COMMIT").unwrap_or("unknown")
}

pub(crate) fn build_channel() -> &'static str {
    option_env!("SKM_BUILD_CHANNEL").unwrap_or("local")
}

pub fn version_display() -> String {
    format!(
        "skm {} ({} - {})",
        package_version(),
        build_channel(),
        build_commit()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_display_uses_embedded_build_metadata() {
        let display = version_display();
        assert!(display.starts_with("skm "));
        assert!(display.contains(env!("CARGO_PKG_VERSION")));
    }
}
