import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { FolderOpen } from "@phosphor-icons/react";
import { bridge, native } from "../bridge";
import { flushBeforeStorageChange } from "./useSafeExit";
import "../styles/storage-settings.css";

type Storage = { directory: string; available: boolean };
export function StorageSettings({ projectId }: { projectId?: string }) {
  const [storage, setStorage] = useState<Storage | null>(null);
  const [target, setTarget] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    bridge<Storage>("storage_settings")
      .then(setStorage)
      .catch((e) => setError(String(e)));
  }, []);
  async function choose() {
    setError("");
    setBusy(true);
    try {
      setTarget(await bridge<string | null>("choose_storage_directory"));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function migrate() {
    if (!target || busy) return;
    setBusy(true);
    setError("");
    try {
      await flushBeforeStorageChange();
      const result = await bridge<{
        directory: string;
        files: number;
        warning: string;
      }>("migrate_storage", { directory: target });
      sessionStorage.setItem(
        "mstudio-storage-notice",
        result.warning || `存储目录已更新，已迁移 ${result.files} 个文件。`,
      );
      if (projectId && !result.warning)
        sessionStorage.setItem("mstudio-reopen-project", projectId);
      window.location.reload();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }
  return (
    <section className="storage-settings">
      <h3>文件存储目录</h3>
      <p>
        上传素材、生成结果、预览和成片统一保存在此处。项目记录与设置仍由应用管理。
      </p>
      <div className="storage-location">
        <FolderOpen size={24} />
        <code>{storage?.directory || "正在读取…"}</code>
      </div>
      {storage && !storage.available && (
        <p className="error">
          存储目录当前不可用，请连接原存储设备。项目中的文件位置已保留。
        </p>
      )}
      <button
        className="primary"
        disabled={!native || busy || !storage}
        onClick={() => void choose()}
      >
        选择新目录并迁移
      </button>
      <p className="subtle">
        会在所选位置创建 Mstudio
        文件夹，并迁移已有文件。文件量较大时可能需要一些时间；完成后会重新载入应用。
      </p>
      <h3>文件缺失时</h3>
      <p>
        如果从文件夹删除了文件，项目会保留“文件缺失”占位。镜头、时间线位置和引用关系不会被删除；把文件放回原位置后会自动恢复。
      </p>
      {error && !target && (
        <ErrorNotice error={error} fallback="STORAGE_FAILED" />
      )}
      <AlertDialog.Root
        open={!!target}
        onOpenChange={(open) => {
          if (!open && !busy) setTarget(null);
        }}
      >
        <AlertDialog.Content
          className="modal storage-migration"
          aria-busy={busy}
        >
          <AlertDialog.Title>迁移文件存储目录</AlertDialog.Title>
          <AlertDialog.Description>
            已有文件会复制并校验后迁移到下方位置，项目和全局素材的引用会一同更新。请保持存储设备连接。
          </AlertDialog.Description>
          <div className="storage-location">
            <code>{target}</code>
          </div>
          {busy && <p role="status">正在迁移并校验文件，请勿关闭应用…</p>}
          {error && <ErrorNotice error={error} fallback="STORAGE_FAILED" />}
          <footer>
            <AlertDialog.Cancel>
              <button disabled={busy}>取消</button>
            </AlertDialog.Cancel>
            <button
              className="primary"
              disabled={busy}
              onClick={() => void migrate()}
            >
              {busy ? "迁移中…" : "迁移并切换"}
            </button>
          </footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </section>
  );
}
