use serde::Serialize;
#[derive(schemars::JsonSchema, Debug, Serialize)]
pub struct ToolDiagnostic {
    pub name: String,
    pub available: bool,
    pub path: Option<String>,
}
pub fn discover(path: &std::ffi::OsStr) -> Vec<ToolDiagnostic> {
    ["cargo", "mvn"]
        .into_iter()
        .map(|name| {
            let found = std::env::split_paths(path)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(name))
                .find(|p| {
                    p.metadata().is_ok_and(|m| {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            m.is_file() && m.permissions().mode() & 0o111 != 0
                        }
                        #[cfg(not(unix))]
                        {
                            m.is_file()
                        }
                    })
                });
            ToolDiagnostic {
                name: name.into(),
                available: found.is_some(),
                path: found.map(|p| p.to_string_lossy().into_owned()),
            }
        })
        .collect()
}
