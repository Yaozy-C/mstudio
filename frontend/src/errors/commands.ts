import { errorText, type ErrorCode } from "./catalog";
const commandCodes: Record<string, ErrorCode> = {
  native_preview_open: "PREVIEW_FAILED",
  native_preview_control: "PREVIEW_FAILED",
  native_preview_status: "PREVIEW_FAILED",
  native_preview_frame: "PREVIEW_FAILED",
  submit_job: "SUBMISSION_UNKNOWN",
  refresh_job: "JOB_SYNC_FAILED",
  list_jobs: "JOB_SYNC_FAILED",
  cancel_job: "JOB_CANCEL_FAILED",
  import_job_result: "RESULT_IMPORT_FAILED",
  upload_references: "ASSET_IMPORT_FAILED",
  save_project: "SAVE_FAILED",
  create_project: "SAVE_FAILED",
  render_video: "EXPORT_FAILED",
  save_export: "EXPORT_SAVE_FAILED",
  import_media: "ASSET_IMPORT_FAILED",
  import_global_media: "ASSET_IMPORT_FAILED",
  list_global_assets: "ASSET_OPERATION_FAILED",
  add_global_asset: "ASSET_OPERATION_FAILED",
  remove_global_asset: "ASSET_OPERATION_FAILED",
  use_global_asset: "ASSET_OPERATION_FAILED",
  assistant_chat: "CHAT_FAILED",
  migrate_storage: "STORAGE_FAILED",
  storage_settings: "STORAGE_READ_FAILED",
  choose_storage_directory: "STORAGE_FAILED",
};
export function commandError(command: string, value: unknown): string {
  return errorText(value, commandCodes[command] ?? "OPERATION_FAILED");
}
