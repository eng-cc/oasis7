//! Ordinary signed native task client. No local world access or model service.
fn main() {
    if let Err(error) = oasis7_game_client::commands::run(std::env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
