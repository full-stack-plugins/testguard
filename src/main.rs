fn main() {
    std::process::exit(testguard::cli::run(std::env::args().skip(1).collect()));
}
