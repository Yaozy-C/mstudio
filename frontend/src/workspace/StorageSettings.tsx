import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { ActionButton } from "../ui/ActionButton";
import { FolderOpen, ArrowsClockwise } from "@phosphor-icons/react";
import { bridge, native } from "../bridge";
import { flushBeforeStorageChange } from "./useSafeExit";
import "../styles/storage-settings.css";

type Storage = { directory: string; available: boolean };
export function StorageSettings({ projectId }: { projectId?: string }) {
  useLanguage();
  const [storage, setStorage] = useState<Storage | null>(null);
  const [target, setTarget] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(native);
  const [loadError, setLoadError] = useState("");
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    if (!native) return;
    let active = true;
    setLoading(true);
    setLoadError("");
    bridge<Storage>("storage_settings")
      .then((value) => {
        if (active) setStorage(value);
      })
      .catch((e) => {
        if (active) setLoadError(String(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [attempt]);
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
        result.warning ||
          t("存储目录已更新，已迁移 {v0} 个文件。", { v0: result.files }),
      );
      if (projectId && !result.warning)
        sessionStorage.setItem("mstudio-reopen-project", projectId);
      window.location.reload();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }
  if (!native)
    return (
      <section className="storage-settings">
        <h2>{t("存储")}</h2>
        <div className="storage-location storage-preview">
          <FolderOpen size={24} />
          <span>{t("请在桌面应用中管理素材目录")}</span>
        </div>
      </section>
    );
  return (
    <section className="storage-settings">
      <h2>{t("存储")}</h2>
      {loading ? (
        <p role="status">{t("正在读取…")}</p>
      ) : loadError ? (
        <ErrorNotice error={loadError} fallback="STORAGE_READ_FAILED">
          <ActionButton
            icon={ArrowsClockwise}
            onClick={() => setAttempt((value) => value + 1)}
          >
            {t("重新读取")}
          </ActionButton>
        </ErrorNotice>
      ) : (
        storage && (
          <div className="storage-location">
            <FolderOpen size={24} />
            <code>{storage.directory}</code>
          </div>
        )
      )}
      {storage && !storage.available && (
        <p className="error">
          {t("存储目录当前不可用，请连接原存储设备。项目中的文件位置已保留。")}
        </p>
      )}
      <ActionButton
        icon={FolderOpen}
        disabled={loading || !!loadError || busy || !storage}
        onClick={() => void choose()}
      >
        {t("选择新目录并迁移")}
      </ActionButton>
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
          <AlertDialog.Title>{t("迁移文件存储目录")}</AlertDialog.Title>
          <AlertDialog.Description>
            {t(
              "已有文件会复制并校验后迁移到下方位置，完成后重新载入应用。请保持存储设备连接。",
            )}
          </AlertDialog.Description>
          <div className="storage-location">
            <code>{target}</code>
          </div>
          {busy && (
            <p role="status">{t("正在迁移并校验文件，请勿关闭应用…")}</p>
          )}
          {error && <ErrorNotice error={error} fallback="STORAGE_FAILED" />}
          <footer>
            <AlertDialog.Cancel>
              <button disabled={busy}>{t("取消")}</button>
            </AlertDialog.Cancel>
            <button
              className="primary"
              disabled={busy}
              onClick={() => void migrate()}
            >
              {busy ? t("迁移中…") : t("迁移并切换")}
            </button>
          </footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </section>
  );
}
