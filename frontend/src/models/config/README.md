# 模型配置

模型参数、默认值和说明都在此目录。业务代码不按模型名或端点判断能力。

- `catalog/*.json`：内置模型目录，添加模型时将配置复制到模型中心，之后以模型中心保存的配置为准。
- `dashscope.json`：百炼异步协议的路径和媒体字段映射；Wan 参数仍在目录 JSON。
- `services.json`：服务连接预设。
- `text-transports.json`：按服务地址声明附加认证头，不在代码中识别模型。
- `presets.json`：快捷添加的预设。
- `native-requests.json`：原生工具实际能传输的请求字段及固定值。
- `protocols.json`：原生传输协议的默认字段映射；显式模型配置优先。
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

Codex 图像连接在模型中心的「配置」中填写模型 ID，保存到 `params.model`。
新增连接默认使用 `gpt-image-2.5-sunburst`；文生图与图片编辑都原样发送该 ID，失败时不切换型号。
保存和生成均要求明确型号，缺失型号或 `codex-image` 别名直接报错；重新连接不会覆盖已选型号。
Codex 图像请求不再使用 `size=auto` / `quality=auto`：默认 `2048x1152`（16:9）和 `high`，项目生成请求会按项目画幅选择合法的 16 倍数尺寸；显式 `auto` 会被拒绝。
本机 Codex 内部接口实测可能忽略 `size`：请求 `1024x1024` 得到 `1672x940`，请求 `2048x1152` 得到 `1374x1145`。请求字段保持显式，但实际画布以返回结果为准；要精确尺寸需后处理，或改用支持 `image_size` 的 fal 路线。
Codex 登录接口的可用型号取决于账号权限，公开 Image API 支持某型号不代表此接口一定开放。

对话模型的自动容量只使用匹配端点和型号的服务元数据或官方目录。未知时留空，需刷新容量或手动填写后才能使用；输出上限留空时仅使用已知的模型默认值，否则交由服务决定。Anthropic 接口要求明确的输出上限，未知默认值时需手动填写。
