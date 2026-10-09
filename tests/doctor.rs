use testguard::doctor::discover;
#[test]
fn installed_and_absent_tools_detected_without_executing_them() {
    use std::os::unix::fs::PermissionsExt;
    for name in ["cargo", "mvn"] {
        let dir =
            std::env::temp_dir().join(format!("testguard-doctor-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let executable = dir.join(name);
        let marker = dir.join("SHOULD_NOT_EXIST");
        std::fs::write(
            &executable,
            format!("#!/bin/sh\ntouch {}\nexit 99\n", marker.display()),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let found = discover(dir.as_os_str());
        for diagnostic in found {
            assert_eq!(diagnostic.available, diagnostic.name == name);
        }
        assert!(!marker.exists());
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(discover(dir.as_os_str()).iter().all(|t| !t.available));
    }
}
