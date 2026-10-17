//! Offline shadow packing only. Never changes an active endpoint or selects a generation.
use oasis7_distfs::controlled_authority::formal::build_formal_storage_shadow;
use std::path::Path;
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: formal_storage_shadow SOURCE_ENDPOINT NEW_SHADOW_DIRECTORY");
        std::process::exit(2);
    }
    match build_formal_storage_shadow(Path::new(&args[0]), Path::new(&args[1])) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(error) => {
            eprintln!("shadow failed; source retained; output may be incomplete: {error:?}");
            std::process::exit(1);
        }
    }
}
