import { expect, test } from "bun:test";
import { conceptScriptTask, exploreConceptsTask } from "./conceptTasks";

test("concept exploration stays in concept planning and preserves user choices", () => {
  const task = exploreConceptsTask();
  expect(task.agentId).toBe("concept");
  expect(task.text).toContain("8–12");
  expect(task.text).toContain("保留用户已选方向");
  expect(task.text).toContain("truth");
  expect(task.text).toContain("brandIdea");
  expect(task.text).toContain("格式");
  expect(task.text).toContain("不要写完整脚本");
});

test("script handoff reads the exact selected card and updates an existing screenplay", () => {
  const task = conceptScriptTask("direction-2", "existing-script");
  expect(task.agentId).toBe("writer");
  expect(task.targetNodeId).toBe("existing-script");
  expect(task.text).toContain('"direction-2"');
  expect(task.text).toContain("mstudio_read_concepts");
  expect(task.text).toContain("更新原稿并保留 ID");
  expect(task.text).toContain("该卡已删除或不再是已选中");
  expect(task.text).toContain("本轮只写脚本");
  expect(conceptScriptTask("direction-1").targetNodeId).toBeUndefined();
});
