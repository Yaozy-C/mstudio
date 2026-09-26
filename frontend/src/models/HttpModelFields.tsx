export const defaultHttpMapping = { outputPointer: "/data/0/url" };
export function HttpModelFields({
  mapping,
  setMapping,
}: {
  mapping: string;
  setMapping: (v: string) => void;
}) {
  return (
    <>
      <label className="model-field-wide">
        响应与队列映射
        <textarea
          rows={8}
          spellCheck={false}
          value={mapping}
          onChange={(e) => setMapping(e.target.value)}
        />
        <small>
          outputPointer 指向结果 URL 或 URL 数组；对象数组可另填
          itemUrlPointer。使用 JSON Pointer，例如 /data/0/url。
        </small>
      </label>
      <details className="model-field-wide">
        <summary>异步接口配置示例</summary>
        <pre className="request-preview">
          {JSON.stringify(
            {
              idPointer: "/id",
              pollUrl: "/tasks/{id}",
              statusPointer: "/status",
              doneValue: "completed",
              failedValues: ["failed", "cancelled"],
              outputPointer: "/output/url",
            },
            null,
            2,
          )}
        </pre>
        <p className="model-hint">
          轮询使用 GET，地址须与提交接口同源。结果在另一接口时填写
          resultUrl，例如
          /tasks/&#123;id&#125;/result。取消任务需在服务端操作。当前支持 URL
          媒体结果。
        </p>
      </details>
    </>
  );
}
