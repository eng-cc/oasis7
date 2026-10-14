fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(oasis7_context::cli::run(&args, "oasis7_context"));
}
