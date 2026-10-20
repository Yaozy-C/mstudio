import type { BrowseWork } from "../browseWorks";

export type AnalysisVideo = {
  missing?: boolean;
  id: string;
  name: string;
  path: string;
  duration: number;
  width: number;
  height: number;
  hasAudio: boolean;
};
export type AnalysisSegment = {
  start: number;
  end: number;
  title: string;
  visual: string;
  speech: string;
  onScreenText: string;
  sound: string;
  purpose: string;
};
export type VideoAnalysisResult = {
  summary: string;
  access: string;
  segments: AnalysisSegment[];
  craft: Record<"idea" | "visuals" | "camera" | "language" | "sound", string>;
};
export type ProgressUpdate = {
  fragments?: boolean;
  stage:
    | "selecting"
    | "connecting"
    | "downloading"
    | "copying"
    | "checking"
    | "encoding"
    | "analyzing"
    | "formatting"
    | "saving";
  completed?: number | null;
  total?: number | null;
};
export type AnalysisProgress = ProgressUpdate & { startedAt: number };
export type RecordContext = {
  title: string;
  workId?: string | null;
  sourceUrl?: string | null;
  modelName: string;
};
export type AnalysisRecord = {
  id: string;
  createdAt: number;
  context: RecordContext;
  video: AnalysisVideo;
  question: string;
  modelId: string;
  language: string;
  result: VideoAnalysisResult;
};
export type RecordSummary = {
  id: string;
  title: string;
  createdAt: number;
  duration: number;
  modelName: string;
  question: string;
  summary: string;
  poster?: string | null;
  missingVideo: boolean;
};
export type RecordPage = { items: RecordSummary[]; total: number };
export type AnalysisOutcome = {
  result: VideoAnalysisResult;
  recordId?: string | null;
  saveError?: string | null;
};
export type OpenedRecord = {
  record: AnalysisRecord;
  video: AnalysisVideo | null;
};
export type AnalysisState = {
  recordId?: string;
  recordError?: string;
  restoredModelId?: string;
  openingRecord?: boolean;
  progress?: AnalysisProgress;
  work?: BrowseWork;
  link?: string;
  video?: AnalysisVideo;
  result?: VideoAnalysisResult;
  busy: "preparing" | "analyzing" | null;
  error: string;
  question: string;
};
export function timecode(seconds: number) {
  const value = Math.max(0, Math.floor(seconds));
  return `${Math.floor(value / 60)
    .toString()
    .padStart(2, "0")}:${(value % 60).toString().padStart(2, "0")}`;
}
export function videoLink(raw: string) {
  const value = raw.trim();
  const parsed = new URL(value);
  if (
    !["http:", "https:"].includes(parsed.protocol) ||
    parsed.username ||
    parsed.password
  )
    throw new Error("请输入有效的 HTTP 或 HTTPS 视频链接");
  return parsed.href;
}
export function workVideoLink(work: BrowseWork) {
  const player = new URL(work.embed);
  if (
    player.origin !== "https://player.mux.com" ||
    !/^\/[a-zA-Z0-9]+$/.test(player.pathname)
  )
    throw new Error("这部作品暂不支持直接读取，请添加原视频文件或链接");
  return `https://stream.mux.com${player.pathname}.m3u8`;
}

export type AnalysisInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;
