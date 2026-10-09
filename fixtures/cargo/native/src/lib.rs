#[cfg(test)]
mod cases {
    #[test] fn pass() {assert_eq!(2 + 2, 4);}
    #[test] fn fail() {assert_eq!(2 + 2, 5);}
    #[test] #[ignore] fn ignored() {panic!("ignored test must not execute");}
    #[test] fn timeout() {std::thread::sleep(std::time::Duration::from_secs(30));}
    #[cfg(feature="extra")]
    #[test] fn feature_case() {assert!(true);}
}
