use oasis7_command_center_core::{
    CommandCenter,
    codex::{Runtime, SessionConfig},
};
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 || (args.len() == 2 && args[1] != "--turn") {
        return Err("usage: codex_smoke ABSOLUTE_NATIVE_CODEX_PATH [--turn]".into());
    }
    let directory = tempfile::tempdir()?.keep();
    println!("acceptance_artifacts={}", directory.display());
    let root = directory.join("project");
    std::fs::create_dir_all(root.join("doc"))?;
    std::fs::write(
        root.join("doc/test.md"),
        "# Isolated runtime acceptance\nNo writes expected.\n",
    )?;
    git(&root, &["init", "-q"])?;
    git(&root, &["add", "doc/test.md"])?;
    git(
        &root,
        &[
            "-c",
            "user.name=Runtime Acceptance",
            "-c",
            "user.email=acceptance@example.invalid",
            "commit",
            "-qm",
            "Initialize isolated acceptance fixture",
        ],
    )?;
    let db = directory.join("acceptance.sqlite");
    let mut core = CommandCenter::open(&db)?;
    let project = core.open_project(&root)?.project.ok_or("project missing")?;
    let mut runtime = Runtime::open(&db)?;
    runtime.set_enabled(&project, true)?;
    let connected = runtime.connect(&project, &args[0])?;
    println!(
        "version={:?} state={} capabilities={:?}",
        connected.version, connected.state, connected.capabilities
    );
    if args.len() == 2 {
        let session = runtime.create_session(
            &project,
            SessionConfig {
                purpose: "Isolated protocol acceptance".into(),
                scope: "One reply; no tool calls or file modifications".into(),
                model: None,
                basis_revision: project.head.clone(),
            },
        )?;
        let id = session
            .sessions
            .last()
            .ok_or("session missing")?
            .thread_id
            .clone();
        let readiness = Instant::now() + Duration::from_secs(10);
        loop {
            let snapshot = runtime.snapshot(&project)?;
            if snapshot
                .sessions
                .iter()
                .any(|s| s.thread_id == id && s.background_state == "clear")
            {
                break;
            }
            if Instant::now() >= readiness {
                return Err("new thread background scope unconfirmed".into());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let started = runtime.start_turn(
            &project,
            &id,
            "Reply with exactly OK. Do not call tools, run commands, or modify any file.",
        )?;
        let turn = started
            .sessions
            .iter()
            .find(|s| s.thread_id == id)
            .and_then(|s| s.turn_id.clone())
            .ok_or("turn identity unconfirmed")?;
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let snapshot = runtime.snapshot(&project)?;
            let session = snapshot
                .sessions
                .iter()
                .find(|s| s.thread_id == id)
                .ok_or("session lost")?;
            if session.turn_id.as_deref() == Some(&turn)
                && ["completed", "failed", "interrupted"].contains(&session.state.as_str())
            {
                println!(
                    "turn_state={} operation_states={:?}",
                    session.state,
                    snapshot
                        .operations
                        .iter()
                        .map(|o| o.state.as_str())
                        .collect::<Vec<_>>()
                );
                if session.state != "completed" {
                    println!("source_error_category={:?}", snapshot.error);
                    return Err("source did not confirm successful completion".into());
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err("turn result unconfirmed before acceptance timeout".into());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        let status = Command::new("/usr/bin/git")
            .current_dir(&root)
            .args(["status", "--porcelain"])
            .output()?;
        if !status.status.success() || !status.stdout.is_empty() {
            return Err("isolated fixture changed unexpectedly".into());
        }
    }
    let status = runtime.shutdown_if_idle()?;
    println!(
        "quit_safe={} active={} unknown={} requests={}",
        status.safe, status.active, status.unknown, status.pending_requests
    );
    if !status.safe {
        return Err("owned runtime shutdown unconfirmed; acceptance records retained".into());
    }
    drop(runtime);
    drop(core);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if !Command::new("/usr/bin/git")
        .current_dir(root)
        .args(args)
        .status()?
        .success()
    {
        return Err("fixture Git command failed".into());
    }
    Ok(())
}
