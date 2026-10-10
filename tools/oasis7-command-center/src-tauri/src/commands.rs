use oasis7_command_center_core::{
    CommandCenter,
    domain::*,
    github::{GitHubClient, GitHubSnapshot},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Semaphore;

struct Application {
    core: CommandCenter,
    github: GitHubClient,
    validated: bool,
}

pub struct Host {
    application: Arc<Mutex<Application>>,
    slots: Arc<Semaphore>,
}

impl Host {
    pub fn open(path: PathBuf) -> oasis7_command_center_core::Result<Self> {
        let core = CommandCenter::open(path)?;
        let github = core
            .load_github_cache()?
            .map(GitHubClient::restore)
            .unwrap_or_else(GitHubClient::new);
        Ok(Self {
            application: Arc::new(Mutex::new(Application {
                core,
                github,
                validated: false,
            })),
            slots: Arc::new(Semaphore::new(4)),
        })
    }

    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Application) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        // Reject excess consumers rather than queueing unbounded blocking jobs.
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "客户端正在处理请求，请稍后重试".to_string())?;
        let application = self.application.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            let mut application = application
                .lock()
                .map_err(|_| "应用状态不可用".to_string())?;
            work(&mut application)
        })
        .await
        .map_err(|_| "后台请求未能完成".to_string())?
    }
}

fn error(error: oasis7_command_center_core::Error) -> String {
    error.to_string()
}

#[tauri::command]
pub async fn open_project(host: State<'_, Host>, path: String) -> Result<Snapshot, String> {
    host.run(move |app| {
        let previous = app.core.snapshot().project.map(|p| (p.id, p.path));
        let result = app.core.open_project(path).map_err(error);
        let current = app.core.snapshot().project.map(|p| (p.id, p.path));
        if previous != current {
            app.github.reset();
            if let Some(snapshot) = app.core.load_github_cache().map_err(error)? {
                app.github = GitHubClient::restore(snapshot);
            }
        }
        result
    })
    .await
}

#[tauri::command]
pub async fn query_view(host: State<'_, Host>) -> Result<Snapshot, String> {
    host.run(|app| {
        // Restored paths are revalidated on a worker before the first projection.
        // Source failures are retained by Core; actual storage failures propagate.
        if !app.validated {
            if app.core.snapshot().project.is_some() {
                app.core.refresh().map_err(error)?;
            }
            app.validated = true;
        }
        Ok(app.core.snapshot())
    })
    .await
}

#[tauri::command]
pub async fn search_rules(
    host: State<'_, Host>,
    query: String,
    limit: usize,
) -> Result<Vec<DocumentSection>, String> {
    if query.len() > 4096 {
        return Err("搜索文字过长".into());
    }
    host.run(move |app| app.core.search(&query, limit.min(100)).map_err(error))
        .await
}

#[tauri::command]
pub async fn save_decision_draft(
    host: State<'_, Host>,
    draft: DecisionDraft,
) -> Result<DecisionRecord, String> {
    host.run(move |app| app.core.save_decision(draft).map_err(error))
        .await
}

#[tauri::command]
pub async fn save_preference(
    host: State<'_, Host>,
    preferences: Preferences,
) -> Result<(), String> {
    host.run(move |app| app.core.save_preferences(preferences).map_err(error))
        .await
}

#[tauri::command]
pub async fn read_source(
    host: State<'_, Host>,
    section_id: String,
    revision: String,
    line: usize,
) -> Result<(), String> {
    host.run(move |app| {
        app.core
            .record_reading(&section_id, &revision, line)
            .map_err(error)
    })
    .await
}

#[tauri::command]
pub async fn refresh_source(host: State<'_, Host>) -> Result<Snapshot, String> {
    host.run(|app| app.core.refresh().map_err(error)).await
}

#[tauri::command]
pub async fn rebuild_cache(host: State<'_, Host>) -> Result<Snapshot, String> {
    host.run(|app| app.core.rebuild_cache().map_err(error))
        .await
}

#[tauri::command]
pub async fn refresh_github(
    host: State<'_, Host>,
    gh_path: Option<String>,
) -> Result<GitHubSnapshot, String> {
    host.run(move |app| {
        let project = app.core.snapshot().project.ok_or("请先选择项目")?;
        let mut candidate = GitHubClient::restore(app.github.snapshot());
        let snapshot = candidate.refresh(std::path::Path::new(&project.path), gh_path.as_deref());
        app.core.save_github_cache(&snapshot).map_err(error)?;
        app.github = candidate;
        Ok(snapshot)
    })
    .await
}

#[tauri::command]
pub async fn choose_project(
    host: State<'_, Host>,
    handle: tauri::AppHandle,
) -> Result<Option<Snapshot>, String> {
    host.run(move |app| {
        let Some(folder) = handle
            .dialog()
            .file()
            .set_title("选择 Oasis7 Git 工作树")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let path = folder.into_path().map_err(|e| e.to_string())?;
        let previous = app.core.snapshot().project.map(|p| (p.id, p.path));
        let result = app.core.open_project(path).map_err(error);
        let current = app.core.snapshot().project.map(|p| (p.id, p.path));
        if previous != current {
            app.github.reset();
            if let Some(snapshot) = app.core.load_github_cache().map_err(error)? {
                app.github = GitHubClient::restore(snapshot);
            }
        }
        result.map(Some)
    })
    .await
}

#[tauri::command]
pub async fn associate_work(
    host: State<'_, Host>,
    work_url: String,
    goal_section_ids: Vec<String>,
) -> Result<WorkAssociation, String> {
    host.run(move |app| {
        app.core
            .associate_work(&work_url, goal_section_ids)
            .map_err(error)
    })
    .await
}

#[tauri::command]
pub async fn read_version(
    host: State<'_, Host>,
    section_id: String,
    revision: String,
) -> Result<Option<DocumentSection>, String> {
    host.run(move |app| app.core.read_version(&section_id, &revision).map_err(error))
        .await
}

#[tauri::command]
pub async fn get_github_snapshot(host: State<'_, Host>) -> Result<GitHubSnapshot, String> {
    host.run(|app| Ok(app.github.snapshot())).await
}

#[tauri::command]
pub async fn open_source_target(host: State<'_, Host>, target: String) -> Result<(), String> {
    host.run(move |app| {
        let destination = if target.starts_with("https:") {
            let url = url::Url::parse(&target).map_err(|_| "来源链接无效".to_string())?;
            if url.scheme() != "https"
                || url.host_str() != Some("github.com")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
            {
                return Err("只支持 GitHub HTTPS 来源链接".into());
            }
            url.to_string()
        } else {
            app.core
                .document_path(&target)
                .map_err(error)?
                .to_string_lossy()
                .into_owned()
        };
        #[cfg(target_os = "macos")]
        {
            let result = std::process::Command::new("/usr/bin/open")
                .arg("--")
                .arg(destination)
                .status()
                .map_err(|e| e.to_string())?;
            if !result.success() {
                return Err("系统未能打开来源".into());
            }
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = destination;
            Err("来源打开目前支持 macOS".into())
        }
    })
    .await
}
