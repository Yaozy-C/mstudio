export const tools = [
  {
    id: "project-assets",
    name: "整理项目素材",
    description: "维护素材参考记录；配合媒体生成准备独立图片资产",
    kind: "项目工具",
  },
  {
    id: "agent-delegate",
    name: "委派专业 Agent",
    description:
      "将明确任务交给已启用的专业角色；支持独立或继承历史的子 Agent，以及后台继续、消息和中断",
    kind: "协作工具",
  },
  {
    id: "project-read",
    name: "读取项目",
    description: "读取画布、素材和项目历史",
    kind: "项目工具",
  },
  {
    id: "project-edit",
    name: "编辑项目（完整权限）",
    description: "编辑项目全部内容；依赖读取项目，生成准备还需媒体生成权限",
    kind: "项目工具",
  },
  {
    id: "project-brief",
    name: "维护项目目标",
    description: "维护整片目标与保留约束",
    kind: "项目工具",
  },
  {
    id: "project-script",
    name: "编辑脚本与创意",
    description: "创建和修改结构化声画脚本与段落，保留镜头关联",
    kind: "项目工具",
  },
  {
    id: "project-shots",
    name: "编辑分镜",
    description: "创建和修改镜头动作、台词与参考",
    kind: "项目工具",
  },
  {
    id: "project-frames",
    name: "绘制分镜图",
    description:
      "编辑图片描述、画格与参考；配合媒体生成仅可生成图片，不可改镜头叙事",
    kind: "项目工具",
  },
  {
    id: "project-production",
    name: "准备镜头制作",
    description: "编写生成提示词与参考；在对话中执行生成还需媒体生成权限",
    kind: "项目工具",
  },
  {
    id: "project-timeline",
    name: "编辑时间线与声音",
    description: "选用镜头素材、编排多轨与字幕",
    kind: "项目工具",
  },
  {
    id: "media-generation",
    name: "媒体生成",
    description:
      "使用模型目录中已启用的图像、视频和音频模型；遵循你选择的先确认或自动执行方式",
    kind: "项目工具",
  },
];
