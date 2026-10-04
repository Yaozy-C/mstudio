# 架构

Mstudio 使用 Tauri 2、React 19、TypeScript、Radix UI 和 Phosphor 图标。前端维护交互草稿，后端是工程持久化与任务执行的唯一入口，经 Tauri IPC 同步。SQLite 保存工程、对话、任务与模型配置；Cordis 管理前端插件生命周期。

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

结构化脚本使用 `screenplay` 节点与 `screenplay.script`；镜头通过 `shot.screenplayId` 和 `shot.scriptId` 关联脚本及段落。当前默认配置包含 10 个角色（审片质检默认关闭）和 11 套仓库技能。角色定义见 `frontend/src/agents/defaults.json`，技能注册见 `desktop/src/assistant/skills.rs`；用户保存的配置决定实际可用角色与权限。

## 工程工具与生成执行

`frontend/src/domain/operationContract.ts` 定义每项操作的封闭参数对象；构建时生成 Rust 使用的 schema。模型看到的字段和执行前校验使用同一份契约，`oneOf`、nullable、数组长度与数字约束均受校验；不支持的 schema 关键字拒绝执行。验证一次返回字段路径列表。生成参数属于 `parameters`，转场时长属于 `set_transition.duration`，选定模型的参数能力与预设默认值随模型规则一起返回。

生成操作按图片、视频和独立参考资产分支声明。资产角色只暴露要求 `generationPurpose=asset` 和显式 `references` 的独立图片分支；视频组合 `mode` 不出现在图片参数中。模式、引用角色、状态、比例和分辨率使用有限值约束，比例与分辨率集合和模型适配器共用来源。模型说明使用 `inputDescription` 描述输入语义，避免把 `mode:image` 混入工具参数。联合类型校验根据已知判别字段定位分支，一次返回可定位的缺失、非法值和多余字段；不纠正字段别名或静默忽略参数。

Skill 的 `CORE.md` 提供关键专业原则，入口文档提供工作方法与条件阅读路径。提示词角色默认加载对应入口与写作指南；动作教程、真人表演、皮肤和环境光线案例按需读取，角色方法不再维护重复副本。公共保存、查询和任务生命周期规则由运行时维护。已有安装通过一次性事务更新已识别的默认正文、移除已合并的默认文档，并保留自定义正文、角色权限和启停状态；剪辑角色已知的强制复查句单独替换为完整回执验收。数据库仍是用户可编辑规则的事实来源。

`desktop/src/project_service` 管理读取、授权、对象观察、事务和回执。纯 TypeScript 领域函数在构建时由 Bun 编译为嵌入包，由受内存、栈和时间限制的 QuickJS 执行；不需要随应用携带 Node/Bun，不暴露 DOM、文件或网络。界面与后端复用纯领域规则，不维护 Rust/TypeScript 两套业务实现。已删除 WebView 工具执行事件、回调和 30 秒应答超时路径。

读取和宿主快照建立本轮对象观察，写入检查实际改变的已有对象及操作依赖，失败返回准确目标与读取参数。成功后刷新相关观察，并在同一 SQLite 事务保存工程与 `project_executions` 回执。相同执行编号的重放返回原回执；编号复用不同参数会被拒绝。重启恢复优先读取回执，避免把已保存修改误报为未知结果。工程快照带宿主来源元数据，持久化保留来源；供应商请求移除该内部元数据。用户文本前缀不参与快照识别。

界面自动保存提交基线与草稿，由后端对最新工程做三方合并：独立对象/字段合并，重叠编辑明确冲突。存储版本阻止乱序通知覆盖较新内容。`project-changed` 只通知界面刷新，没有执行职责。

保存的 `READY` 制作任务就是持久待执行队列。`production_queue` 跨项目按最多两个提交槽认领任务；上传与远端提交由应用持有。上传结束再次核查任务状态，取消后不会继续提交。`jobs::reserve` 在网络请求前保存提交编号；重启仅恢复没有远端占位记录的准备任务，有占位记录的任务沿用原 job 查询，绝不因超时自动创建新付费请求。远端本身不保证 exactly-once，`UNKNOWN` 保留这种不确定性。

后台 job 状态和已下载结果通过同一领域服务写回工程，页面关闭不阻止落盘。旧的页面生成扫描器、提交器、上传恢复和结果写回器均已删除。测试覆盖参数失败零写入、回执重放、并发合并/冲突、来源识别、跨重启队列恢复、取消准备和多结果幂等收取。

素材查询按 `jobs.asset/assets` 中的实际素材 ID 关联来源，返回原始提交 Prompt、模型和引用；不解析文件名，不用当前可编辑草稿代替原始请求。只展开请求的文本字段，媒体二进制与服务凭据不进入查询结果；原始文字和批量来源继续分页。

子 Agent 的执行进度以项目、父轮次、调用 ID 和子轮次关联。公开阶段、当前工具、最近进展和耗时投影到 `agent_child_activity`，由真实事件推送刷新，展开后读取该子轮次的操作记录。取消、失败、完成和重启中断分别显示，历史轮次不会跟随同一个子会话的后续任务变化。过程文字保留在会话记录与进度中；完成回传只使用最后一条无工具调用的模型回复，完整结论在输出限额内直接交给父 Agent。

编辑回执的 `savedValues[].values` 返回请求字段的实际保存值，包括图片/视频提示词、分镜图和引用；数组返回合并后的完整值。生成回执与查询共用 `taskOutcome`，提供 `status`、实际 `resultAssetIds` 和 `continuation`：等待用户、等待后台、结果可检查、已结束。委派返回时刷新任务事实，长输出卸载保留这些控制字段。Prompt 和 Skill 直接使用完整回执，等待确认时报告用户动作，不重复查询。

后续检查确实依赖后台结果时，Agent 使用 `mstudio_await_generation`。工具先订阅提交事件再读取持久任务，后台状态写入、导入结果、取消、暂停、删除都会唤醒等待；无关项目事件不触发重读。等待期间没有模型轮询或远端查询，只由原有后台 worker 管理远端状态。确认阻塞立即返回，后台任务在收到实际结果、失败或需要用户动作时返回；共享本轮取消信号与时限。重启后中断的等待从持久任务恢复事实，不重新提交生成。普通回复和独立工作仍由通用 Agent 循环处理，不按读取次数截断。

设计对照：[DSH 工具契约](https://github.com/deepseek-ai/deepseek-harness/blob/639ed015397290b3745d163aafe02ffee4aa3f84/packages/core/tools/src/schema.ts)、[观察策略](https://github.com/deepseek-ai/deepseek-harness/blob/639ed015397290b3745d163aafe02ffee4aa3f84/packages/fs/fs-observation-policy/src/index.ts)、[工具调度](https://github.com/deepseek-ai/deepseek-harness/blob/639ed015397290b3745d163aafe02ffee4aa3f84/packages/core/agent-loop/src/tool-calls.ts)。工程对象、SQLite 事务和媒体任务队列是 Mstudio 的领域实现。

## 预览与导出

桌面预览由 GES 处理解码、合成与音频，通过二进制 IPC 将最新 RGBA 帧呈现在 WebView Canvas。Windows 已有独立的 DLL/插件打包、NSIS 安装器与 CI 检查流程，物理声音、多屏 DPI 等仍需实机验证，见[开发指南](development.md)。浏览器界面使用 HTML 媒体预览作为开发模式。导出使用 FFmpeg 合成管线；两条管线需要分别验证。

## 外部服务

模型适配器负责请求转换、凭据和任务轮询。远程服务仅接收本次请求选择的上下文与媒体；配置的端点决定数据接收方。本地凭据数据库不应上传仓库或用于公开复现。

对话支持 OpenAI 兼容、Responses、Gemini、Claude 原生协议及本机 Codex 接入。媒体供应商当前注册 `fal`、`gemini-native`、`http-json`、`codex-image`，对话与媒体能力分别校验。后台按输出索引导入全部结果，部分输出失败可继续收取；不再只导入第一张图片。附件每轮最多 12 个，原始内联素材单个最多 12 MiB、组合序列化后最多 18 MiB；文本文件最多 120 KB。原始音视频与 PDF 的可发送范围由具体模型及接口共同决定，视频抽帧不等于完整视听理解。

预览由单个前端会话控制器管理，连续定位和倍速请求合并，播放/暂停保留操作边界。指令应答以状态完成及新目标帧到达为准。轨道名称和画布关联不参与渲染规格比较，连续参数变动停止 120 毫秒后再重建。旧会话的回调不能更新新会话；预处理缓存写入串行，已过期的等待请求跳过。隐藏助手面板时保留任务运行时，但卸载消息视图。

## Agent 任务上下文

所有内置和自定义角色共用 `assistant/task_context.rs`、`harness/session_selection.rs` 和工具执行管线。聊天长期保留，工具明细保留七天，恢复依赖独立于日志保留；模型读取任务范围内的投影。

- 消息 attribution 保存任务 ID、角色、引用对象、工作区及原始要求。同角色的后续消息继续任务；对象或工作区变化只更新当前操作上下文。对话顶部的“新任务”显式另起任务，不删除聊天或共享约束。程序不靠关键词猜话题变化。
- 会话恢复按项目、角色、任务及模型路由匹配；重试沿用原任务和已提交操作。缺少任务上下文的记录不支持重试，需重新发起任务；重置会阻止恢复旧任务。
- 普通新消息仅读取同任务、同角色最近一个已完成问答，按 turn ID 配对，不自动重放旧工具过程；显式重试或 `/compact` 才恢复匹配会话。首轮可选上下文预算为模型预算与 12,000 估算 tokens 的较小值；当前用户输入及必要规则不会为了达标而静默删除。恢复会话继续使用已有压缩机制。
- 保留共享项目约束、格式、版本和对象数量；指定目标后省略无关节点目录。`inspect` 支持 `ids/nodeIds/fields`，返回缺失对象、已省略字段和分页信息。约束截断时通过 `section=creation` 补读；`history` 支持按 `taskId` 查询。
- 子 Agent 默认 spawn 独立历史；显式 fork 保留父会话已完成历史。子 Agent 工程快照不再重复系统消息中的角色规则与技能目录。
- 编辑回执返回状态、版本、修改对象/字段及任务信息，不重复整份工程概览。普通结果超过 6,000 Unicode 字符时卸载；受限列表、含 savedClips 的编辑回执、生成等待与委派结果使用 14,000 字符限额。模型收到预览和 `turnId/resultRef`，通过 `mstudio_read_result` 分页回读。精简消息持久化，重启不恢复成庞大原文；图片沿用媒体工具。
- `context/usage` 记录估算组成及供应商校准后的压力；`context/selection` 记录任务和恢复条数。普通请求及摘要分别记录供应商总量和缓存输入。估算不是账单，缺失用量不是零。

测试覆盖角色/任务隔离、连续修改、新任务重试、重置、共享约束、字段读取、Unicode 回读和跨项目拒绝。此次优化不改变原始媒体附件策略，也不承诺固定比例的 token 节省。

参考：[DSH 压缩](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/compaction)、[DSH 计量](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/llm/token-meter)、[Deep Agents](https://www.langchain.com/blog/context-management-for-deepagents)。

编辑批次在副本上执行，镜头顺序唯一性在整批结束时校验，再一次保存；失败不提交中间结果。宿主按本轮实际提供的对象保存内容哈希，忽略节点布局与任务执行遥测；不再要求模型携带工程 revision。`inspect nodeIds` 的 `fields` 真正筛选内容，排序读取使用 `title/shot.order/shot.duration`，方案镜头目录使用 `shots`。同一子任务连续三次修改遇到相同错误会停止重复尝试，读取操作不会清零计数，成功修改才清零。通用执行循环不设固定轮数上限，也不按媒体任务数量扩容；工具执行后继续请求模型，直到正常完成、取消或发生错误。

### GES Canvas 预览

`native_preview` 只负责会话所有权，所有权锁内不等待引擎。`preview_validate` 校验输入，`preview_prepare` 编译播放计划。`preview_process` 启动同一安装包的 `--preview-worker` 子进程，子进程在初始化 Tauri 前进入工作模式；`ges_engine` 的专用线程是 GES 管线唯一的控制入口。

控制通过带请求编号的有界二进制协议收发，逐条确认完成。状态和 RGBA 帧由独立读取线程维护为最新快照，不经过控制队列；帧不积压。快照与应答按发送顺序捕获，避免旧快照覆盖新应答。`previewFrames` 保持一个在途请求，并在绘制前复查会话状态与目标位置。

子进程退出会使待执行指令失败并显示重新加载入口，不会直接终止主应用。关闭给予 500 毫秒退出时间，超时强制结束并回收子进程；控制超时同样终止失去响应的进程。`ORC_CODE=backup` 仍在 macOS arm64 子进程启动前设置，规避已观测到的透明混合动态代码崩溃。

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

模型侧按任务注册独立工具，不再暴露 `mstudio_edit.operations`：视频/图片生成、镜头提示词、任务提示词、剧本和镜头结构各自使用独立参数。角色权限同时裁剪工具与字段；宿主将参数转换为既有内部操作，复用事务、权限和回执恢复。镜头重排用 `mstudio_update_shots(items)` 原子提交。界面及恢复流程识别新工具名，并兼容历史回执。内部操作参数仍在操作分支及字段本身声明用途、层级、单位、枚举和省略语义。`update_node.shot.prompt/framePrompt` 保存镜头草稿，`request_generation.text` 和 `update_generation.text` 写生成任务提示词，互不隐式覆盖。图片与视频分支分别展示参数；媒体能力查询与操作契约复用 `parameterSchema`，执行侧仍按所选模型校验，不把镜头时长或工程导出规格当作生成规格。
