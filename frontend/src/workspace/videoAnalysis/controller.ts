import { browseWorks, type BrowseWork } from "../browseWorks";
import {
  videoLink,
  workVideoLink,
  type AnalysisState,
  type ProgressUpdate,
  type AnalysisVideo,
  type AnalysisOutcome,
  type OpenedRecord,
  type RecordPage,
  type AnalysisInvoke,
} from "./types";

export function createAnalysisController(
  invoke: AnalysisInvoke,
  channel: (receive: (progress: ProgressUpdate) => void) => unknown = () =>
    undefined,
) {
  let state: AnalysisState = { busy: null, error: "", question: "" };
  let serial = 0;
  let progressSerial = 0;
  let activeId: string | undefined;
  const previews = new Map<string, string>();
  const listeners = new Set<() => void>();
  const update = (next: Partial<AnalysisState>) => {
    state = { ...state, ...next };
    listeners.forEach((listener) => listener());
  };
  const release = (id?: string) => {
    if (!id) return;
    const preview = previews.get(id);
    if (preview) {
      URL.revokeObjectURL(preview);
      previews.delete(id);
    } else void invoke("release_video_analysis", { id }).catch(() => {});
  };
  const progressChannel = (request: number) => {
    const channelId = ++progressSerial;
    return channel((progress) => {
      if (
        request !== serial ||
        channelId !== progressSerial ||
        !state.busy ||
        !state.progress
      )
        return;
      update({
        progress: { ...progress, startedAt: state.progress.startedAt },
      });
    });
  };
  function cancel() {
    serial++;
    if (activeId)
      void invoke("cancel_video_analysis", { id: activeId }).catch(() => {});
    activeId = undefined;
    update({ busy: null, progress: undefined, openingRecord: false });
  }
  function clear() {
    const previous = state.video?.id;
    cancel();
    release(previous);
    state = { busy: null, error: "", question: "" };
    listeners.forEach((listener) => listener());
  }
  async function prepare(
    source: { kind: "file" } | { kind: "url"; url: string },
    link?: string,
  ) {
    if (state.busy || state.openingRecord) return;
    const request = ++serial;
    const id = crypto.randomUUID();
    activeId = id;
    update({
      busy: "preparing",
      error: "",
      progress: {
        stage: source.kind === "file" ? "selecting" : "connecting",
        startedAt: Date.now(),
      },
    });
    const onProgress = progressChannel(request);
    try {
      const video = await invoke<AnalysisVideo | null>(
        "prepare_video_analysis",
        { id, source, onProgress },
      );
      if (request !== serial) {
        release(id);
        return;
      }
      if (video) {
        release(state.video?.id);
        update({
          video,
          link,
          work: undefined,
          result: undefined,
          recordId: undefined,
          recordError: undefined,
          restoredModelId: undefined,
        });
      }
    } catch (error) {
      if (request === serial) update({ error: String(error) });
      release(id);
    } finally {
      if (request === serial) {
        activeId = undefined;
        update({ busy: null, progress: undefined, openingRecord: false });
      }
    }
  }
  return {
    getSnapshot: () => state,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    selectWork(work: BrowseWork) {
      if (state.work?.id === work.id) return;
      clear();
      update({ work });
    },
    previewFile(file: File) {
      if (state.busy || state.openingRecord) return;
      if (!file.size || file.size > 1024 ** 3) {
        update({ error: "文件为空或超过 1 GiB" });
        return;
      }
      clear();
      const id = crypto.randomUUID();
      const path = URL.createObjectURL(file);
      previews.set(id, path);
      update({
        video: {
          id,
          name: file.name,
          path,
          duration: 0,
          width: 0,
          height: 0,
          hasAudio: false,
        },
      });
    },
    previewMetadata(
      id: string,
      duration: number,
      width: number,
      height: number,
    ) {
      if (state.video?.id !== id || !Number.isFinite(duration)) return;
      update({ video: { ...state.video, duration, width, height } });
    },
    pickFile: () => prepare({ kind: "file" }),
    async loadUrl(raw: string) {
      try {
        const url = videoLink(raw);
        await prepare({ kind: "url", url }, url);
      } catch (error) {
        update({ error: String(error) });
      }
    },
    async analyze(
      modelId: string,
      question: string,
      language: string,
      modelName = "",
    ) {
      if (
        state.busy ||
        state.openingRecord ||
        state.video?.missing ||
        (!state.work && !state.video)
      )
        return;
      const request = ++serial;
      const context = {
        title: state.work?.title ?? state.video?.name ?? "",
        workId: state.work?.id,
        sourceUrl: state.work?.source ?? state.link,
        modelName,
      };
      let video = state.video;
      const id = video?.id ?? crypto.randomUUID();
      activeId = id;
      update({
        busy: video ? "analyzing" : "preparing",
        error: "",
        recordError: undefined,
        progress: {
          stage: video ? "encoding" : "connecting",
          startedAt: Date.now(),
        },
      });
      try {
        if (!video && state.work) {
          video =
            (await invoke<AnalysisVideo | null>("prepare_video_analysis", {
              id,
              onProgress: progressChannel(request),
              source: { kind: "url", url: workVideoLink(state.work) },
            })) ?? undefined;
          if (request !== serial) {
            release(id);
            return;
          }
          if (!video)
            throw new Error("未读取到参考视频，请重试或添加原视频文件");
          update({
            video,
            busy: "analyzing",
            progress: {
              stage: "encoding",
              startedAt: state.progress!.startedAt,
            },
          });
        }
        const outcome = await invoke<AnalysisOutcome>(
          "analyze_reference_video",
          {
            id,
            modelId,
            context,
            question,
            language,
            onProgress: progressChannel(request),
          },
        );
        if (request === serial)
          update({
            result: outcome.result,
            restoredModelId: modelId,
            recordId: outcome.recordId ?? undefined,
            recordError: outcome.saveError ?? undefined,
          });
      } catch (error) {
        if (request === serial) update({ error: String(error) });
        if (!video) release(id);
      } finally {
        if (request === serial) {
          activeId = undefined;
          update({ busy: null, progress: undefined, openingRecord: false });
        }
      }
    },
    listRecords: (query = "", offset = 0) =>
      invoke<RecordPage>("list_video_analysis_records", { query, offset }),
    async openRecord(id: string) {
      if (state.busy || state.openingRecord) return false;
      const request = ++serial;
      update({ openingRecord: true, error: "" });
      try {
        const { record, video } = await invoke<OpenedRecord>(
          "open_video_analysis_record",
          { id },
        );
        if (request !== serial) {
          release(video?.id);
          return false;
        }
        release(state.video?.id);
        update({
          video: video ?? {
            ...record.video,
            path: "",
            id: `missing:${record.id}`,
            missing: true,
          },
          result: record.result,
          question: record.question,
          recordId: record.id,
          restoredModelId: record.modelId,
          recordError: undefined,
          work: browseWorks.find((work) => work.id === record.context.workId),
          link: record.context.sourceUrl ?? undefined,
        });
        return true;
      } catch (error) {
        if (request === serial) update({ error: String(error) });
        return false;
      } finally {
        if (request === serial) update({ openingRecord: false });
      }
    },
    setQuestion: (question: string) => update({ question }),
    cancel,
    clear,
    dispose() {
      clear();
      listeners.clear();
    },
  };
}
export type AnalysisController = ReturnType<typeof createAnalysisController>;
