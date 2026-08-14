use oasis7_distfs::{BlobStore, HashAlgorithm, LocalCasStore};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
struct Options {
    root: PathBuf,
    manifest: PathBuf,
    built_dir: PathBuf,
}

fn usage() -> &'static str {
    "Usage: cargo run -p oasis7_distfs --bin hydrate_builtin_wasm -- \\
  --root <distfs-root> --manifest <module-hash-manifest> --built-dir <built-wasm-dir>"
}

fn parse_args() -> Result<Options, String> {
    let mut root: Option<PathBuf> = None;
    let mut manifest: Option<PathBuf> = None;
    let mut built_dir: Option<PathBuf> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --root".to_string());
                };
                root = Some(PathBuf::from(value));
            }
            "--manifest" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --manifest".to_string());
                };
                manifest = Some(PathBuf::from(value));
            }
            "--built-dir" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --built-dir".to_string());
                };
                built_dir = Some(PathBuf::from(value));
            }
            "-h" | "--help" => {
                return Err(usage().to_string());
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }

    let Some(root) = root else {
        return Err("--root is required".to_string());
    };
    let Some(manifest) = manifest else {
        return Err("--manifest is required".to_string());
    };
    let Some(built_dir) = built_dir else {
        return Err("--built-dir is required".to_string());
    };

    Ok(Options {
        root,
        manifest,
        built_dir,
    })
}

fn parse_manifest_hash_token(token: &str) -> Result<String, String> {
    let value = token
        .split_once('=')
        .map(|(_, hash)| hash)
        .unwrap_or(token)
        .trim();
    if value.is_empty() {
        return Err(format!("invalid manifest hash token: {token}"));
    }
    Ok(value.to_string())
}

fn parse_manifest_line(line: &str) -> Result<Option<(String, Vec<String>)>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(module_id) = parts.next() else {
        return Ok(None);
    };
    let mut hashes = Vec::new();
    for token in parts {
        hashes.push(parse_manifest_hash_token(token)?);
    }
    if hashes.is_empty() {
        return Err(format!("invalid manifest line (missing hash): {line}"));
    }

    Ok(Some((module_id.to_string(), hashes)))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn run() -> Result<(), String> {
    let options = parse_args()?;

    let manifest_content = fs::read_to_string(&options.manifest).map_err(|error| {
        format!(
            "failed to read manifest {}: {error}",
            options.manifest.display()
        )
    })?;

    let store = LocalCasStore::new_with_hash_algorithm(&options.root, HashAlgorithm::Sha256);
    let mut module_count = 0usize;
    let mut newly_written = 0usize;

    for line in manifest_content.lines() {
        let Some((module_id, expected_hashes)) = parse_manifest_line(line)? else {
            continue;
        };

        let built_path = options.built_dir.join(format!("{module_id}.wasm"));
        let bytes = fs::read(&built_path).map_err(|error| {
            format!(
                "failed to read built wasm for module {module_id} at {}: {error}",
                built_path.display()
            )
        })?;

        let actual_hash = sha256_hex(&bytes);
        if !expected_hashes
            .iter()
            .any(|expected| expected == &actual_hash)
        {
            return Err(format!(
                "built wasm hash not listed in manifest module_id={} built_hash={} expected_hashes=[{}]",
                module_id,
                actual_hash,
                expected_hashes.join(",")
            ));
        }

        let existed = store.has(&actual_hash).map_err(|error| {
            format!(
                "failed to check distfs blob existence hash={} root={}: {error:?}",
                actual_hash,
                options.root.display()
            )
        })?;

        store.put(&actual_hash, &bytes).map_err(|error| {
            format!(
                "failed to put distfs blob hash={} module_id={} root={}: {error:?}",
                actual_hash,
                module_id,
                options.root.display()
            )
        })?;

        if !existed {
            newly_written += 1;
        }
        module_count += 1;
    }

    if module_count == 0 {
        return Err(format!(
            "manifest has no module entries: {}",
            options.manifest.display()
        ));
    }

    println!(
        "hydrated builtin wasm distfs blobs: module_count={}, newly_written={}, root={}",
        module_count,
        newly_written,
        options.root.display()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        eprintln!("{}", usage());
        std::process::exit(2);
    }
}
