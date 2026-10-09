pub fn supports_profile(tool: &str, version: &str, protocol: &str) -> bool {
    matches!(
        (tool, version, protocol),
        ("cargo", "1.99.0", "libtest-pretty-v1")
            | ("maven-surefire", "3.5.2", "junit-xml-v1")
            | ("gradle", "8.14.3", "junit-xml-v1")
    )
}

pub mod adapters;
pub mod cli;
pub mod coverage;
pub mod doctor;
pub mod obligation;
pub mod plan;
pub mod policy;
pub mod report;
pub mod runner;
