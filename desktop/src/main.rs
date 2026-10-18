#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_error;
mod asset_library;
mod assistant;
mod canvas_inputs;
mod caption_animation;
mod creation_commands;
mod database;
mod diagnostics;
mod ges_engine;
mod ges_output;
mod ges_runtime;
mod imports;
mod job_download;
mod job_locks;
mod job_recovery;
mod job_worker;
mod jobs;
mod media_commands;
#[cfg(target_os = "macos")]
mod metal_preview;
mod model_adapters;
mod models;
mod native_preview;
mod preview_prepare;
mod preview_process;
mod preview_protocol;
mod preview_session;
#[cfg(target_os = "macos")]
mod preview_shared;
mod preview_surface;
mod preview_worker;
mod previs;
mod production_queue;
mod project_service;
mod project_storage;
mod projects;
mod reference_commands;
mod storage;
mod system_voice;
mod updates;
mod waveform;
#[cfg(windows)]
mod windows_host;
use tauri::{Emitter, Manager};
#[tauri::command]
fn finish_exit(app: tauri::AppHandle) {
    assistant::parent_activation::close_all();
    app.exit(0);
}
fn main() {
    ges_runtime::configure();
    if std::env::args().nth(1).as_deref() == Some("--preview-worker") {
        if let Err(error) = preview_worker::run() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .setup(|app| {
            match app
                .path()
                .app_log_dir()
                .ok()
                .and_then(|path| diagnostics::init(&path).ok())
            {
                Some(guard) => {
                    app.manage(guard);
                }
                None => eprintln!("Operational diagnostic logging unavailable"),
            }
            let store = database::Store::open(app.path().app_data_dir()?)?;
            storage::allow(app.handle(), &store.media_root())?;
            app.manage(store);
            app.manage(previs::RuntimeState::default());
            app.manage(previs::Chats::default());
            app.manage(previs::Renders::default());
            assistant::skills::initialize(app.handle())?;
            production_queue::start(app.handle().clone());
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
            previs::files::previs_workspace,
            previs::files::save_previs_file,
            previs::files::delete_previs_model,
            previs::chat::previs_chat,
            previs::chat::cancel_previs_chat,
            previs::runtime::open_previs_runtime,
            previs::runtime::close_previs_runtime,
            previs::render::begin_previs_render,
            previs::render::append_previs_frame,
            previs::render::finish_previs_render,
            previs::render::cancel_previs_render,
            previs::references::attach_previs_reference,
            previs::references::collect_previs_video,
            updates::check_app_update,
            updates::open_release_page,
            waveform::audio_waveform,
            native_preview::native_preview_open,
            native_preview::native_preview_control,
            native_preview::native_preview_frame,
            native_preview::native_preview_frame_info,
            native_preview::native_preview_present,
            native_preview::native_preview_bounds,
            native_preview::native_preview_status,
            storage::storage_settings,
            storage::maintenance::scan_storage,
            storage::maintenance::clean_storage,
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
            caption_animation::store_caption_animation,
            creation_commands::save_subtitles,
            assistant::profiles::agent_catalog,
            assistant::profiles::save_agent,
            assistant::agent_history,
            assistant::execution::agent_execution_state,
            assistant::skills::creative_skills,
            assistant::skills::read_creative_skill,
            assistant::skills::save_creative_skill,
            assistant::journal::agent_turn_events,
            assistant::child_activity::agent_child_activity,
            assistant::assistant_chat,
            assistant::media_prompt::prepare_media_prompt,
            assistant::pending::cancel_assistant,
            models::codex_connection::connect_codex_service,
            model_adapters::codex_setup::codex_login,
            model_adapters::codex_setup::open_codex_download,
            models::media::media_model_catalog,
            models::media::save_media_model,
            models::media::remove_media_model,
            models::commands::model_catalog,
            models::connections::service_connections,
            models::connections::save_service_connection,
            models::connections::remove_service_connection,
            models::connections::discover_service_models,
            models::context_commands::resolve_text_model_limits,
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
            project_service::get_project,
            projects::create_project,
            projects::delete_project,
            imports::import_media,
            imports::video_url::import_video_url,
            imports::video_url::cancel_video_import,
            imports::import_clipboard_files,
            imports::transfer::begin_import,
            imports::transfer::append_import,
            imports::transfer::finish_import,
            imports::transfer::cancel_import,
            media_commands::render_video,
            media_commands::video_source_ends,
            media_commands::save_export,
            media_commands::reveal_export
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
