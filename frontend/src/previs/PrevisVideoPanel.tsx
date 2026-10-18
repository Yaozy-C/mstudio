import { useEffect } from "react";
import { Dialog } from "@radix-ui/themes";
import { ArrowClockwise, Cube, VideoCamera, X } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { t, useLanguage } from "../i18n";
import { ActionButton } from "../ui/ActionButton";
import { IconButton } from "../ui/IconButton";
import { StatusMessage } from "../ui/AsyncState";
import { usePrevis } from "./context";
import { PrevisTaskList } from "./PrevisTaskList";
import { PrevisVideoDetail } from "./PrevisVideoDetail";
import { latestPrevisVideos } from "./savedTasks";
import { isStale } from "./types";
import "./previs-videos.css";

export function PrevisVideoPanel() {
  useLanguage();
  const {
    workspace,
    project,
    videoPanel,
    showVideos,
    closeVideos,
    refresh,
    loading,
    error,
    tasks,
    open,
  } = usePrevis();
  const videos = latestPrevisVideos(workspace.exports);
  const selected =
    videos.find((video) => video.fileId === videoPanel.fileId) ?? videos[0];
  const activity = tasks.filter(
    (task) => task.running || task.error || task.phase === "cancelled",
  );
  useEffect(() => {
    if (videoPanel.open) void refresh();
  }, [videoPanel.open, refresh]);
  return (
    <Dialog.Root
      open={videoPanel.open}
      onOpenChange={(value) => (value ? showVideos() : closeVideos())}
    >
      <Dialog.Trigger>
        <button type="button" className="chrome-action task-dialog-trigger">
          <Cube />
          {t("预演视频")} <span>{videos.length}</span>
        </button>
      </Dialog.Trigger>
      <Dialog.Content
        className="previs-videos-dialog"
        aria-describedby="previs-videos-description"
      >
        <header className="task-panel-heading">
          <Dialog.Title>{t("预演视频")}</Dialog.Title>
          <ActionButton
            icon={ArrowClockwise}
            busy={loading}
            onClick={() => void refresh()}
          >
            {t("刷新")}
          </ActionButton>
          <Dialog.Close>
            <IconButton icon={X} label={t("关闭预演视频")} />
          </Dialog.Close>
        </header>
        <Dialog.Description
          id="previs-videos-description"
          className="previs-videos-description"
        >
          {t("集中查看各镜头最近保存的预演视频。重新进入项目后仍会保留。")}
        </Dialog.Description>
        {error && <StatusMessage kind="error">{error}</StatusMessage>}
        <div className="previs-videos-body">
          <nav className="task-panel-list" aria-label={t("预演视频列表")}>
            <PrevisTaskList
              tasks={activity}
              open={(fileId) => {
                closeVideos();
                open(fileId);
              }}
            />
            {videos.map((video) => {
              const target = project.nodes.find(
                (node) => node.id === video.fileId && node.shot,
              );
              const file = workspace.files.find(
                (entry) => entry.id === video.fileId,
              );
              const name = target?.title || file?.name || video.asset.name;
              return (
                <button
                  type="button"
                  key={video.fileId}
                  aria-pressed={selected?.fileId === video.fileId}
                  className={
                    selected?.fileId === video.fileId ? "selected" : ""
                  }
                  onClick={() => showVideos(video.fileId)}
                >
                  <span className="task-thumbnail" aria-hidden="true">
                    {video.asset.preview ? (
                      <img
                        src={mediaUrl(video.asset.preview)}
                        alt=""
                        loading="lazy"
                      />
                    ) : (
                      <VideoCamera size={20} />
                    )}
                  </span>
                  <span className="task-list-copy">
                    <strong title={name}>{name}</strong>
                    <small>
                      {t("{seconds} 秒", {
                        seconds: video.asset.duration.toFixed(2),
                      })}
                    </small>
                    <small>
                      {!target
                        ? t("镜头已移除，视频已保留")
                        : isStale(video, workspace.files)
                          ? t("场景已有修改")
                          : t("预演视频已保存")}
                    </small>
                  </span>
                </button>
              );
            })}
            {!videos.length && (
              <p>
                {loading ? t("正在读取预演视频…") : t("暂无已保存的预演视频。")}
              </p>
            )}
          </nav>
          <section
            className="previs-videos-detail"
            aria-label={t("预演视频详情")}
          >
            {selected ? (
              <PrevisVideoDetail key={selected.asset.id} video={selected} />
            ) : (
              <div className="previs-videos-empty">
                <Cube size={40} aria-hidden="true" />
                <p>{t("从镜头的三维预演中生成视频，完成后会显示在这里。")}</p>
              </div>
            )}
          </section>
        </div>
      </Dialog.Content>
    </Dialog.Root>
  );
}
