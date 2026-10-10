import { t, useLanguage } from "../i18n";
export const defaultHttpMapping = { outputPointer: "/data/0/url" };
export function HttpModelFields({
  mapping,
  setMapping,
}: {
  mapping: string;
  setMapping: (v: string) => void;
}) {
  useLanguage();
  return (
    <>
      <label className="model-field-wide">
        {t("响应与队列映射")}
        <textarea
          rows={8}
          spellCheck={false}
          value={mapping}
          onChange={(e) => setMapping(e.target.value)}
        />
        <small>
          {t(
            "outputPointer 指向结果 URL 或 URL 数组；对象数组可另填 itemUrlPointer。使用 JSON Pointer，例如 /data/0/url。厂商要求固定请求头时用 headers 声明，例如阿里云百炼异步视频接口的 X-DashScope-Async 需填 enable。",
          )}
        </small>
      </label>
      <details className="model-field-wide">
        <summary>{t("异步接口配置示例")}</summary>
        <pre className="request-preview">
          {JSON.stringify(
            {
              headers: { "X-DashScope-Async": "enable" },
              idPointer: "/output/task_id",
              pollUrl: "/api/v1/tasks/{id}",
              statusPointer: "/output/task_status",
              doneValue: "SUCCEEDED",
              failedValues: ["FAILED", "CANCELED", "UNKNOWN"],
              outputPointer: "/output/video_url",
            },
            null,
            2,
          )}
        </pre>
        <p className="model-hint">
          {t(
            "轮询使用 GET，地址须与提交接口同源。结果在另一接口时填写 resultUrl，例如 /tasks/{id}/result。headers 会附加到提交、轮询和结果请求，最多 8 个；Authorization、Host 等由服务连接管理，不能在此声明。取消任务需在服务端操作。当前支持 URL 媒体结果。",
          )}
        </p>
      </details>
    </>
  );
}
