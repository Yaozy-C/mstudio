#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_error;
mod asset_library;
mod assistant;
mod canvas_inputs;
mod creation_commands;
mod database;
mod ges_engine;
mod ges_runtime;
mod imports;
mod job_download;
mod job_locks;
mod job_recovery;
mod job_worker;
mod jobs;
mod media_commands;
mod model_adapters;
mod models;
mod native_preview;
mod preview_prepare;
mod project_storage;
mod projects;
mod reference_commands;
mod storage;
mod waveform;
use tauri::{Emitter, Manager};
#[tauri::command]
fn finish_exit(app: tauri::AppHandle) {
    app.exit(0);
}
fn main() {
    ges_runtime::configure();
    tauri::Builder::default()
        .setup(|app| {
            let store = database::Store::open(app.path().app_data_dir()?)?;
            storage::allow(app.handle(), &store.media_root())?;
            app.manage(store);
            assistant::skills::initialize(app.handle())?;
            job_worker::start(app.handle().clone());
            database::history_cleanup::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("before-exit", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            finish_exit,
            waveform::audio_waveform,
            native_preview::native_preview_open,
            native_preview::native_preview_control,
            native_preview::native_preview_frame,
            native_preview::native_preview_status,
            storage::storage_settings,
            storage::choose_storage_directory,
            storage::migrate_storage,
            storage::asset_file_status,
            asset_library::list_global_assets,
            asset_library::add_global_asset,
            asset_library::remove_global_asset,
            asset_library::use_global_asset,
            imports::import_global_media,
            reference_commands::upload_references,
            creation_commands::list_voices,
            creation_commands::generate_voice,
            creation_commands::store_caption_image,
            creation_commands::save_subtitles,
            assistant::profiles::agent_catalog,
            assistant::profiles::save_agent,
            assistant::agent_history,
            assistant::memory::commands::project_memory,
            assistant::memory::commands::save_project_memory,
            assistant::skills::creative_skills,
            assistant::skills::read_creative_skill,
            assistant::skills::save_creative_skill,
            assistant::journal::agent_events,
            assistant::journal::agent_turn_events,
            assistant::tools::agent_tool_result,
            assistant::tools::agent_tool_active,
            assistant::assistant_chat,
            assistant::media_prompt::prepare_media_prompt,
            assistant::pending::cancel_assistant,
            model_adapters::codex_rpc::codex_image_status,
            models::media::media_model_catalog,
            models::media::save_media_model,
            models::media::remove_media_model,
            models::commands::model_catalog,
            models::connections::service_connections,
            models::connections::save_service_connection,
            models::connections::remove_service_connection,
            models::connections::discover_service_models,
            models::commands::save_model,
            models::commands::remove_model,
            models::commands::default_agent_model,
            models::commands::select_conversation_model,
            models::commands::test_model,
            jobs::list_jobs,
            jobs::submit_job,
            jobs::refresh_job,
            job_recovery::resolve_unknown_job,
            jobs::cancel_job,
            job_download::import_job_result,
            projects::list_projects,
            projects::save_project,
            projects::create_project,
            projects::delete_project,
            projects::get_settings,
            projects::save_setting,
            imports::import_media,
            imports::import_clipboard_files,
            imports::transfer::begin_import,
            imports::transfer::append_import,
            imports::transfer::finish_import,
            imports::transfer::cancel_import,
            media_commands::render_video,
            media_commands::save_export
        ])
        .build(tauri::generate_context!())
        .expect("启动 Mstudio 失败")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
                let _ = app.emit("before-exit", ());
            }
        });
}

#[cfg(test)]
mod storage_tests;
