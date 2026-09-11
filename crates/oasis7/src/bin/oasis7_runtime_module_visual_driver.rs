//! Append typed commands for the opt-in live runtime module visual driver.
//!
//! The live server consumes these commands only when
//! `OASIS7_RUNTIME_MODULE_VISUAL_DRIVER` points at the same JSONL file.

use oasis7::simulator::ModuleVisualAnchor;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!(
        "usage: oasis7_runtime_module_visual_driver --driver-file PATH \
         upsert --entity-id ID --module-id ID --anchor absolute:X:Y:Z\n\n\
         usage: oasis7_runtime_module_visual_driver --driver-file PATH remove --entity-id ID"
    );
    std::process::exit(2);
}

fn take_flag(args: &mut Vec<String>, name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    if index + 1 >= args.len() {
        usage();
    }
    let value = args.remove(index + 1);
    args.remove(index);
    Some(value)
}

fn parse_anchor(raw: &str) -> ModuleVisualAnchor {
    let mut fields = raw.split(':');
    if fields.next() != Some("absolute") {
        usage();
    }
    let x_cm = fields.next().and_then(|value| value.parse().ok());
    let y_cm = fields.next().and_then(|value| value.parse().ok());
    let z_cm = fields.next().and_then(|value| value.parse().ok());
    if fields.next().is_some() || x_cm.is_none() || y_cm.is_none() || z_cm.is_none() {
        usage();
    }
    ModuleVisualAnchor::Absolute {
        pos: oasis7::geometry::GeoPos::new(x_cm.unwrap(), y_cm.unwrap(), z_cm.unwrap()),
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let Some(driver_file) = take_flag(&mut args, "--driver-file") else {
        usage();
    };
    if args.is_empty() {
        usage();
    }
    let operation = args.remove(0);
    let entity_id = take_flag(&mut args, "--entity-id");
    let command = match operation.as_str() {
        "upsert" => {
            let Some(entity_id) = entity_id else { usage() };
            let Some(module_id) = take_flag(&mut args, "--module-id") else {
                usage()
            };
            let Some(anchor) = take_flag(&mut args, "--anchor") else {
                usage()
            };
            if !args.is_empty() {
                usage();
            }
            json!({
                "operation": "upsert",
                "entity_id": entity_id,
                "module_id": module_id,
                "anchor": parse_anchor(&anchor),
            })
        }
        "remove" => {
            let Some(entity_id) = entity_id else { usage() };
            if !args.is_empty() {
                usage();
            }
            json!({ "operation": "remove", "entity_id": entity_id })
        }
        _ => usage(),
    };

    let path = PathBuf::from(&driver_file);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!("create driver directory: {error}");
            std::process::exit(1);
        }
    }
    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("open driver file {}: {error}", path.display());
            std::process::exit(1);
        }
    };
    if let Err(error) = serde_json::to_writer(&mut file, &command)
        .map_err(std::io::Error::other)
        .and_then(|_| file.write_all(b"\n"))
    {
        eprintln!("write driver command: {error}");
        std::process::exit(1);
    }
    println!("driver_file={} command={}", path.display(), command);
}
