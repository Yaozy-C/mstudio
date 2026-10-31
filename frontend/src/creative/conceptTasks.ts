import { creativeTask, type CreativeTask } from "./aiTasks";

export function exploreConceptsTask(): CreativeTask {
  return {
    agentId: "concept",
    text: "请读取 creative-concepts Skill、项目目标和现有创意板，结合我本轮提供的资料探索创意方向。方向开放时先保存 8–12 个粗案，按淘汰标准筛选，再开发 3 个方向并给出一个有理由的推荐。每张卡必须写 truth（去掉产品仍成立、具体而不舒服的人性观察）和 brandIdea（品牌的世界观，不是功能或定位）；格式（ASMR、POV、pack-with-me、three-ways、before/after 等）不是概念。三个开发方向必须来自不同的轴，不能是同一种格式或同一种权力关系。沿用已有卡片 ID，保留用户已选方向和明确约束；淘汰保留记录，只有我明确要求删除时才删除。不要写完整脚本、拆镜头或生成媒体。",
    refs: [],
  };
}

export function conceptScriptTask(
  conceptId: string,
  screenplayId?: string,
): CreativeTask {
  return creativeTask(
    "write",
    screenplayId,
    undefined,
    `我已选定创意卡 ID ${JSON.stringify(conceptId)}。请用 mstudio_read_concepts 按这个 ID 读取最新完整卡片，以它作为本轮脚本依据，保留核心事件、产品证据、最后一句和项目中已确认的市场、语言、时长等约束，不重新比较或擅自更换方向。若该卡已删除或不再是已选中状态，请先说明，不要根据旧内容继续写。${screenplayId ? "本项目已有脚本，请更新原稿并保留 ID，不要新建第二份脚本。" : "没有脚本时创建一份，已有脚本时更新原稿。"}`,
  );
}
