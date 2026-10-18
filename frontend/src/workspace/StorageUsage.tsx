import { useEffect, useRef, useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { ArrowsClockwise, Broom, HardDrives } from "@phosphor-icons/react";
import { bridge } from "../bridge";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { ActionButton } from "../ui/ActionButton";
import { AsyncButton, LoadingState, StatusMessage } from "../ui/AsyncState";
import { flushBeforeStorageChange } from "./useSafeExit";
import {
  formatBytes,
  type StorageUsage as Usage,
  type CleanupResult,
} from "./storageUsageModel";
import "../styles/storage-usage.css";
const names: Record<string, string> = {
  media: "素材与缩略图",
  previs: "预演与导出",
  snapshots: "任务与会话媒体",
  cache: "预览与字幕缓存",
  temporary: "临时文件",
  other: "其他文件",
};
export function StorageUsage({
  disabled,
  onBusyChange,
}: {
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
}) {
  useLanguage();
  const [usage, setUsage] = useState<Usage | null>(null);
  const [busy, setBusy] = useState<"scan" | "clean" | null>(null);
  const [error, setError] = useState("");
  const [open, setOpen] = useState(false);
  const [result, setResult] = useState<CleanupResult | null>(null);
  const active = useRef(true);
  const running = useRef(false);
  async function scan(preview = false) {
    if (running.current || disabled) return;
    running.current = true;
    setBusy("scan");
    onBusyChange(true);
    setError("");
    try {
      await flushBeforeStorageChange();
      const value = await bridge<Usage>("scan_storage");
      if (active.current) {
        setUsage(value);
        if (preview) setOpen(true);
      }
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      running.current = false;
      if (active.current) {
        setBusy(null);
        onBusyChange(false);
      }
    }
  }
  useEffect(() => {
    active.current = true;
    void scan();
    return () => {
      active.current = false;
    };
  }, []);
  async function clean() {
    if (!usage || running.current || disabled) return;
    running.current = true;
    setBusy("clean");
    onBusyChange(true);
    setError("");
    try {
      await flushBeforeStorageChange();
      const value = await bridge<CleanupResult>("clean_storage", {
        fingerprint: usage.fingerprint,
      });
      if (active.current) {
        setUsage(value.report);
        setResult(value);
        setOpen(false);
      }
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      running.current = false;
      if (active.current) {
        setBusy(null);
        onBusyChange(false);
      }
    }
  }
  const hasUnused = !!usage && usage.unusedFiles + usage.unusedRecords > 0;
  return (
    <div className="storage-usage" aria-busy={!!busy}>
      <div className="storage-usage-heading">
        <h3>
          <HardDrives size={20} />
          {t("空间占用")}
        </h3>
        <ActionButton
          icon={ArrowsClockwise}
          busy={busy === "scan"}
          disabled={!!busy || disabled}
          onClick={() => void scan()}
        >
          {t("刷新")}
        </ActionButton>
      </div>
      {!usage && busy === "scan" && (
        <LoadingState label={t("正在统计空间与引用…")} rows={2} />
      )}
      {usage && (
        <>
          <div className="storage-total">
            <strong>{formatBytes(usage.totalBytes)}</strong>
            <span>{t("Mstudio 数据占用")}</span>
          </div>
          <dl className="storage-summary">
            <div>
              <dt>{t("素材目录")}</dt>
              <dd>{formatBytes(usage.mediaBytes)}</dd>
            </div>
            <div>
              <dt>{t("应用记录与其他数据")}</dt>
              <dd>{formatBytes(usage.dataBytes)}</dd>
            </div>
            <div>
              <dt>{t("预计可释放")}</dt>
              <dd>{formatBytes(usage.reclaimableBytes)}</dd>
            </div>
          </dl>
          <details className="storage-breakdown">
            <summary>{t("查看文件分类")}</summary>
            <dl>
              {Object.entries(usage.categories).map(([key, value]) => (
                <div key={key}>
                  <dt>{t(names[key] || "其他文件")}</dt>
                  <dd>
                    {formatBytes(value.bytes)}
                    <small>{t("{v0} 个文件", { v0: value.files })}</small>
                  </dd>
                </div>
              ))}
            </dl>
          </details>
          <div className="storage-cleanup-card">
            <div>
              <h3>{t("清理无引用数据")}</h3>
              <p>
                {t(
                  "检查所有项目、素材库、任务、会话和预演，清理没有引用的记录与文件。",
                )}
              </p>
            </div>
            <ActionButton
              icon={Broom}
              disabled={!!busy || disabled || !hasUnused || !usage.canClean}
              onClick={() => void scan(true)}
            >
              {t("查看并清理")}
            </ActionButton>
            <span className="storage-cleanup-count">
              {hasUnused
                ? t("{v0} 个文件 · {v1} 条记录可清理", {
                    v0: usage.unusedFiles,
                    v1: usage.unusedRecords,
                  })
                : t("暂无可清理的无引用数据")}
            </span>
            {!usage.canClean && (
              <p>{t("有任务尚未结束，结束后刷新即可清理。")}</p>
            )}
          </div>
          <p className="storage-note">
            {t(
              "仅统计本机数据，应用程序本身不计入。最近 5 分钟写入的文件暂不清理。",
            )}
          </p>
        </>
      )}
      {result && (
        <StatusMessage kind={result.warning ? "info" : "success"}>
          {result.warning ||
            t("已清理 {v0} 个文件、{v1} 条记录，释放 {v2}。", {
              v0: result.deletedFiles,
              v1: result.deletedRecords,
              v2: formatBytes(result.freedBytes),
            })}
        </StatusMessage>
      )}
      {error && !open && (
        <ErrorNotice error={error} fallback="STORAGE_FAILED" />
      )}
      {!usage && !busy && (
        <ActionButton icon={ArrowsClockwise} onClick={() => void scan()}>
          {t("重新读取")}
        </ActionButton>
      )}
      <AlertDialog.Root
        open={open}
        onOpenChange={(value) => {
          if (!busy) setOpen(value);
        }}
      >
        <AlertDialog.Content
          className="modal storage-cleanup-dialog"
          aria-busy={busy === "clean"}
        >
          <AlertDialog.Title>{t("清理无引用数据")}</AlertDialog.Title>
          <AlertDialog.Description>
            {t(
              "将永久删除下列无引用数据。执行前会再次检查引用；仍在使用的素材、任务媒体与已导出的成片会保留。",
            )}
          </AlertDialog.Description>
          {usage && (
            <dl className="storage-summary">
              <div>
                <dt>{t("文件")}</dt>
                <dd>{usage.unusedFiles}</dd>
              </div>
              <div>
                <dt>{t("记录")}</dt>
                <dd>{usage.unusedRecords}</dd>
              </div>
              <div>
                <dt>{t("预计可释放")}</dt>
                <dd>{formatBytes(usage.reclaimableBytes)}</dd>
              </div>
            </dl>
          )}
          {usage && (
            <details className="storage-breakdown">
              <summary>{t("查看清理明细（最多显示 100 项）")}</summary>
              <ul className="storage-cleanup-items">
                {usage.items.map((item, index) => (
                  <li key={index}>
                    <span>{item.name}</span>
                    <small>
                      {item.kind === "file"
                        ? formatBytes(item.bytes)
                        : t("记录")}
                    </small>
                  </li>
                ))}
              </ul>
            </details>
          )}
          <p className="storage-note">
            {t(
              "包括无引用的中间文件与缓存；缺失但仍被引用的素材不会删除。数据库整理后可能释放额外空间。",
            )}
          </p>
          {error && (
            <ErrorNotice error={error} fallback="STORAGE_FAILED">
              <ActionButton
                icon={ArrowsClockwise}
                disabled={!!busy}
                onClick={() => void scan(true)}
              >
                {t("重新扫描")}
              </ActionButton>
            </ErrorNotice>
          )}
          <footer>
            <AlertDialog.Cancel>
              <button disabled={!!busy}>{t("取消")}</button>
            </AlertDialog.Cancel>
            <AsyncButton
              className="primary"
              busy={busy === "clean"}
              busyLabel={t("正在清理…")}
              disabled={!!busy || !usage?.canClean || !hasUnused}
              onClick={() => void clean()}
            >
              {t("确认清理")}
            </AsyncButton>
          </footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </div>
  );
}
