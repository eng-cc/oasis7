use oasis7_command_center_core::tools::{
    self, FileSignInput, FileSignPrepared, ToolObservation,
    jobs::{ToolJob, ToolJobs},
};
use std::{path::PathBuf, sync::Arc};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
pub struct ToolHost {
    pub(crate) jobs: Arc<ToolJobs>,
    pub(crate) secrets: Arc<tools::secrets::SecretStore>,
}
impl ToolHost {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        Ok(Self {
            jobs: Arc::new(ToolJobs::open(path).map_err(|e| e.to_string())?),
            secrets: Arc::new(tools::secrets::SecretStore::default()),
        })
    }
    // Admission records the exact nonsecret recovery projection alongside immutable coordinates.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn mutate_context<T: Send + 'static>(
        &self,
        id: String,
        fingerprint: String,
        resource: String,
        kind: &'static str,
        evidence: oasis7_command_center_core::context::ContextRecoveryEvidence,
        work: impl FnOnce() -> oasis7_command_center_core::Result<T> + Send + 'static,
    ) -> Result<T, String> {
        let jobs = self.jobs.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let (job, fresh) = jobs
                .admit_once_with_evidence(&id, &fingerprint, &resource, kind, &evidence)
                .map_err(|e| e.to_string())?;
            if !fresh {
                return Err(format!(
                    "Original operation {} is {}; inspect before retry",
                    job.id, job.state
                ));
            }
            let result = work();
            let state = match &result {
                Ok(_) => "succeeded",
                Err(oasis7_command_center_core::Error::Context(e)) if e.effect == "none" => {
                    "failed"
                }
                Err(_) => "unknown",
            };
            jobs.finish(&id, state, None).map_err(|e| e.to_string())?;
            result.map_err(|e| e.to_string())
        })
        .await
        .map_err(|_| "Operation did not complete; inspect original target".to_string())?
    }
    pub(crate) async fn mutate<T: Send + 'static>(
        &self,
        id: String,
        fingerprint: String,
        resource: String,
        kind: &'static str,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let jobs = self.jobs.clone();
        tauri::async_runtime::spawn_blocking(move || {
            // Admission is durable before starting; reusing any recorded ID only
            // returns its observed state, never replays a side effect.
            let (job, fresh) = jobs
                .admit_once(&id, &fingerprint, &resource, kind)
                .map_err(|e| e.to_string())?;
            if !fresh {
                return Err(format!(
                    "原操作 {} 状态为 {}；请核对原操作，不能自动重发",
                    job.id, job.state
                ));
            }
            let result = work();
            // Do not persist caller output, paths, context text or secret inputs.
            jobs.finish(
                &id,
                if result.is_ok() {
                    "succeeded"
                } else {
                    "unknown"
                },
                None,
            )
            .map_err(|e| e.to_string())?;
            result
        })
        .await
        .map_err(|_| "工具操作未能完成；请核对原操作".to_string())?
    }
}
#[tauri::command]
pub async fn tool_jobs(host: State<'_, ToolHost>) -> Result<Vec<ToolJob>, String> {
    let jobs = host.jobs.clone();
    tauri::async_runtime::spawn_blocking(move || jobs.list().map_err(|e| e.to_string()))
        .await
        .map_err(|_| "操作列表未能完成".to_string())?
}
#[tauri::command]
pub async fn tool_signer_observe() -> Result<ToolObservation, String> {
    tauri::async_runtime::spawn_blocking(tools::observe_signer)
        .await
        .map_err(|_| "检查未能完成".into())
}
#[tauri::command]
pub async fn tool_signer_doctor() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        tools::signer_action("doctor", None).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "检查未能完成".to_string())?
}
#[tauri::command]
pub async fn tool_choose_file(handle: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("选择本机签名文件")
            .blocking_pick_file()
            .map(|f| {
                f.into_path()
                    .map(|p| p.display().to_string())
                    .map_err(|e| e.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|_| "选择未能完成".to_string())?
}
#[tauri::command]
pub async fn tool_file_prepare(
    host: State<'_, ToolHost>,
    input: FileSignInput,
    operation_id: String,
) -> Result<FileSignPrepared, String> {
    let identity_input = input.clone();
    let (resource, fingerprint) = tauri::async_runtime::spawn_blocking(move || {
        tools::file_prepare_identity(&identity_input).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Input identity did not complete")??;
    let expected = fingerprint.clone();
    host.mutate(
        operation_id,
        fingerprint,
        resource,
        "file_prepare",
        move || {
            if tools::file_prepare_identity(&input)
                .map_err(|e| e.to_string())?
                .1
                != expected
            {
                return Err("Prepared input identity changed".into());
            }
            tools::prepare_file(&input).map_err(|e| e.to_string())
        },
    )
    .await
}
#[tauri::command]
pub async fn tool_signer_submit(
    host: State<'_, ToolHost>,
    job_id: String,
    operation_id: String,
    expected_prepared_fingerprint: String,
) -> Result<serde_json::Value, String> {
    let identity_job = job_id.clone();
    let (resource, fingerprint) = tauri::async_runtime::spawn_blocking(move || {
        tools::signer_job_identity(&identity_job).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Job identity did not complete")??;
    if fingerprint != expected_prepared_fingerprint {
        return Err("当前 Job 与预览的请求不同，请重新核对；未提交".into());
    }
    let expected = expected_prepared_fingerprint;
    host.mutate(operation_id, fingerprint, resource, "submit", move || {
        if tools::signer_job_identity(&job_id)
            .map_err(|e| e.to_string())?
            .1
            != expected
        {
            return Err("签名请求身份变化；请核对原请求".into());
        }
        tools::signer_action("submit", Some(&job_id)).map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub async fn tool_signer_inspect(
    host: State<'_, ToolHost>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let jobs = host.jobs.clone();
    tauri::async_runtime::spawn_blocking(move || {
        tools::reconcile_signer_job(&jobs, &job_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Inspection did not complete".to_string())?
}

#[tauri::command]
pub async fn secret_stage(
    host: State<'_, ToolHost>,
    window: tauri::WebviewWindow,
    operation_id: String,
    purpose: String,
    passphrase: String,
) -> Result<String, String> {
    if ![
        "context_snapshot_export",
        "context_snapshot_inspect",
        "context_snapshot_restore",
        "context_reconcile",
    ]
    .contains(&purpose.as_str())
    {
        return Err("未知机密输入用途".into());
    }
    let jobs = host.jobs.clone();
    let secrets = host.secrets.clone();
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        jobs.with_open(|| secrets.insert(&label, &operation_id, &purpose, passphrase.into_bytes()))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Secret staging did not complete".to_string())?
}

#[tauri::command]
pub async fn tool_cli_install(
    host: State<'_, ToolHost>,
    handle: tauri::AppHandle,
    operation_id: String,
) -> Result<tools::cli::CliInstallation, String> {
    let resource = handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("context-cli");
    let destination = handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("tools");
    let (reservation, fingerprint) =
        tools::cli::bundle_identity(&resource, &destination).map_err(|e| e.to_string())?;
    let expected = fingerprint.clone();
    host.mutate(
        operation_id,
        fingerprint,
        reservation,
        "cli_install",
        move || {
            if tools::cli::bundle_identity(&resource, &destination)
                .map_err(|e| e.to_string())?
                .1
                != expected
            {
                return Err("Bundled release identity changed".into());
            }
            tools::cli::install_bundled(&resource, &destination).map_err(|e| e.to_string())
        },
    )
    .await
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[tauri::command]
pub async fn tool_file_verify(
    file_path: String,
    context: serde_json::Value,
    expected_public_key: Vec<u8>,
    signature: Vec<u8>,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        tools::verify_file_signature(
            std::path::Path::new(&file_path),
            context,
            expected_public_key,
            signature,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Verification did not complete".to_string())?
}

#[tauri::command]
pub async fn tool_file_reconcile(
    host: State<'_, ToolHost>,
    input: FileSignInput,
) -> Result<serde_json::Value, String> {
    let jobs = host.jobs.clone();
    tauri::async_runtime::spawn_blocking(move || {
        tools::reconcile_file_prepare(&jobs, &input).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Reconciliation did not complete".to_string())?
}
#[tauri::command]
pub async fn tool_cli_observe(
    handle: tauri::AppHandle,
) -> Result<Option<tools::cli::CliInstallation>, String> {
    let resource = handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("context-cli");
    let destination = handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("tools");
    tauri::async_runtime::spawn_blocking(move || {
        tools::cli::observe_installed(&resource, &destination).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "CLI observation did not complete".to_string())?
}

#[tauri::command]
pub async fn tool_cli_reconcile(
    host: State<'_, ToolHost>,
    handle: tauri::AppHandle,
    operation_id: String,
) -> Result<Option<tools::cli::CliInstallation>, String> {
    let resource = handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("context-cli");
    let destination = handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("tools");
    let jobs = host.jobs.clone();
    tauri::async_runtime::spawn_blocking(move || {
        tools::cli::reconcile_cli_install(&jobs, &operation_id, &resource, &destination)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "CLI reconciliation did not complete".to_string())?
}
