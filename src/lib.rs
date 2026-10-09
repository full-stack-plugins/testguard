pub fn supports_profile(tool: &str, version: &str, protocol: &str) -> bool {
    (tool, version, protocol) == ("cargo", "1.99.0", "libtest-pretty-v1")
}
