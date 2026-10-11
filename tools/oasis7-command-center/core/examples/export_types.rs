use oasis7_command_center_core::domain::*;
use oasis7_command_center_core::github::{Check, GitHubSnapshot, PullRequest};
use ts_rs::TS;
fn main() {
    let declarations = [
        Project::decl(),
        SourceStatus::decl(),
        DocumentSection::decl(),
        Preferences::decl(),
        DecisionDraft::decl(),
        DecisionRecord::decl(),
        ReadingPosition::decl(),
        GitSnapshot::decl(),
        WorkAssociation::decl(),
        Snapshot::decl(),
        GitHubSnapshot::decl(),
        PullRequest::decl(),
        Check::decl(),
    ];
    let result = format!(
        "// Generated from Rust DTOs by cargo run -p oasis7-command-center-core --example export_types\n{}\n",
        declarations
            .iter()
            .map(|s| format!("export {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/generated/core.ts");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, result).unwrap();
}
