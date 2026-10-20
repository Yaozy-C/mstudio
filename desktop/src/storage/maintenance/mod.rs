//! Current reference graph and on-demand storage maintenance; no legacy conversion.
#[cfg(test)]
mod analysis_record_tests;
mod graph;
mod run;
#[cfg(test)]
mod safety_tests;
mod scan;
#[cfg(test)]
mod tests;
use crate::database::Store;
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::Manager;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    bytes: u64,
    files: u64,
    reclaimable_bytes: u64,
}
#[derive(Serialize)]
pub struct CleanupItem {
    kind: String,
    name: String,
    bytes: u64,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    total_bytes: u64,
    media_bytes: u64,
    data_bytes: u64,
    reclaimable_bytes: u64,
    unused_files: u64,
    unused_records: u64,
    categories: BTreeMap<String, Category>,
    items: Vec<CleanupItem>,
    fingerprint: String,
    can_clean: bool,
}
#[tauri::command]
pub async fn scan_storage(app: tauri::AppHandle) -> Result<Report, String> {
    let guard = app.state::<Store>().files.clone().read_owned().await;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let previs_active =
            crate::previs::render::is_rendering(&app.state::<crate::previs::Renders>())
                || !app
                    .state::<crate::previs::Chats>()
                    .0
                    .lock()
                    .unwrap()
                    .is_empty();
        let db = store.conn().map_err(|e| e.to_string())?;
        let mut report = scan::build(&db, &store.media_root(), &data)
            .map_err(|e| e.to_string())?
            .report;
        report.can_clean &= !previs_active;
        Ok(report)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn clean_storage(
    app: tauri::AppHandle,
    fingerprint: String,
) -> Result<run::Outcome, String> {
    let guard = app
        .state::<Store>()
        .files
        .clone()
        .try_write_owned()
        .map_err(|_| "正在读写文件，请等待导入、生成或导出结束后重试".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
        if crate::previs::render::is_rendering(&app.state::<crate::previs::Renders>())
            || !app
                .state::<crate::previs::Chats>()
                .0
                .lock()
                .unwrap()
                .is_empty()
        {
            return Err("预演任务正在执行，请等待结束后再清理".into());
        }
        let result = run::clean(&store, &data, &fingerprint).map_err(|e| e.to_string())?;
        for project in &result.changed_projects {
            store.project_changed(project);
            crate::project_service::notify(&app, project);
        }
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
