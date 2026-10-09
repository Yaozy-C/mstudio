import { test, expect } from "bun:test";
import { creativeTask } from "./aiTasks";

test("creative tasks route each stage with an independent task target and preserve idea and scope", () => {
  const write = creativeTask("write", undefined, undefined, "A lunch bag ad");
  expect(write.agentId).toBe("writer");
  expect(creativeTask("revise-script", "p1").agentId).toBe("writer");
  expect(write.refs).toEqual([]);
  expect(write.text).toContain("A lunch bag ad");
  expect(write.text).toContain("本轮只写脚本");
  const split = creativeTask("split", "p1", "第 2 段");
  // Shot design has its own role.
  expect(split.agentId).toBe("director");
  expect(split.refs).toEqual([]);
  expect(split.targetNodeId).toBe("p1");
  expect(split.text).toContain("只处理第 2 段");
  expect(split.text).toContain("避免重复创建");
  expect(creativeTask("revise-shot", "s1").targetNodeId).toBe("s1");
  expect(creativeTask("split", "p1").refs).toEqual([]);
  expect(creativeTask("split", "p1").targetNodeId).toBe("p1");
});
