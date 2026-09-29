import {
  ArrowRight,
  ArrowUpRight,
  PencilSimple,
  ChatText,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { ActionButton } from "../ui/ActionButton";
import { requestCreativeTask } from "./aiTasks";
import "../styles/script-start.css";

const steps = [
  ["脚本", "告诉 Agent 你想做什么视频，一起确定内容、画面和台词。"],
  ["制作画布", "把脚本拆成镜头，生成图片、视频，或使用你已有的素材。"],
  ["成片", "把素材加入时间线，调整顺序、声音与字幕，预览后导出。"],
];
const examples = [
  "帮我写一支 30 秒的商品介绍视频，突出使用场景和产品特点。",
  "我有一组旅行素材，想剪成 1 分钟的短片，先帮我梳理脚本。",
];
export function ScriptAIStart({
  screenplayId,
  manual,
}: {
  screenplayId?: string;
  manual: () => void;
}) {
  useLanguage();
  const start = (text = "") =>
    requestCreativeTask({
      text,
      agentId: "concept",
      targetNodeId: screenplayId,
    });
  return (
    <section className="script-onboarding" aria-label={t("开始创作")}>
      <div className="script-onboarding-intro">
        <h2>{t("从一个想法，开始你的第一支视频")}</h2>
        <p>{t("告诉 Agent 视频主题、用途和大致时长，从脚本开始一起完成。")}</p>
        <div className="script-onboarding-actions">
          <button className="primary" onClick={() => start()}>
            <ChatText size={20} aria-hidden="true" />
            {t("和 Agent 开始创作")}
            <ArrowRight size={18} aria-hidden="true" />
          </button>
          <ActionButton icon={PencilSimple} onClick={manual}>
            {t("手动写作")}
          </ActionButton>
        </div>
      </div>
      <ol className="script-onboarding-steps" aria-label={t("创作流程")}>
        {steps.map(([title, description], index) => (
          <li key={title}>
            <span className="script-step-number" aria-hidden="true">
              0{index + 1}
            </span>
            <h3>{t(title)}</h3>
            <p>{t(description)}</p>
          </li>
        ))}
      </ol>
      <div className="script-onboarding-examples">
        <div className="script-example-heading">
          <h3>{t("可以这样开始")}</h3>
          <span>{t("点击示例，修改后发送")}</span>
        </div>
        {examples.map((example) => (
          <button key={example} onClick={() => start(t(example))}>
            <span>{t(example)}</span>
            <ArrowUpRight size={20} aria-hidden="true" />
          </button>
        ))}
        <p className="script-onboarding-tip">
          {t("有参考资料？在「素材」中上传，再用对话框的 @ 引用给 Agent。")}
        </p>
      </div>
    </section>
  );
}
