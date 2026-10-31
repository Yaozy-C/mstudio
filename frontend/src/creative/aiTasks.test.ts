import { expect, test } from "bun:test";
import { creativeTask } from "./aiTasks";

test("writing uses available source readers before asking for another attachment", () => {
  const task = creativeTask("write");
  expect(task.agentId).toBe("writer");
  expect(task.text).toContain("已分配的素材读取工具");
  expect(task.text).toContain("素材与资产可同时存在");
  expect(task.text).toContain("没有本地文件路径也不代表无法读取");
  expect(task.text).not.toContain("请我用 @ 补充");
  expect(creativeTask("split").text).toContain("脚本和已确认资产");
});
