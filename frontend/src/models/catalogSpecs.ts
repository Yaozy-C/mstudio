import { multimodalSpecs } from "./multimodalSpecs";
import type { ModelConnection } from "./types";
export type OutputKind = "text" | "image" | "video";
export type ModelSpec = {
  id: string;
  name: string;
  provider: string;
  kind: OutputKind;
  inputs: string[];
  outputs: string[];
  note: string;
  source: string;
  protocol: string;
  request: Record<string, unknown>;
  response: string;
  connection?: Pick<
    ModelConnection,
    "endpoint" | "adapter" | "model" | "inputs"
  >;
  endpoint?: string;
};
const chat = (model: string) => ({
  model,
  messages: [
    {
      role: "user",
      content: [
        { type: "text", text: "解释这张图" },
        {
          type: "image_url",
          image_url: { url: "https://example.com/image.png" },
        },
      ],
    },
  ],
});
export const modelLibrary: ModelSpec[] = [
  {
    id: "codex-image",
    name: "GPT Image 2.5 · Codex（型号未验证）",
    provider: "OpenAI / 本机 Codex",
    kind: "image",
    inputs: ["文本", "最多 5 张参考图"],
    outputs: ["PNG"],
    note: "使用 Codex 原生生图。底层型号由 Codex 决定，无法保证或选择 Image 2.5。",
    source: "https://learn.chatgpt.com/docs/app-server",
    protocol: "Codex App Server",
    endpoint: "codex://local/images",
    request: { model: "codex-image" },
    response: "data[].b64_json",
  },
  {
    id: "gemini-image-direct",
    name: "Gemini 3.1 Flash Image",
    provider: "Google / 官方直连",
    kind: "image",
    inputs: ["文本", "图像"],
    outputs: ["图像"],
    note: "Google 官方连接，无需 fal",
    source: "https://ai.google.dev/gemini-api/docs/image-generation",
    protocol: "Gemini 原生",
    request: { model: "gemini-3.1-flash-image" },
    response: "inlineData",
    endpoint: "https://generativelanguage.googleapis.com",
  },

  {
    id: "deepseek-flash",
    name: "DeepSeek Flash",
    provider: "DeepSeek",
    kind: "text",
    inputs: ["文本", "图像 · JPEG / PNG / GIF / WebP"],
    outputs: ["文本", "工具调用"],
    note: "图片可用 URL、Base64 或 Files API。图片文件不等于任意文档；当前 Agent 接收文字和项目图片。",
    source: "https://api-docs.deepseek.com/guides/vision/",
    protocol: "Chat Completions / Responses",
    request: chat("deepseek-flash"),
    response: "choices[].message.content / tool_calls",
    connection: {
      endpoint: "https://api.deepseek.com",
      adapter: "openai-compatible",
      model: "deepseek-flash",
      inputs: { image: true, audio: false, video: false, document: false },
    },
  },
  ...multimodalSpecs,
  {
    id: "minimax-m3",
    name: "MiniMax M3",
    provider: "MiniMax",
    kind: "text",
    inputs: ["文本", "图像"],
    outputs: ["文本", "工具调用"],
    note: "通过 OpenAI 兼容接口连接。",
    source: "https://platform.minimax.io/docs/api-reference/text-openai-api",
    protocol: "Chat Completions",
    request: { model: "MiniMax-M3" },
    response: "choices[].message",
    connection: {
      endpoint: "https://api.minimax.io/v1",
      adapter: "openai-compatible",
      model: "MiniMax-M3",
      inputs: { image: true, audio: false, video: false, document: false },
    },
  },
  ...(["sunburst", "flare"] as const).map((variant): ModelSpec => ({
    id: `gpt-image-2.5-${variant}`,
    name: `GPT Image 2.5 · ${variant === "sunburst" ? "Sunburst" : "Flare"}`,
    provider: "OpenAI",
    kind: "image",
    inputs: ["文本", "图像 · 编辑接口"],
    outputs: ["PNG / JPEG / WebP"],
    note: "2026-09-08 发布。文字生成与图片编辑使用不同端点；编辑需要 image_urls。通过 fal 接入，非 OpenAI 文本连接。",
    source: `https://fal.ai/models/openai/gpt-image-2.5/${variant}/text-to-image/api`,
    protocol: "fal Queue（Images 原生协议另有格式）",
    endpoint: `openai/gpt-image-2.5/${variant}/text-to-image`,
    request: { prompt: "设计一张海报", num_images: 1, output_format: "png" },
    response: "images[].url / content_type / width / height",
  })),
  {
    id: "nano-banana-2",
    name: "Nano Banana 2",
    provider: "Google",
    kind: "image",
    inputs: ["文本", "图像 · 编辑接口"],
    outputs: ["图像"],
    note: "原生型号 gemini-3.1-flash-image。此处使用 fal 图片接口；编辑模式使用 image_urls，不能把所有 Banana 版本的输入能力混在一起。",
    source: "https://fal.ai/models/fal-ai/nano-banana-2/edit/api",
    protocol: "fal Queue / Gemini 原生",
    endpoint: "fal-ai/nano-banana-2",
    request: { prompt: "创作产品主视觉", num_images: 1 },
    response: "images[].url",
  },
  {
    id: "midjourney",
    name: "Midjourney · MJ",
    provider: "Midjourney",
    kind: "image",
    inputs: ["文本", "图像提示", "风格参考"],
    outputs: ["图像"],
    note: "官方未提供通用公开 API。保留模型能力资料，暂不提供假接入或第三方冒充的官方协议。",
    source:
      "https://docs.midjourney.com/hc/en-us/articles/32013696484109-Community-Guidelines",
    protocol: "官方网页 / Discord",
    request: { prompt: "<图片 URL> 产品视觉 --ar 16:9" },
    response: "官方界面中的图像结果；无公开 JSON API 合约",
  },
  {
    id: "h3-max",
    name: "MiniMax H3 Max",
    provider: "MiniMax / fal",
    kind: "video",
    inputs: ["文本", "图像 · 首帧"],
    outputs: ["视频"],
    note: "2026 年 8 月发布的 fal 后训练版本。此预设支持 image_url 首帧；不提供图片时转为文字生成；不要套用 H3 参考视频接口的参数。",
    source: "https://fal.ai/models/minimax/h3-max/image-to-video/api",
    protocol: "fal Queue",
    endpoint: "minimax/h3-max/image-to-video",
    request: { prompt: "镜头缓慢推进" },
    response: "video.url",
  },
  {
    id: "h3",
    name: "MiniMax H3",
    provider: "MiniMax",
    kind: "video",
    inputs: ["文本", "图像 / 视频 / 音频 · 按任务端点"],
    outputs: ["带声音的视频"],
    note: "2026-07-31 发布。模型支持多模态上下文；文字、参考生成等任务分别配置，不把家族能力当作单个端点的能力。",
    source: "https://www.minimax.io/blog/minimax-h3",
    protocol: "fal Queue",
    endpoint: "minimax/h3/text-to-video",
    request: { prompt: "一段产品展示镜头", duration: 5 },
    response: "video.url",
  },
  ...(["image-to-video", "reference-to-video"] as const).map(
    (mode): ModelSpec => ({
      id: `h3-${mode}`,
      name:
        mode === "image-to-video"
          ? "MiniMax H3 · 首尾帧"
          : "MiniMax H3 · 全参考",
      provider: "MiniMax / fal",
      kind: "video",
      inputs:
        mode === "image-to-video"
          ? ["文本", "首帧", "尾帧", "固定音轨"]
          : ["文本", "参考图片", "参考视频", "参考音频"],
      outputs: ["带声音的视频"],
      note:
        mode === "image-to-video"
          ? "支持首帧、尾帧或首尾帧组合，固定音轨可选。"
          : "图片最多 9 张，视频、音频各 3 段；合计最多 12 个。视频、音频每段 2–15 秒，各类总时长最多 15 秒。",
      source: `https://fal.ai/models/minimax/h3/${mode}/api`,
      protocol: "fal Queue",
      endpoint: `minimax/h3/${mode}`,
      request: { prompt: "产品展示", duration: 5 },
      response: "video.url",
    }),
  ),
  {
    id: "seedance-2.5",
    name: "Seedance 2.5",
    provider: "ByteDance",
    kind: "video",
    inputs: ["文本", "图像 / 视频 / 音频 · 参考模式"],
    outputs: ["视频"],
    note: "近期可用的 2.5 系列。文字生成、图生视频、多模态参考使用独立端点；当前预设为文字生成。参考模式支持最多 50 个混合参考，仍须遵守各类型限制。",
    source: "https://fal.ai/seedance-2.5",
    protocol: "fal Queue",
    endpoint: "bytedance/seedance-2.5/text-to-video",
    request: { prompt: "一个连续的产品展示镜头" },
    response: "video.url",
  },
  {
    id: "omni",
    name: "Gemini Omni 1.1 Flash",
    provider: "Google",
    kind: "video",
    inputs: ["文本", "图像", "视频 · 编辑 / 延长 ≤10s"],
    outputs: ["视频 · 3–10s"],
    note: "官方型号页更新于 2026 年 8 月。以型号页公开的输入为准；不能因 Omni 家族宣传就假定此接口接收音频。原生适配器待接入。",
    source: "https://ai.google.dev/gemini-api/docs/models/gemini-omni-flash",
    protocol: "Gemini generateContent",
    request: {
      model: "gemini-omni-1.1-flash",
      contents: [{ parts: [{ text: "生成一个产品镜头" }] }],
    },
    response: "异步视频生成结果；按原生任务协议读取",
  },
  {
    id: "veo",
    name: "Veo 3.1",
    provider: "Google",
    kind: "video",
    inputs: ["文本", "首帧 / 尾帧", "参考图", "视频 · 延长"],
    outputs: ["视频 + 原生音频"],
    note: "不同任务对分辨率、时长和参考数有约束；原生 API 使用异步 operation，不是文本对话返回。原生适配器待接入。",
    source: "https://ai.google.dev/gemini-api/docs/veo",
    protocol: "predictLongRunning",
    request: {
      instances: [{ prompt: "生成产品展示视频" }],
      parameters: { aspectRatio: "16:9" },
    },
    response:
      "operation → response.generateVideoResponse.generatedSamples[].video",
  },
];
