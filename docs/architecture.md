# 架构

Mstudio 使用 Tauri 2、React 19、TypeScript、Radix UI 和 Phosphor 图标。前端管理脚本、画布和剪辑状态，经 Tauri IPC 调用本机 Rust 服务。SQLite 保存工程、对话、任务与模型配置；Cordis 管理前端插件生命周期。

## 核心边界

- `src/`：独立 Rust 媒体核心，调用 FFmpeg / ffprobe 进行媒体探测、代理、混音和导出。`preview_mlt.rs` 将时间线转换为 MLT 图。
- `desktop/src/`：Tauri 命令、持久化、导入、模型适配、生成任务和 Agent 执行。`assistant/harness/` 负责轮次、工具执行和任务恢复。
- `desktop/native/`：MLT 播放器与平台显示表面；桌面视频像素不通过 IPC 传输。
- `frontend/src/creative/`、`production/`：脚本、分镜、画布和生成流程。
- `frontend/src/timeline/`：多轨编辑、播放时钟、素材拖放及预览控制。
- `skills/`：应用自带的创作规则，由 Tauri resources 分发，不依赖开发者个人技能目录。

## 媒体与项目

素材记录与时间线片段分离，片段引用素材 ID 并保存开始位置、裁切、速度和变换。图片作为默认三秒的画面片段插入，可继续调整。拖入指定画面轨道时按项目帧率对齐。

工程自动保存在 SQLite；导入的媒体、代理、导出和生成文件位于应用数据目录。原始用户文件不因项目删除而被删除。项目操作通过统一变更入口支持当前会话撤销。

## 预览与导出

桌面预览由 MLT 处理解码、合成与音频，macOS 使用 Core Animation 呈现，Windows 表面仍待实机验证。浏览器界面使用 HTML 媒体预览作为开发模式。导出使用 FFmpeg 合成管线；两条管线需要分别验证。

## 外部服务

模型适配器负责请求转换、凭据和任务轮询。远程服务仅接收本次请求选择的上下文与媒体；配置的端点决定数据接收方。本地凭据数据库不应上传仓库或用于公开复现。

预览控制保持音频设备运行，定位请求在前端合并并保留播放/暂停边界；原生层清除旧缓冲帧。轨道名称和画布关联不参与渲染规格比较，连续参数变动停止 120 毫秒后再重建。播放器回收先在主线程移交所有权，再由后台线程关闭音频设备，最后回主线程销毁显示表面；回收与新建串行执行，避免旧设备关闭影响新设备。隐藏助手面板时保留任务运行时，但卸载消息视图。

## Agent 任务上下文

所有内置和自定义角色共用 `assistant/task_context.rs`、`harness/session_selection.rs` 和工具执行管线。完整聊天与工具原文保存在 SQLite；模型读取任务范围内的投影。

- 消息 attribution 保存任务 ID、角色、引用对象、工作区及原始要求。同角色同对象或无新对象的后续消息继续任务；对象或工作区变化开启新任务。对话顶部的“新任务”显式另起任务，不删除聊天或共享约束。程序不靠关键词猜话题变化。
- 会话恢复按项目、角色、任务及模型路由匹配；重试沿用原任务和已提交操作。缺少任务上下文的记录不支持重试，需重新发起任务；重置会阻止恢复旧任务。
- 没有可恢复会话时，仅读取同任务最近六个已完成轮次，按 turn ID 配对。首轮可选上下文预算为模型预算与 12,000 估算 tokens 的较小值；当前用户输入及必要规则不会为了达标而静默删除。恢复会话继续使用已有压缩机制。
- 保留共享项目约束、格式、版本和对象数量；指定目标后省略无关节点目录。`inspect` 支持 `ids/nodeIds/fields`，返回缺失对象、已省略字段和分页信息。约束截断时通过 `section=creation` 补读；`history` 支持按 `taskId` 查询。
- 子 Agent 默认 spawn 独立历史；显式 fork 保留父会话已完成历史。子 Agent 工程快照不再重复系统消息中的角色规则与技能目录。
- 编辑回执返回状态、版本、修改对象/字段及任务信息，不重复整份工程概览。超过 6,000 Unicode 字符的结果保留原文，模型收到预览和 `turnId/resultRef`，通过 `mstudio_read_result` 分页回读。精简消息持久化，重启不恢复成庞大原文；图片沿用媒体工具。
- `context/usage` 记录估算组成及供应商校准后的压力；`context/selection` 记录任务和恢复条数。普通请求及摘要分别记录供应商总量和缓存输入。估算不是账单，缺失用量不是零。

测试覆盖角色/任务隔离、连续修改、新任务重试、重置、共享约束、字段读取、Unicode 回读和跨项目拒绝。此次优化不改变原始媒体附件策略，也不承诺固定比例的 token 节省。

参考：[DSH 压缩](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/compaction)、[DSH 计量](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/llm/token-meter)、[Deep Agents](https://www.langchain.com/blog/context-management-for-deepagents)。
