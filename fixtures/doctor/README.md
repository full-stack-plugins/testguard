# Read-only discovery fixture environments

`tests/doctor.rs` creates isolated absolute PATH directories separately for Cargo and Maven. Each contains a supplied executable script whose only behavior would create an observable marker and exit 99. Discovery must report it present without executing it. The other supported tool is absent. Removing execute permission makes both unavailable. The marker must remain absent. These are deliberately synthetic installation fixtures, not native execution evidence, and require no network or external writes by doctor.
