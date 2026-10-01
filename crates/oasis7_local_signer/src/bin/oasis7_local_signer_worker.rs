fn main() {
    let exit_code = oasis7_local_signer::worker::run_worker();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}
