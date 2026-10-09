fn main() {
    std::process::exit(testguard::cli::run_os(std::env::args_os().skip(1)));
}
