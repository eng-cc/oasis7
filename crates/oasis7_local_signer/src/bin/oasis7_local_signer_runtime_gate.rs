fn main() {
    if let Err(error) = oasis7_local_signer::runtime_gate::run_from_environment() {
        eprintln!("oasis7-local-signer-runtime-gate: {error}");
        std::process::exit(9);
    }
}
