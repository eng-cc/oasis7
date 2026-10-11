use super::{RuntimeHost, exit_ready};
use std::sync::atomic::Ordering;
use tauri::Emitter;

impl RuntimeHost {
    pub fn exit_is_approved(&self) -> bool {
        self.approved_exit.load(Ordering::Acquire)
    }

    // Every menu, keyboard and OS exit request reaches this same atomic check.
    // The approval flag is set only after Core confirms idle and reaps owned children.
    pub fn request_exit(&self, app: tauri::AppHandle) {
        if self.exit_checking.swap(true, Ordering::AcqRel) {
            return;
        }
        let runtime = self.runtime.clone();
        let failure = self.failure.clone();
        let approved = self.approved_exit.clone();
        let checking = self.exit_checking.clone();
        std::thread::spawn(move || {
            let status = runtime
                .lock()
                .map_err(|_| "执行状态不可用".to_string())
                .and_then(|mut r| {
                    let r = r.as_mut().ok_or_else(|| {
                        failure
                            .as_ref()
                            .clone()
                            .unwrap_or_else(|| "执行状态不可用".into())
                    })?;
                    let status = r.shutdown_if_idle().map_err(|e| e.to_string())?;
                    if exit_ready(&status) {
                        approved.store(true, Ordering::Release);
                    }
                    Ok(status)
                });
            checking.store(false, Ordering::Release);
            match status {
                Ok(status) if exit_ready(&status) => {
                    app.exit(0);
                }
                _ => {
                    crate::show_main(&app);
                    let _ = app.emit_to("main", "command-center:exit-required", ());
                }
            }
        });
    }

    pub(super) fn watch_exit(&self, app: tauri::AppHandle) {
        if self.watching.swap(true, Ordering::AcqRel) {
            return;
        }
        let owner = self.clone();
        let runtime = self.runtime.clone();
        let failure = self.failure.clone();
        let waiting = self.waiting.clone();
        let watching = self.watching.clone();
        let approved = self.approved_exit.clone();
        std::thread::spawn(move || {
            while waiting.load(Ordering::Acquire) {
                let status = runtime
                    .lock()
                    .map_err(|_| "执行状态不可用".to_string())
                    .and_then(|mut r| {
                        if !waiting.load(Ordering::Acquire) {
                            return Err("已取消退出等待".into());
                        }
                        let r = r.as_mut().ok_or_else(|| {
                            failure
                                .as_ref()
                                .clone()
                                .unwrap_or_else(|| "执行状态不可用".into())
                        })?;
                        let status = r.shutdown_if_idle().map_err(|e| e.to_string())?;
                        if exit_ready(&status) {
                            approved.store(true, Ordering::Release);
                        }
                        Ok(status)
                    });
                if let Ok(status) = status
                    && exit_ready(&status)
                {
                    app.exit(0);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(750));
            }
            watching.store(false, Ordering::Release);
            // A new wait request may race the previous watcher's cancellation.
            if waiting.load(Ordering::Acquire) && !approved.load(Ordering::Acquire) {
                owner.watch_exit(app);
            }
        });
    }
}
