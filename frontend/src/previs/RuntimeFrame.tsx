import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { bridge } from "../bridge";
import { t } from "../i18n";
import { fileIdentity, type PrevisFile } from "./types";
export type RuntimeHandle = {
  frame: (time: number) => Promise<ArrayBuffer>;
  session: () => string;
  revision: () => number;
  seek: (time: number) => void;
  play: (playing: boolean) => void;
};
type Session = {
  sessionId: string;
  url: string;
  signature: string;
  fileRevision: number;
};
export const RuntimeFrame = forwardRef<
  RuntimeHandle,
  {
    projectId: string;
    file: PrevisFile;
    files: PrevisFile[];
    onReady: (ready: boolean) => void;
    onTime: (time: number) => void;
    onError: (error: string) => void;
  }
>(function RuntimeFrame(
  { projectId, file, files, onReady, onTime, onError },
  ref,
) {
  const iframe = useRef<HTMLIFrameElement>(null);
  const current = useRef<Session | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const callbacks = useRef({ onReady, onTime, onError });
  callbacks.current = { onReady, onTime, onError };
  const pending = useRef(
    new Map<
      string,
      {
        resolve: (a: ArrayBuffer) => void;
        reject: (e: Error) => void;
        timer: ReturnType<typeof setTimeout>;
      }
    >(),
  );
  const identity = fileIdentity(file, files);
  const post = (type: string, extra: Record<string, unknown> = {}) => {
    const s = current.current;
    if (s)
      iframe.current?.contentWindow?.postMessage(
        { type, channel: s.sessionId, ...extra },
        new URL(s.url).origin,
      );
  };
  useImperativeHandle(ref, () => ({
    frame(time) {
      return new Promise((resolve, reject) => {
        if (!current.current) {
          reject(Error("预演尚未加载"));
          return;
        }
        const id = crypto.randomUUID();
        const timer = setTimeout(() => {
          pending.current.delete(id);
          reject(Error("预演帧渲染超时，请检查代码或重新加载"));
        }, 15000);
        pending.current.set(id, { resolve, reject, timer });
        post("frame", { time, requestId: id });
      });
    },
    session: () => current.current?.sessionId || "",
    revision: () => current.current?.fileRevision || 0,
    seek: (time) => post("seek", { time }),
    play: (playing) => post("play", { playing }),
  }));
  useEffect(() => {
    let active = true;
    let opened: Session | null = null;
    callbacks.current.onReady(false);
    setSession(null);
    current.current = null;
    if (!file.code.trim()) return;
    bridge<Session>("open_previs_runtime", { projectId, fileId: file.id })
      .then((s) => {
        opened = s;
        if (active) {
          current.current = s;
          setSession(s);
        } else void bridge("close_previs_runtime", { sessionId: s.sessionId });
      })
      .catch((e) => {
        if (active) callbacks.current.onError(String(e));
      });
    return () => {
      active = false;
      for (const p of pending.current.values()) {
        clearTimeout(p.timer);
        p.reject(Error("预演环境已重新加载"));
      }
      pending.current.clear();
      if (opened) {
        post("dispose");
        void bridge("close_previs_runtime", {
          sessionId: opened.sessionId,
        }).catch(() => {});
      }
      current.current = null;
    };
    // Source and model revisions form one frozen runtime identity.
  }, [projectId, identity, file.code]);
  useEffect(() => {
    const receive = (e: MessageEvent) => {
      const s = current.current;
      if (
        !s ||
        e.source !== iframe.current?.contentWindow ||
        e.origin !== new URL(s.url).origin ||
        e.data?.channel !== s.sessionId
      )
        return;
      if (e.data.type === "ready") callbacks.current.onReady(true);
      if (e.data.type === "time") callbacks.current.onTime(e.data.time);
      if (e.data.type === "error") {
        callbacks.current.onReady(false);
        callbacks.current.onError(e.data.message);
      }
      const p = pending.current.get(e.data.requestId);
      if (!p) return;
      if (e.data.type === "frame" && e.data.bytes instanceof ArrayBuffer) {
        clearTimeout(p.timer);
        pending.current.delete(e.data.requestId);
        p.resolve(e.data.bytes);
      }
      if (e.data.type === "frame-error") {
        clearTimeout(p.timer);
        pending.current.delete(e.data.requestId);
        p.reject(Error(e.data.message));
      }
    };
    window.addEventListener("message", receive);
    return () => window.removeEventListener("message", receive);
  }, []);
  return session ? (
    <iframe
      ref={iframe}
      key={session.sessionId}
      src={session.url}
      title={t("三维预演")}
      sandbox="allow-scripts allow-same-origin"
      referrerPolicy="no-referrer"
      className="previs-frame"
    />
  ) : (
    <div className="previs-placeholder">
      {file.code.trim()
        ? t("正在加载三维场景…")
        : t("描述镜头或添加模型，开始预演。")}
    </div>
  );
});
