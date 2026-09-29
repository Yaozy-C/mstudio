# 架构

Mstudio 使用 Tauri 2、React 19、TypeScript、Radix UI 和 Phosphor 图标。前端管理脚本、画布和剪辑状态，经 Tauri IPC 调用本机 Rust 服务。SQLite 保存工程、对话、任务与模型配置；Cordis 管理前端插件生命周期。

## 核心边界

- `src/`：独立 Rust 媒体核心，调用 FFmpeg / ffprobe 进行媒体探测、代理、混音和导出。`preview_ges.rs` 将时间线转换为 GES 播放计划。
- `desktop/src/`：Tauri 命令、持久化、导入、模型适配、生成任务和 Agent 执行。`assistant/harness/` 负责轮次、工具执行和任务恢复。
- `desktop/native/`：开发和打包所需的 GStreamer/GES 运行库（不提交）。
- `frontend/src/creative/`、`production/`：脚本、分镜、画布和生成流程。
- `frontend/src/timeline/`：多轨编辑、播放时钟、素材拖放及预览控制。
- `skills/`：应用自带的创作规则，由 Tauri resources 分发，不依赖开发者个人技能目录。

## 媒体与项目

素材记录与时间线片段分离，片段引用素材 ID 并保存开始位置、裁切、速度和变换。图片作为默认三秒的画面片段插入，可继续调整。拖入指定画面轨道时按项目帧率对齐。

工程自动保存在 SQLite；导入的媒体、代理、导出和生成文件位于应用数据目录。原始用户文件不因项目删除而被删除。项目操作通过统一变更入口支持当前会话撤销。

## 预览与导出

桌面预览由 GES 处理解码、合成与音频，通过二进制 IPC 将最新 RGBA 帧呈现在 WebView Canvas。Windows 打包仍待实现与实机验证。浏览器界面使用 HTML 媒体预览作为开发模式。导出使用 FFmpeg 合成管线；两条管线需要分别验证。

## 外部服务

模型适配器负责请求转换、凭据和任务轮询。远程服务仅接收本次请求选择的上下文与媒体；配置的端点决定数据接收方。本地凭据数据库不应上传仓库或用于公开复现。

预览控制保持音频设备运行，定位请求在前端合并并保留播放/暂停边界；原生层清除旧缓冲帧。轨道名称和画布关联不参与渲染规格比较，连续参数变动停止 120 毫秒后再重建。播放器在后台线程关闭 GES 管线并回收；回收与新建串行执行，避免旧设备关闭影响新设备。隐藏助手面板时保留任务运行时，但卸载消息视图。

## Agent 任务上下文

所有内置和自定义角色共用 `assistant/task_context.rs`、`harness/session_selection.rs` 和工具执行管线。聊天长期保留，工具明细保留七天，恢复依赖独立于日志保留；模型读取任务范围内的投影。

- 消息 attribution 保存任务 ID、角色、引用对象、工作区及原始要求。同角色的后续消息继续任务；对象或工作区变化只更新当前操作上下文。对话顶部的“新任务”显式另起任务，不删除聊天或共享约束。程序不靠关键词猜话题变化。
- 会话恢复按项目、角色、任务及模型路由匹配；重试沿用原任务和已提交操作。缺少任务上下文的记录不支持重试，需重新发起任务；重置会阻止恢复旧任务。
- 没有可恢复会话时，仅读取同任务最近六个已完成轮次，按 turn ID 配对。首轮可选上下文预算为模型预算与 12,000 估算 tokens 的较小值；当前用户输入及必要规则不会为了达标而静默删除。恢复会话继续使用已有压缩机制。
- 保留共享项目约束、格式、版本和对象数量；指定目标后省略无关节点目录。`inspect` 支持 `ids/nodeIds/fields`，返回缺失对象、已省略字段和分页信息。约束截断时通过 `section=creation` 补读；`history` 支持按 `taskId` 查询。
- 子 Agent 默认 spawn 独立历史；显式 fork 保留父会话已完成历史。子 Agent 工程快照不再重复系统消息中的角色规则与技能目录。
- 编辑回执返回状态、版本、修改对象/字段及任务信息，不重复整份工程概览。超过 6,000 Unicode 字符的结果保留原文，模型收到预览和 `turnId/resultRef`，通过 `mstudio_read_result` 分页回读。精简消息持久化，重启不恢复成庞大原文；图片沿用媒体工具。
- `context/usage` 记录估算组成及供应商校准后的压力；`context/selection` 记录任务和恢复条数。普通请求及摘要分别记录供应商总量和缓存输入。估算不是账单，缺失用量不是零。

测试覆盖角色/任务隔离、连续修改、新任务重试、重置、共享约束、字段读取、Unicode 回读和跨项目拒绝。此次优化不改变原始媒体附件策略，也不承诺固定比例的 token 节省。

参考：[DSH 压缩](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/compaction)、[DSH 计量](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/llm/token-meter)、[Deep Agents](https://www.langchain.com/blog/context-management-for-deepagents)。

编辑批次在副本上执行，镜头顺序唯一性在整批结束时校验，再一次保存；失败不提交中间结果。工程编辑版本忽略画布视口、卡片位置与尺寸变化，内容修改仍使用严格版本检查。`inspect nodeIds` 的 `fields` 真正筛选内容，排序读取使用 `title/shot.order/shot.duration`，方案镜头目录使用 `shots`。同一子任务连续三次修改遇到相同错误会停止重复尝试，读取操作不会清零计数，成功修改才清零；16 步上限保留，以 `step-limit` 返回给统筹。

### GES Canvas 预览

`native_preview` 保留现有 IPC 命名，负责会话所有权与生命周期；`preview_validate` 校验输入，`preview_prepare` 将工程编译成 GES 播放计划。`ges_engine` 在独立线程持有 GES 管线，返回最新一帧的 RGBA 二进制数据；`previewFrames` 在一个在途请求内提交给 Canvas。无需 C++ 桥接或原生显示浮层。

GES 首版按片段缓存 FFmpeg 调色/变速，复用转场与导出音频混音逻辑；GES 负责时间线层级、字幕、时钟和 seek。切换预览清晰度不会写入工程或改变导出路径。技术取舍是保持现有效果语义，代价是效果修改后的准备时间。未来可逐项引入 GES 原生效果，但必须先验证与现有导出的一致性。

## SQLite content storage

Project documents remain atomic editable aggregates. Job and event query metadata stay in
`jobs.data` / `agent_events.payload`; large text strings are moved to compressed,
SHA-256-addressed `content_blobs`, shared through `job_content` and `event_content`.
Reference ownership and cascade deletion are enforced with foreign keys. Jobs also
reference projects. Paths stay inline for relocation and ownership tracking.

Use `jobs::{save,reserve,get}` and `journal::{append,insert}` for complete records.
The parent and content references must be written in the same transaction. `insert`
requires the caller's transaction. Full event readers use `database::blobs::event`;
SQL projections may omit large fields and hydrate only retained properties. Never
interpret user-provided JSON objects as stored-content references. Job list/worker
queries intentionally omit input/result/output bodies. Replacing jobs and deleting
projects collect unreferenced blobs while retaining content shared by other owners.

`database/schema.rs` owns the versioned migration and indexes for turn replay, tool
result lookup, message attribution, job ownership/connections, reverse media ownership,
and pending inbox delivery. Migrations commit one parent per transaction and compact
the database. They do not
retain database backups. Restarting an interrupted migration retains committed
references. The opt-in database audits verify records on disposable copies.

Binary media are decoded into immutable files under the configured local storage's
`reference-media/` folder. `stored_media` records their content hashes and sizes;
`content_blobs` keeps only a media reference and the exact provider encoding prefix.
Text content remains compressed in SQLite. Raw Base64 and data URLs for identical
bytes share one file. Replay reads and verifies the file and creates Base64 only in
memory for the provider. File writes complete before references commit; orphan cleanup
and project deletion retain shared files. Storage relocation includes this folder.

Agent history has three lifetimes, implemented by `database/history_cleanup.rs`:

- Chat messages and generation tasks keep their durable content and media references.
- Tool execution details and retry diagnostics expire seven days after the last activity
  of an inactive turn. Results explicitly referenced by recovery snapshots remain readable.
- `session/start` is reused as a recovery snapshot with a `checkpointSeq` watermark.
  Settlement and startup fold internal messages, compaction and image rewrites into this
  snapshot, transactionally replacing previous versions and duplicate result messages.
  Replay starts after the watermark; original sequence IDs preserve reset boundaries.
  Completed predecessor snapshots in the same scoped task are replaced; interrupted
  tasks and current child continuations retain recovery state.

`turn/usage` aggregates provider token counts and request/compaction counts per turn.
Transient progress, skill-read notifications and startup notifications are live-only,
classified in `event_retention.rs`; stream fragments still update the assistant-message
crash checkpoint. Unknown event kinds remain durable. Active parent/child runs are
excluded from cleanup. Cleanup runs at startup and hourly while the app remains open;
settlement immediately folds recovery state. Referenced media use the existing blob
ownership and post-commit file cleanup. Migration v4 compacts existing data without a
retained backup. No cleanup path sends model requests or replays tool side effects.
