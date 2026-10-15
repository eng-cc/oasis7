#[cfg(test)]
use std::path::Path;
use std::{collections::BTreeMap, io::Read};

const FILE_BYTES: usize = 1024 * 1024;
const OMITTED_SAMPLE: usize = 20;

/// Coverage of this tracked working-tree scan, not all repository documents.
pub(crate) struct Coverage {
    candidates: usize,
    pub read: usize,
    omitted: BTreeMap<&'static str, usize>,
    sample: Vec<String>,
}
impl Coverage {
    pub fn new(candidates: usize) -> Self {
        Self {
            candidates,
            read: 0,
            omitted: BTreeMap::new(),
            sample: Vec::new(),
        }
    }
    pub fn omit(&mut self, path: &str, reason: &'static str) {
        *self.omitted.entry(reason).or_default() += 1;
        if self.sample.len() < OMITTED_SAMPLE {
            self.sample.push(format!("{reason}: {path}"));
        }
    }
    pub fn has_omissions(&self) -> bool {
        !self.omitted.is_empty()
    }
    pub fn summary(&self, bytes: usize) -> String {
        let omitted: usize = self.omitted.values().sum();
        let counts = self
            .omitted
            .iter()
            .map(|(reason, count)| format!("{reason} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut summary = format!(
            "tracked Markdown in doc, AGENTS.md and project map; untracked files excluded; 候选 {}，已读 {}，遗漏 {}，已读字节 {}; 3000 paths, 1 MiB/file, 16 MiB total",
            self.candidates, self.read, omitted, bytes
        );
        if omitted > 0 {
            summary.push_str(&format!(
                "; 遗漏原因: {counts}; 路径样本（前 {} / {}）: {}",
                self.sample.len(),
                omitted,
                self.sample.join("; ")
            ));
        }
        summary
    }
}

#[cfg(test)]
pub(super) fn read_document(path: &Path, remaining: usize) -> Result<String, &'static str> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A tracked path replaced by a FIFO must not block before type validation.
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => "deleted",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "read_error",
    })?;
    read_open_document(file, remaining)
}

pub(super) fn read_open_document(
    file: std::fs::File,
    remaining: usize,
) -> Result<String, &'static str> {
    let metadata = file.metadata().map_err(|_| "read_error")?;
    if !metadata.is_file() {
        return Err("not_regular_file");
    }
    if metadata.len() > FILE_BYTES as u64 {
        return Err("file_budget");
    }
    if metadata.len() > remaining as u64 {
        return Err("total_budget");
    }
    // Cap the actual read too: a file can grow after its metadata was observed.
    let limit = FILE_BYTES.min(remaining);
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "read_error")?;
    if bytes.len() > FILE_BYTES {
        return Err("file_budget");
    }
    if bytes.len() > remaining {
        return Err("total_budget");
    }
    String::from_utf8(bytes).map_err(|_| "invalid_utf8")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_boundaries_and_non_text_are_explicit() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, vec![b'x'; FILE_BYTES]).unwrap();
        assert_eq!(read_document(&path, FILE_BYTES).unwrap().len(), FILE_BYTES);
        assert_eq!(read_document(&path, FILE_BYTES - 1), Err("total_budget"));
        std::fs::write(&path, vec![b'x'; FILE_BYTES + 1]).unwrap();
        assert_eq!(read_document(&path, FILE_BYTES * 2), Err("file_budget"));
        std::fs::write(&path, [0xff]).unwrap();
        assert_eq!(read_document(&path, FILE_BYTES), Err("invalid_utf8"));
        std::fs::write(&path, "").unwrap();
        assert_eq!(read_document(&path, 0).unwrap(), "");
        assert_eq!(
            read_document(tmp.path(), FILE_BYTES),
            Err("not_regular_file")
        );
    }
    #[cfg(unix)]
    #[test]
    fn fifo_is_rejected_without_waiting_for_a_writer() {
        use std::os::unix::ffi::OsStrExt;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fifo.md");
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert_eq!(read_document(&path, FILE_BYTES), Err("not_regular_file"));
    }
    #[test]
    fn omission_sample_is_bounded_but_counts_are_complete() {
        let mut coverage = Coverage::new(3025);
        coverage.read = 3000;
        for index in 0..25 {
            coverage.omit(&format!("doc/{index}.md"), "file_count_budget");
        }
        let summary = coverage.summary(42);
        assert!(summary.contains("候选 3025，已读 3000，遗漏 25"));
        assert!(summary.contains("file_count_budget 25"));
        assert!(summary.contains("前 20 / 25"));
        assert!(!summary.contains("doc/20.md"));
    }
}
