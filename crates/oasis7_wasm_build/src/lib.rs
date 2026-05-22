use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEFAULT_WASM_TARGET: &str = "wasm32-unknown-unknown";

#[derive(Debug)]
pub enum SourceHashError {
    InvalidManifestPath(String),
    MetadataCommand {
        manifest_path: PathBuf,
        stderr: String,
        status_code: Option<i32>,
    },
    MetadataJson(serde_json::Error),
    MissingRootPackage(PathBuf),
    MissingResolveGraph,
    MissingPackageNode(String),
    PathOutsideWorkspace {
        workspace_root: PathBuf,
        path: PathBuf,
    },
    NoTrackedFiles(PathBuf),
    Io {
        path: Option<PathBuf>,
        source: io::Error,
    },
}

impl fmt::Display for SourceHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceHashError::InvalidManifestPath(detail) => write!(f, "{detail}"),
            SourceHashError::MetadataCommand {
                manifest_path,
                stderr,
                status_code,
            } => write!(
                f,
                "cargo metadata failed for {} (status={status_code:?}): {}",
                manifest_path.display(),
                stderr
            ),
            SourceHashError::MetadataJson(source) => {
                write!(f, "failed to parse cargo metadata output: {source}")
            }
            SourceHashError::MissingRootPackage(path) => {
                write!(f, "root package not found for manifest {}", path.display())
            }
            SourceHashError::MissingResolveGraph => {
                write!(f, "cargo metadata did not return a resolve graph")
            }
            SourceHashError::MissingPackageNode(id) => {
                write!(f, "cargo metadata missing package for node {id}")
            }
            SourceHashError::PathOutsideWorkspace {
                workspace_root,
                path,
            } => write!(
                f,
                "path {} is outside workspace root {}",
                path.display(),
                workspace_root.display()
            ),
            SourceHashError::NoTrackedFiles(path) => {
                write!(
                    f,
                    "source whitelist produced no tracked files under {}",
                    path.display()
                )
            }
            SourceHashError::Io { path, source } => {
                if let Some(path) = path {
                    write!(f, "io error at {}: {}", path.display(), source)
                } else {
                    write!(f, "io error: {source}")
                }
            }
        }
    }
}

impl std::error::Error for SourceHashError {}

pub fn compute_source_hash(manifest_path: &Path, target: &str) -> Result<String, SourceHashError> {
    let manifest_path = canonical_or_original(manifest_path);
    let Some(module_dir) = manifest_path.parent() else {
        return Err(SourceHashError::InvalidManifestPath(format!(
            "manifest has no parent: {}",
            manifest_path.display()
        )));
    };
    let source_manifest_rel = manifest_path
        .strip_prefix(module_dir)
        .unwrap_or(manifest_path.as_path())
        .to_string_lossy()
        .to_string();
    let metadata = read_cargo_metadata(&manifest_path, target)?;
    let workspace_root = workspace_root(&metadata, &manifest_path);
    let package_dirs = collect_local_package_dirs(&metadata, &manifest_path)?;
    let mut hasher = Sha256::new();
    hasher.update(format!("source_manifest_rel={source_manifest_rel}\n").as_bytes());

    for package_dir in package_dirs {
        let manifest_rel = package_dir
            .join("Cargo.toml")
            .strip_prefix(&workspace_root)
            .map_err(|_| SourceHashError::PathOutsideWorkspace {
                workspace_root: workspace_root.clone(),
                path: package_dir.join("Cargo.toml"),
            })?
            .to_string_lossy()
            .to_string();
        hasher.update(format!("package_manifest_rel={manifest_rel}\n").as_bytes());
        let files = collect_source_files_for_hash(&package_dir)?;
        for file in files {
            let rel = file
                .strip_prefix(&package_dir)
                .map_err(|_| SourceHashError::Io {
                    path: Some(file.clone()),
                    source: io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "failed to strip package dir prefix {}",
                            package_dir.display()
                        ),
                    ),
                })?;
            let bytes = fs::read(&file).map_err(|source| SourceHashError::Io {
                path: Some(file.clone()),
                source,
            })?;
            hasher.update(
                format!(
                    "package_file:{}:{}:{}\n",
                    manifest_rel,
                    rel.to_string_lossy(),
                    sha256_hex(&bytes)
                )
                .as_bytes(),
            );
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn read_cargo_metadata(
    manifest_path: &Path,
    target: &str,
) -> Result<CargoMetadata, SourceHashError> {
    let mut args = vec![
        "metadata".to_string(),
        "--manifest-path".to_string(),
        manifest_path.to_string_lossy().to_string(),
        "--format-version".to_string(),
        "1".to_string(),
        "--filter-platform".to_string(),
        target.to_string(),
    ];
    let lock_path = manifest_path
        .parent()
        .map(|parent| parent.join("Cargo.lock"))
        .filter(|path| path.exists());
    if lock_path.is_some() {
        args.push("--locked".to_string());
    }

    let mut command = Command::new("cargo");
    command.env_remove("RUSTC_WRAPPER");
    command.args(&args);
    let output = command
        .output()
        .map_err(|source| SourceHashError::Io { path: None, source })?;
    if !output.status.success() {
        return Err(SourceHashError::MetadataCommand {
            manifest_path: manifest_path.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            status_code: output.status.code(),
        });
    }
    serde_json::from_slice(&output.stdout).map_err(SourceHashError::MetadataJson)
}

fn workspace_root(metadata: &CargoMetadata, manifest_path: &Path) -> PathBuf {
    if metadata.workspace_root.trim().is_empty() {
        manifest_path
            .parent()
            .map(canonical_or_original)
            .unwrap_or_else(|| canonical_or_original(manifest_path))
    } else {
        canonical_or_original(Path::new(metadata.workspace_root.as_str()))
    }
}

fn collect_local_package_dirs(
    metadata: &CargoMetadata,
    manifest_path: &Path,
) -> Result<Vec<PathBuf>, SourceHashError> {
    let canonical_manifest = canonical_or_original(manifest_path);
    let root_package = metadata
        .packages
        .iter()
        .find(|package| {
            canonical_or_original(Path::new(package.manifest_path.as_str())) == canonical_manifest
        })
        .ok_or_else(|| SourceHashError::MissingRootPackage(canonical_manifest.clone()))?;
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or(SourceHashError::MissingResolveGraph)?;
    let package_by_id = metadata
        .packages
        .iter()
        .map(|package| (package.id.clone(), package))
        .collect::<BTreeMap<_, _>>();
    let node_by_id = resolve
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<BTreeMap<_, _>>();
    let mut visited = BTreeSet::new();
    let mut ordered_dirs = Vec::new();
    let mut queue = VecDeque::from([root_package.id.clone()]);

    while let Some(package_id) = queue.pop_front() {
        if !visited.insert(package_id.clone()) {
            continue;
        }
        let package = package_by_id
            .get(&package_id)
            .copied()
            .ok_or_else(|| SourceHashError::MissingPackageNode(package_id.clone()))?;
        if package.source.is_none() {
            let manifest_path = canonical_or_original(Path::new(package.manifest_path.as_str()));
            let package_dir = manifest_path.parent().ok_or_else(|| {
                SourceHashError::InvalidManifestPath(format!(
                    "package manifest has no parent: {}",
                    manifest_path.display()
                ))
            })?;
            ordered_dirs.push(package_dir.to_path_buf());
        }
        if let Some(node) = node_by_id.get(&package_id) {
            for dep in &node.dependencies {
                queue.push_back(dep.clone());
            }
        }
    }

    ordered_dirs.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    ordered_dirs.dedup();
    Ok(ordered_dirs)
}

fn collect_source_files_for_hash(package_dir: &Path) -> Result<Vec<PathBuf>, SourceHashError> {
    let mut files = Vec::new();
    for rel in ["Cargo.toml", "Cargo.lock", "build.rs"] {
        let path = package_dir.join(rel);
        if path.is_file() {
            files.push(path);
        }
    }
    for root in ["src", "wit", ".cargo", "assets"] {
        collect_files_recursively(package_dir.join(root).as_path(), &mut files)?;
    }
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    files.dedup();
    if files.is_empty() {
        return Err(SourceHashError::NoTrackedFiles(package_dir.to_path_buf()));
    }
    Ok(files)
}

fn collect_files_recursively(dir: &Path, output: &mut Vec<PathBuf>) -> Result<(), SourceHashError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(SourceHashError::Io {
                path: Some(dir.to_path_buf()),
                source,
            })
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| SourceHashError::Io {
            path: Some(dir.to_path_buf()),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| SourceHashError::Io {
            path: Some(path.clone()),
            source,
        })?;
        if file_type.is_dir() {
            collect_files_recursively(path.as_path(), output)?;
            continue;
        }
        if file_type.is_file() {
            output.push(path);
        }
    }
    Ok(())
}

fn canonical_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    #[serde(default)]
    workspace_root: String,
    packages: Vec<CargoPackage>,
    resolve: Option<CargoResolve>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    id: String,
    manifest_path: String,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoResolve {
    nodes: Vec<CargoResolveNode>,
}

#[derive(Debug, Deserialize)]
struct CargoResolveNode {
    id: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs");
        }
        fs::write(path, contents).expect("write file");
    }

    fn write_fixture_workspace(
        root: &Path,
        shared_body: &str,
        extra_note: Option<&str>,
    ) -> PathBuf {
        write_file(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"module\", \"shared\"]\nresolver = \"2\"\n",
        );
        write_file(
            &root.join("module/Cargo.toml"),
            "[package]\nname = \"fixture_module\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nfixture_shared = { path = \"../shared\" }\n",
        );
        write_file(
            &root.join("module/src/lib.rs"),
            "pub fn call() -> u32 { fixture_shared::value() }\n",
        );
        if let Some(note) = extra_note {
            write_file(&root.join("module/README.md"), note);
        }
        write_file(
            &root.join("shared/Cargo.toml"),
            "[package]\nname = \"fixture_shared\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write_file(&root.join("shared/src/lib.rs"), shared_body);
        root.join("module/Cargo.toml")
    }

    #[test]
    fn source_hash_changes_when_local_path_dependency_changes() {
        let root = unique_temp_dir("oasis7-wasm-build-source-hash");
        let manifest_path =
            write_fixture_workspace(&root, "pub fn value() -> u32 { 1 }\n", Some("ignored note"));

        let initial =
            compute_source_hash(&manifest_path, DEFAULT_WASM_TARGET).expect("initial source hash");
        write_file(
            &root.join("shared/src/lib.rs"),
            "pub fn value() -> u32 { 2 }\n",
        );
        let changed =
            compute_source_hash(&manifest_path, DEFAULT_WASM_TARGET).expect("changed source hash");
        assert_ne!(initial, changed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_hash_ignores_non_whitelisted_files() {
        let root = unique_temp_dir("oasis7-wasm-build-source-hash-note");
        let manifest_path =
            write_fixture_workspace(&root, "pub fn value() -> u32 { 1 }\n", Some("note one"));

        let initial =
            compute_source_hash(&manifest_path, DEFAULT_WASM_TARGET).expect("initial source hash");
        write_file(&root.join("module/README.md"), "note two");
        let unchanged = compute_source_hash(&manifest_path, DEFAULT_WASM_TARGET)
            .expect("unchanged source hash");
        assert_eq!(initial, unchanged);
        let _ = fs::remove_dir_all(root);
    }
}
