# 架构

Mstudio 使用 Tauri 2、React 19、TypeScript 和 SQLite。前端维护交互草稿，通过 `bridge` 调用 Tauri 命令；后端负责持久化、模型请求与后台任务。

## 代码边界

- `src/`：独立 Rust 媒体核心，负责 FFmpeg / ffprobe、混音、调色、转场、导出及 GES 播放计划。
- `desktop/src/`：桌面命令、SQLite、素材导入、模型适配、生成队列与 Agent。
- `desktop/image-runtime/`：本机深度图与图像修复推理；与桌面端共用 Cargo workspace 和锁文件。
- `frontend/src/domain/`：界面与宿主共用的纯领域规则。Bun 构建为嵌入包，宿主用受内存、栈和时间限制的 QuickJS 执行，只交换 JSON。
- `frontend/src/creative/`、`production/`、`canvas/`：脚本、镜头、生成任务与画布。
- `frontend/src/timeline/`：多轨编辑、播放、素材拖放与预览控制。
- `skills/`：应用自带的创作方法，随 Tauri resources 分发。
- `scripts/`：开发、验证和打包；`desktop/native/` 保存生成的原生运行库，不提交。

## 工程与素材

`Project` 是工程编辑与自动保存的聚合。素材以 ID 标识，片段引用素材并保存裁切、速度、轨道和变换。素材库收录、画布卡片、生成参考和镜头产出分别表示不同关系。

结构化脚本存于 `screenplay.script`，镜头通过 `shot.screenplayId/scriptId` 关联段落。`references` 保存生成参考，`shot.frames` 保存分镜图，`shot.takes` 保存候选视频。画布坐标只负责展示。独立资产生成结果收录到素材库并创建独立卡片。

界面保存基线与草稿，宿主对最新工程做三方合并：独立字段合并，重叠编辑返回冲突。存储版本阻止乱序通知覆盖新内容；`project-changed` 触发界面刷新。撤销、迟到的生成结果与并发保存都尊重永久删除的素材 ID。

画布视角按项目 ID 存在本机界面偏好中，不进入自动保存或撤销。节点与卡片布局属于工程内容。

素材、代理、生成结果与导出位于配置的媒体目录；SQLite 留在应用数据目录。项目删除只清理应用管理且已无引用的文件，不删除用户原文件。

## 工程工具与生成

`domain/operationContract.ts` 定义操作字段与 schema，构建时同时提供给 Rust。角色工具将模型参数转换为领域操作，由 `project_service` 统一执行权限、对象观察、事务与回执。

读取登记本轮实际取得的对象版本。写入检查改变的对象及依赖；解码期间发生的并发修改不会被登记成已读。批次在副本上执行，整批校验通过后原子保存。执行 ID 和回执在同一事务提交，重放同一请求返回原回执，复用 ID 提交不同参数会被拒绝。

生成草稿与开始执行分开：保存的草稿为 `DRAFT`，用户开始后进入 `READY` 队列。缺少名称时显示“未命名”。模型能力和参数来自 `frontend/src/models/config/`，保存的模型配置决定实际输入角色、尺寸、时长及限制。

`production_queue` 跨项目最多同时提交两个任务，上传完成后再次检查取消状态。网络提交前保存 job 占位；有远端请求记录的任务沿用原 job 查询。提交结果不确定时保留 `UNKNOWN`，避免自动重复付费请求。后台 worker 下载全部结果，并通过领域服务写回工程，页面关闭不影响落盘。

素材来源按 job 中真实的素材 ID 查询，保留原始 prompt、模型和参考用途。普通任务、素材与委派读取只携带结果及必要元数据；明确编辑 prompt、原样重试或调查生成时才读取历史请求。

`image_processing` 接收已有图片，执行深度图、放大修复或 LUT，返回新素材列表并登记项目引用。推理输出是图片文件；源图保留。模型安装清单与校验和见 `desktop/image-runtime/models.json`。

## Agent 与 Skill

内置角色来自 `frontend/src/agents/defaults.json`。用户保存的配置决定角色、工具与 Skill。Skill 正文存于 `skill_resources`，安装时只补缺少的资源，运行时读取数据库当前内容。专业方法由各自 Skill 维护；宿主提供共同执行规则，工具契约说明参数。

普通聊天、委派、续接与父任务唤醒共用 `assistant/turn_context.rs`。消息 attribution 保存任务 ID、角色、目标与原始要求。继续任务与新建任务由明确操作区分；工作区和选中对象是上下文，不授予额外权限。

普通新消息读取同任务、同角色最近一个已完成问答；重试与压缩恢复匹配会话。会话按项目、角色、任务和模型路由隔离。`inspect` 支持对象、字段和分页读取。长工具结果卸载后通过 `mstudio_read_result` 分页恢复，Skill 正文完整交给模型。

工具循环持续到模型正常结束、用户停止、整轮超时或不可恢复错误。工具参数错误返回模型纠正。生成完成、待用户开始和等待远端结果是不同状态；工程回执证明已保存操作，不能证明画面效果。

委派立即返回 `childId/outputRef`。独立目标可以并发，依赖新结果或写入相同目标时顺序执行。调度器与子任务共用四个执行名额，等待期间支持取消和期限。

交接 `artifacts` 携带工程对象或已保存输出的引用；专业角色按权限读取。用户明确发送的附件提供实际媒体。`outputRef` 绑定子任务的具体轮次与写入回执，后续续接不会覆盖之前输出。

子任务结果与通知事务性提交。父任务运行时在模型步骤边界接收；空闲时由 `parent_activation` 恢复会话继续。仍有子任务或未处理通知时，父轮次保持 `waiting`；完成后才结束。前端通过 `agent-execution` 事件与 `agent_execution_state` 恢复状态，并用序号拒绝旧通知。

`media-read` 单独授权主动读取媒体，并依赖 `project-read`。用户主动发送的附件按所选模型的输入能力提供。

## 历史、内容与上传

聊天与生成任务长期保留。工具明细和重试诊断在非活动轮次最后活动七天后清理；恢复仍引用的结果保留。`session/start` 是带 `checkpointSeq` 的恢复快照，结算与启动时折叠已有消息和媒体卸载状态。活跃父子任务不参与清理。

大文本压缩后按 SHA-256 保存到 `content_blobs`，由 `job_content/event_content` 外键关联；查询元数据保留在 job/event 行内。使用 `jobs::{save,reserve,get}` 和 `journal::{append,insert}` 维护完整记录与内容引用。

二进制媒体按内容哈希写入 `reference-media/`，数据库保存媒体引用和原编码前缀。重放时读取并校验，再在内存生成供应商请求。文件先写完再提交引用；共享文件在最后一个所有者释放后才清理。

模型响应只推进媒体消费边界。图片或上下文预算触发卸载时，按顺序省略足够数量的最旧输入；尚未处理的新批次和助手输出保留。省略状态持久化，重新打开只增加新的输入位置。恢复引用使用内容哈希，原文件保留。

`assistant/uploader` 提供内容寻址、持久索引、并发去重与过期 ID 的一次重传。官方 DeepSeek 和 OpenAI 服务使用各自 Files 协议，端点与凭据命名空间隔离。请求投影不修改会话原图；远程 URL 不下载重传，认证或格式失败按实际错误返回。Codex 使用自身支持的本地图片输入。

## GES 与桌面预览

`native_preview` 管理会话所有权，`preview_prepare` 编译播放计划；同一应用的 `--preview-worker` 子进程持有 GES，专用线程是管线唯一控制入口。控制使用带请求 ID 的有界二进制协议。定位完成需要管线应答和目标新帧，普通暂停只切换状态。

macOS 通过三个共享内存槽传 BGRA，Metal 纹理在 GPU 完成后复用；像素不穿过 WebView。Windows 由独立读取线程保存最新 RGBA 快照。前端每次只保留一个帧请求，绘制前复核目标和控制代数。AppKit 视图操作调度到主线程，隐藏页面、遮挡或卸载时同步隐藏原生视图。

预处理缓存串行写入，过期请求跳过。片段效果、转场和导出共用 FFmpeg 定义，GES 负责层级、字幕、时钟与定位。缓存键包含算法版本，输出覆盖完整帧数；定位使用 GES 帧时间网格。macOS 子进程使用 `ORC_CODE=backup` 规避透明混合的动态代码崩溃。

子进程退出或控制超时使待执行指令失败；关闭给予 500 毫秒退出时间，再强制回收。界面提供重新加载入口。

## Three.js 预演

`desktop/src/previs` 是项目内独立的文件与对话工作区。`previs_files/messages/exports/media` 随项目级联删除。文件保存到稳定的 `model.js/scene.js`，内部 revision 用于并发检查。

模型模块导出 `createModel({THREE})`，镜头模块导出 `createShot({THREE,models,seed})`，返回 `scene/camera/update(t)` 与可选 `dispose()`。模型别名绑定稳定 ID，模型修改只使依赖镜头的渲染失效。

预演 Agent 使用共用执行循环和文本传输，接收当前文件、选定模型与要求。`str_replace_editor` 支持读取、创建、唯一替换和插入；成功写入立即持久化。失败或取消保留已完成编辑。

导出冻结镜头及引用版本，按 `frameIndex/fps` 逐帧采样，有界 PNG 经 IPC 交给 FFmpeg。成功渲染原子替换该镜头最新导出，并重定向有效引用；失败或过期渲染保留旧结果。

私有预演视频通过宿主派生的 `Project.previsAssets` 提供，不写入普通素材数组。引用视频与另存到素材库是两个动作，后者建立独立素材副本。关闭编辑器保留任务，显式停止才取消模型；离开项目会释放未完成的帧捕获。

## 存储维护与验证

`storage/maintenance` 按实时引用图扫描空间，区分素材库、工程、任务、对话、恢复回执、预演及内容快照。导出成片、未知用户文件、符号链接和五分钟内写入的文件不自动删除。

清理取得排他文件锁，拒绝活动任务，重新计算计划后事务性删除记录并提交持久文件清理队列。返回实际释放空间和可重试失败。存储迁移复制并校验媒体，更新引用后切换目录。

完整验证入口为 `scripts/check.sh`，包括格式、类型、构建和行为测试；真实像素、声音与 GES 检查见[开发指南](development.md)。桌面打包另验证原生依赖闭包、签名、搬移路径与媒体工具。

## 实现参考

- [DSH 工具调度](https://github.com/deepseek-ai/deepseek-harness/blob/5badb15009ae1756c3afe0ae0cef1faafc290ccc/packages/subagent/tool-subagent/src/index.ts)与[父任务唤醒](https://github.com/deepseek-ai/deepseek-harness/blob/5badb15009ae1756c3afe0ae0cef1faafc290ccc/packages/subagent/subagent/src/continuation-activation.ts)。
- [DSH 图片预算](https://github.com/deepseek-ai/deepseek-harness/blob/571902028a75c64c1d767f4ae5fd099485f0ddf0/packages/llm/llm/src/content.ts)与[图片卸载](https://github.com/deepseek-ai/deepseek-harness/blob/571902028a75c64c1d767f4ae5fd099485f0ddf0/packages/compaction/compaction-image-offload/src/image-offload.ts)。Mstudio 额外保护尚未消费的新批次。
- [Anthropic artifact 交接](https://www.anthropic.com/engineering/multi-agent-research-system)、[DSH 压缩](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/compaction)、[DSH 计量](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/llm/token-meter)。
