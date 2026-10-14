//! Ordinary signed native task client. No local world access or model service.
#[path = "oasis7_game_client/client.rs"]
mod client;
#[path = "oasis7_game_client/commands.rs"]
mod commands;
fn main() {
    if let Err(error) = commands::run(std::env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
