use crate::{Error, Result};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct ProcessIdentity {
    pub pid: u32,
    parent: u32,
    group: u32,
    started: String,
    state: String,
}

impl ProcessIdentity {
    pub(crate) fn same_birth(&self, other: &Self) -> bool {
        self.pid == other.pid && self.started == other.started
    }
}

fn processes() -> Result<Vec<ProcessIdentity>> {
    let raw = crate::process::run(
        "/bin/ps",
        &["-axo", "pid=,ppid=,pgid=,lstart=,stat="],
        Path::new("/"),
    )?;
    let mut rows = Vec::new();
    for line in String::from_utf8_lossy(&raw).lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 9 {
            return Err(Error::Invalid(
                "process identity observation is malformed".into(),
            ));
        }
        let number = |i: usize| {
            fields[i]
                .parse::<u32>()
                .map_err(|_| Error::Invalid("invalid process identity".into()))
        };
        rows.push(ProcessIdentity {
            pid: number(0)?,
            parent: number(1)?,
            group: number(2)?,
            started: fields[3..8].join(" "),
            state: fields[8].into(),
        });
    }
    Ok(rows)
}

/// Capture only this live child's descendants and owned process group, with birth evidence.
pub(crate) fn capture_owned(pid: u32) -> Result<Vec<ProcessIdentity>> {
    let rows = processes()?;
    let root = rows.iter().find(|r| r.pid == pid).ok_or_else(|| {
        Error::Invalid("owned process disappeared; background state unknown".into())
    })?;
    let mut owned = vec![root.clone()];
    loop {
        let before = owned.len();
        for row in &rows {
            if (row.group == pid || owned.iter().any(|p| p.pid == row.parent))
                && !owned.iter().any(|p| p.pid == row.pid)
            {
                owned.push(row.clone());
            }
        }
        if before == owned.len() {
            break;
        }
    }
    Ok(owned)
}

/// PID alone is insufficient after reparenting or reuse. Never signal a different birth identity.
pub(crate) fn survivors(expected: &[ProcessIdentity]) -> Result<Vec<ProcessIdentity>> {
    Ok(processes()?
        .into_iter()
        .filter(|p| {
            !p.state.starts_with('Z')
                && expected
                    .iter()
                    .any(|old| old.pid == p.pid && old.started == p.started)
        })
        .collect())
}

pub(crate) fn stop_verified(expected: &[ProcessIdentity]) -> Result<()> {
    // Children first; separate MCP process groups remain within the captured owned tree.
    for old in expected.iter().rev() {
        // Refresh before each signal rather than reuse a batch observation.
        if survivors(std::slice::from_ref(old))?
            .iter()
            .any(|p| p.same_birth(old))
        {
            #[cfg(unix)]
            if unsafe { libc::kill(old.pid as i32, libc::SIGKILL) } != 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error.into());
                }
            }
            #[cfg(not(unix))]
            return Err(Error::Invalid(
                "owned process stopping is unavailable on this platform".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    #[test]
    fn separate_group_descendants_stop_without_stopping_unrelated_processes() {
        let dir = tempfile::tempdir().unwrap();
        let ready = dir.path().join("helper");
        let mut unrelated = Command::new("python3")
            .args(["-c", "import time;time.sleep(30)"])
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let script = "import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)'],start_new_session=True); open(sys.argv[1],'w').write(str(p.pid)); time.sleep(30)";
        let mut parent = Command::new("python3")
            .args(["-c", script])
            .arg(&ready)
            .process_group(0)
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let helper: u32 = loop {
            if let Ok(value) = std::fs::read_to_string(&ready)
                && let Ok(pid) = value.parse()
            {
                break pid;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "helper PID was not published"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let owned = capture_owned(parent.id()).unwrap();
        assert!(owned.iter().any(|p| p.pid == helper && p.group == helper));
        assert!(!owned.iter().any(|p| p.pid == unrelated.id()));
        stop_verified(&owned).unwrap();
        parent.wait().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while !survivors(&owned).unwrap().is_empty() {
            assert!(
                std::time::Instant::now() < deadline,
                "owned descendant remains active"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(unrelated.try_wait().unwrap().is_none());
        unrelated.kill().unwrap();
        unrelated.wait().unwrap();
    }

    #[test]
    fn reused_birth_evidence_does_not_authorize_stopping_a_process() {
        let mut child = Command::new("python3")
            .args(["-c", "import time;time.sleep(30)"])
            .process_group(0)
            .spawn()
            .unwrap();
        let mut wrong = capture_owned(child.id()).unwrap();
        for identity in &mut wrong {
            identity.started = "different birth".into();
        }
        assert!(survivors(&wrong).unwrap().is_empty());
        stop_verified(&wrong).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        child.kill().unwrap();
        child.wait().unwrap();
    }
}
