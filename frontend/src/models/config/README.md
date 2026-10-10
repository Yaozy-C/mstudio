# 模型配置

模型参数、默认值和说明都在此目录。业务代码不按模型名或端点判断能力。

- `catalog/*.json`：内置模型目录，添加模型时将配置复制到模型中心，之后以模型中心保存的配置为准。
- `dashscope.json`：百炼异步协议的路径和媒体字段映射；Wan 参数仍在目录 JSON。
- `services.json`：服务连接预设。
- `text-transports.json`：按服务地址声明附加认证头，不在代码中识别模型。
- `presets.json`：快捷添加的预设。
- `native-requests.json`：原生工具实际能传输的请求字段及固定值。
- `protocols.json`：原生传输协议的默认字段映射；显式模型配置优先。
- `legacy/*.json`：旧版模型的一次性迁移快照，不覆盖用户已经保存的能力配置。
- `prompt-rules.json`：提示词说明及其关联，独立于创意 Skill。
- `example-capabilities.json`：模型中心的配置示例。

## 用户关心的设置

| 设置                       | 配置位置                                     |
| -------------------------- | -------------------------------------------- |
| 文生、参考图、首帧、尾帧   | `capabilities.references` 的 `role` 和 `key` |
| 分辨率                     | `capabilities.controls.resolution`           |
| 时长（秒，允许小数）       | `capabilities.controls.duration`             |
| 参考图是否必填、单张或多张 | `references` 的 `required`、`multiple`       |
| 比例                       | `capabilities.controls.aspectRatio`          |
| 默认值                     | `request`（目录）或模型中心的默认生成参数    |

`key` / `path` 是请求体的 JSON Pointer，例如 `/input/duration`。
比例和分辨率的 `values` 按服务要求填写，大小写不转换。
时长和自定义尺寸的 `min` / `max` 可选，省略则不附加范围。
参考数量的 `max`、合计 `referenceLimit`、参考视频合计秒数 `referenceSeconds` 同样可选。
没有全局 12 个参考、9 张图片、3 段视频、5–15 秒时长或单段参考 2–15 秒限制。
已有内置模型的已声明范围保留在 JSON 中，可以在模型中心修改。

若服务明确说明比例跟随首帧，可配置 `ratioFromReference: true`；程序不会自行推断。
原生协议的字段格式仍须符合实际传输格式，例如原生图片接口只编码图片。
配置不会自动探测服务端能力，也不会改变服务端自身接受的参数范围。
