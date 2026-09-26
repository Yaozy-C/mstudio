import type { ModelSpec } from "./catalogSpecs";
export const multimodalSpecs: ModelSpec[] = [
  {
    id: "openai-text",
    name: "OpenAI · GPT-6 Astra",
    provider: "OpenAI",
    kind: "text",
    inputs: ["文本", "图像", "文件 · 按型号与文件类型"],
    outputs: ["文本", "工具调用"],
    note: "Responses 使用 input_text / input_image / input_file。当前支持原始 PDF 和 UTF-8 文本资料；连接预填 gpt-6-astra，可按账号权限调整。",
    source: "https://developers.openai.com/api/docs/guides/pdf-files",
    protocol: "Responses",
    request: {
      model: "gpt-6-astra",
      input: [
        {
          role: "user",
          content: [
            { type: "input_text", text: "总结文件" },
            { type: "input_file", file_id: "file-..." },
          ],
        },
      ],
    },
    response: "output[].content[] → output_text；function_call",
    connection: {
      endpoint: "https://api.openai.com/v1",
      adapter: "openai-responses",
      model: "gpt-6-astra",
      inputs: { image: true, audio: false, video: false, document: true },
    },
  },
  {
    id: "gemini-text",
    name: "Gemini · 多模态文本",
    provider: "Google",
    kind: "text",
    inputs: ["文本", "图像", "音频", "视频", "PDF"],
    outputs: ["文本", "工具调用"],
    note: "原生协议用 contents / parts、inlineData 或 fileData。当前使用 Gemini 原生接口传入图片、音频、视频及 PDF；单素材上限 12 MiB。",
    source: "https://ai.google.dev/gemini-api/docs/function-calling",
    protocol: "Gemini 原生 generateContent",
    request: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "总结文件" },
            {
              fileData: {
                mimeType: "application/pdf",
                fileUri: "<已上传文件 URI>",
              },
            },
          ],
        },
      ],
    },
    response: "candidates[].content.parts[].text / functionCall",
    connection: {
      endpoint: "https://generativelanguage.googleapis.com",
      adapter: "gemini-native",
      model: "gemini-3.8-flash",
      inputs: { image: true, audio: true, video: true, document: true },
    },
  },
  {
    id: "claude-native",
    name: "Claude · 原生多模态",
    provider: "Anthropic",
    kind: "text",
    inputs: ["文本", "图像", "PDF"],
    outputs: ["文本", "工具调用"],
    note: "使用 Messages 原生接口；填写账号可用的具体模型 ID。当前不直接接收音视频。",
    source: "https://platform.claude.com/docs/en/build-with-claude/pdf-support",
    protocol: "Messages",
    request: { messages: [{ role: "user", content: "根据资料写脚本" }] },
    response: "content[].text / tool_use",
    connection: {
      endpoint: "https://api.anthropic.com/v1",
      adapter: "anthropic-native",
      model: "",
      inputs: { image: true, audio: false, video: false, document: true },
    },
  },
];
