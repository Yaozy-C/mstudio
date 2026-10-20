import { Dialog } from "@radix-ui/themes";
import {
  ArrowUpRight,
  ClockCounterClockwise,
  FilmStrip,
  MagnifyingGlass,
  X,
} from "@phosphor-icons/react";
import { mediaUrl } from "../../bridge";
import { t, useLanguage } from "../../i18n";
import { ErrorNotice } from "../../errors/ErrorNotice";
import type { AnalysisController } from "./controller";
import type { AnalysisState } from "./types";
import { timecode } from "./types";
import type { AnalysisHistoryState } from "./useAnalysisHistory";

export function AnalysisHistory({
  history,
  controller,
  state,
}: {
  history: AnalysisHistoryState;
  controller: AnalysisController;
  state: AnalysisState;
}) {
  const language = useLanguage();
  const date = (stamp: number) =>
    new Intl.DateTimeFormat(language === "en" ? "en" : "zh-CN", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(stamp);
  return (
    <Dialog.Root open={history.open} onOpenChange={history.setOpen}>
      <Dialog.Content
        className="analysis-history-dialog"
        aria-busy={history.loading || state.openingRecord}
      >
        <header className="analysis-history-heading">
          <div>
            <Dialog.Title>{t("拆解记录")}</Dialog.Title>
            <Dialog.Description>
              {t("保留每次观察，随时回到作品。")}
            </Dialog.Description>
          </div>
          <Dialog.Close>
            <button className="analysis-history-close" aria-label={t("关闭")}>
              <X size={20} aria-hidden="true" />
            </button>
          </Dialog.Close>
        </header>
        <label className="analysis-history-search">
          <MagnifyingGlass size={17} aria-hidden="true" />
          <input
            aria-label={t("搜索拆解记录")}
            placeholder={t("搜索作品、关注点或拆解内容")}
            maxLength={200}
            value={history.query}
            onChange={(event) => history.setQuery(event.target.value)}
          />
        </label>
        <div className="analysis-history-list">
          <ErrorNotice
            error={history.error || (state.openingRecord ? "" : state.error)}
          />
          {history.error && (
            <button onClick={history.reload}>{t("重试")}</button>
          )}
          {!history.items.length && !history.error && (
            <div className="analysis-history-empty">
              <ClockCounterClockwise size={30} aria-hidden="true" />
              <h3>
                {history.loading
                  ? t("正在读取记录…")
                  : history.query
                    ? t("没有找到匹配的记录")
                    : t("还没有拆解记录")}
              </h3>
              <p>
                {history.query
                  ? t("试试其他作品名或关键词。")
                  : t("完成一次拆解，原视频与结果会自动保存在这里。")}
              </p>
            </div>
          )}
          {history.items.map((record) => (
            <button
              key={record.id}
              className="analysis-history-item"
              disabled={!!state.busy || state.openingRecord}
              data-selected={state.recordId === record.id}
              onClick={async () => {
                if (await controller.openRecord(record.id))
                  history.setOpen(false);
              }}
            >
              <span className="analysis-history-poster">
                {record.poster ? (
                  <img src={mediaUrl(record.poster)} alt="" loading="lazy" />
                ) : (
                  <FilmStrip size={22} aria-hidden="true" />
                )}
                <span>{timecode(record.duration)}</span>
              </span>
              <span className="analysis-history-copy">
                <strong>{record.title}</strong>
                <span>
                  {date(record.createdAt)} · {record.modelName}
                </span>
                <p>{record.question || record.summary}</p>
                {record.missingVideo && (
                  <small>{t("原视频暂不可用，仍可查看结果")}</small>
                )}
              </span>
              <ArrowUpRight size={17} aria-hidden="true" />
            </button>
          ))}
          {history.items.length < history.total && (
            <button
              className="analysis-history-more"
              disabled={history.loading}
              onClick={history.more}
            >
              {history.loading ? t("正在读取记录…") : t("加载更多")}
            </button>
          )}
        </div>
        <footer>
          <span>{t("共 {count} 条记录", { count: history.total })}</span>
          <span>
            {state.openingRecord
              ? t("正在打开记录…")
              : t("保存在本机，重启后仍可回看")}
          </span>
        </footer>
      </Dialog.Content>
    </Dialog.Root>
  );
}
