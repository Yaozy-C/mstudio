export const tools = [
  {
    id: "material-read",
    name: "读取创作素材",
    description:
      "读取来源照片、产品资料和参考视频目录，供创意、资产整理和编剧使用",
    kind: "项目工具",
  },
  {
    id: "project-assets",
    name: "整理镜头资产",
    description:
      "维护镜头资产参考记录；配合媒体生成准备候选图片，素材收录与资产确认独立",
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
    description: "读取画布文本、素材目录、尺寸时长、任务状态和项目历史",
    kind: "项目工具",
  },
  {
    id: "media-read",
    name: "读取媒体内容",
    description:
      "按资源 ID 读取图片内容、视频帧和历史媒体；发现素材或资产目录由对应读取工具负责，直接发送的附件不受此开关影响",
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
    id: "project-concepts",
    name: "维护创意板",
    description: "创建、比较、淘汰和选择项目创意方向卡片",
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
