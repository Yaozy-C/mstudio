import "../styles/creative-next-step.css";
export function ScriptNextStep({
  hasScript,
  count,
  next,
}: {
  hasScript: boolean;
  count: number;
  next: () => void;
}) {
  if (!hasScript) return null;
  return (
    <section className="creative-next-step" aria-label="脚本下一步">
      <div>
        <small>下一步</small>
        <h2>设计整片镜头</h2>
        <p>
          {count
            ? `已有 ${count} 个镜头，继续检查镜头设计、制作分镜图。`
            : "先设计整片的观看顺序、节奏与切点，再确定镜头和关键画格。"}
        </p>
      </div>
      <button className="primary" onClick={next}>
        {count ? "调整整片镜头" : "根据完整脚本设计镜头"}
      </button>
    </section>
  );
}
