import type { Dispatch, SetStateAction } from "react";

export type Entry = { key: string; text: string };
export const settle = async () => {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => setTimeout(resolve, 120));
};
export async function checkConversation(
  viewport: HTMLDivElement,
  update: Dispatch<SetStateAction<Entry[]>>,
) {
  const checks: string[] = [];
  const check = (condition: unknown, label: string) => {
    if (!condition) throw new Error(label);
    checks.push(`PASS ${label}`);
  };
  const row = (key: string) =>
    viewport.querySelector<HTMLElement>(`[data-conversation-key="${key}"]`);
  const bottom = () =>
    viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop;
  const scroll = async (top: number) => {
    viewport.scrollTo({ top, behavior: "instant" });
    viewport.dispatchEvent(new Event("scroll"));
    await settle();
    await settle();
  };
  await settle();
  check(bottom() <= 2 && row("message-199"), "首次打开定位最新消息");
  check(
    viewport.querySelectorAll("[data-conversation-key]").length < 12,
    "200 条历史只挂载可见消息和缓冲区",
  );
  check(!viewport.querySelector("pre"), "折叠工具不生成参数或结果代码块");
  await scroll(0);
  check(row("message-0"), "可以滚动访问最早历史");
  const disclosure =
    row("message-0")!.querySelector<HTMLDetailsElement>("details")!;
  disclosure.querySelector<HTMLElement>("summary")!.click();
  await settle();
  check(
    disclosure.open && disclosure.querySelectorAll("pre").length === 2,
    "展开工具后显示完整调用和结果",
  );
  disclosure.querySelector<HTMLElement>("summary")!.click();
  await settle();
  check(
    !disclosure.open && !disclosure.querySelector("pre"),
    "收起工具立即释放详情节点",
  );
  disclosure.querySelector<HTMLElement>("summary")!.click();
  await settle();
  const child = row("message-0")!.querySelector<HTMLDetailsElement>(
    ".agent-child-activity details",
  )!;
  check(
    !child.querySelector(".agent-activity-item"),
    "折叠子任务不挂载工具与思考详情",
  );
  child.querySelector<HTMLElement>("summary")!.click();
  await settle();
  check(
    child.querySelector(".agent-activity-item"),
    "展开子任务可查看原有执行记录",
  );
  await scroll(viewport.scrollHeight);
  check(!row("message-0"), "离屏历史卸载");
  await scroll(0);
  check(
    row("message-0")!.querySelector<HTMLDetailsElement>("details")!.open,
    "滚回历史保留工具展开状态",
  );
  check(
    row("message-0")!.querySelector<HTMLDetailsElement>(
      ".agent-child-activity details",
    )!.open,
    "滚回历史保留子任务展开状态",
  );
  const focused = row("message-0")!.querySelector<HTMLElement>("summary")!;
  focused.focus();
  await scroll(viewport.scrollHeight);
  check(
    document.activeElement === focused && row("message-0"),
    "键盘焦点不因虚拟卸载丢失",
  );
  focused.blur();
  await settle();
  check(!row("message-0"), "离开焦点后释放离屏消息");
  update((entries) => [...entries, { key: "message-200", text: "新增回答" }]);
  await settle();
  await settle();
  check(bottom() <= 2 && row("message-200"), "底部阅读时跟随新增回答");
  for (let step = 0; step < 8; step++) {
    update((entries) =>
      entries.map((entry) =>
        entry.key === "message-200"
          ? { ...entry, text: entry.text + "\n\n流式输出更多内容。".repeat(4) }
          : entry,
      ),
    );
    await settle();
  }
  check(bottom() <= 2, "动态高度的流式回答持续贴底");
  await scroll(viewport.scrollTop - 180);
  const held = viewport.scrollTop;
  update((entries) =>
    entries.map((entry) =>
      entry.key === "message-200"
        ? { ...entry, text: entry.text + "流式新增内容。".repeat(50) }
        : entry,
    ),
  );
  await settle();
  check(Math.abs(viewport.scrollTop - held) <= 2, "向上阅读时流式更新不抢滚动");
  await scroll(viewport.scrollHeight / 2);
  const anchor = Array.from(
    viewport.querySelectorAll<HTMLElement>("[data-conversation-key]"),
  ).find(
    (element) =>
      element.getBoundingClientRect().bottom >
      viewport.getBoundingClientRect().top,
  )!;
  const anchorKey = anchor.dataset.conversationKey!;
  const anchorTop = anchor.getBoundingClientRect().top;
  update((entries) => [
    { key: "older-message", text: "更早的历史" },
    ...entries,
  ]);
  await settle();
  check(
    Math.abs(row(anchorKey)!.getBoundingClientRect().top - anchorTop) <= 2,
    "补入更早历史保持阅读位置",
  );
  update((entries) => [...entries, { key: "message-201", text: "后台新消息" }]);
  await settle();
  check(
    Math.abs(row(anchorKey)!.getBoundingClientRect().top - anchorTop) <= 2,
    "阅读旧历史时新增消息不抢滚动",
  );
  await scroll(viewport.scrollHeight);
  viewport.style.width = "320px";
  await settle();
  await settle();
  check(bottom() <= 2, "调整侧栏宽度后动态测量并保持贴底");
  return checks;
}
