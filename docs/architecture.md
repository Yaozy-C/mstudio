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

素材文件以 `assets[].id` 标识；素材库收录、画布卡片和镜头使用关系分别存储。角色、产品等独立参考图生成完成后，同时收录到项目素材库并创建独立画布卡片，不能因 `generationPurpose=asset` 跳过展示，也不因当前选中镜头而变为它的产出。镜头的 `references` 表示生成参考，`shot.frames` 表示镜头画面，`shot.takes` 表示可供剪辑的候选版本；这些字段引用素材 ID，同一素材可被多个镜头使用。画布坐标只负责展示，不能据此推断镜头归属。此行为仅用于新接收的生成结果，历史已接收素材保持原状，不自动补建画布卡片。

工程自动保存在 SQLite；导入的媒体、代理、导出和生成文件位于应用数据目录。原始用户文件不因项目删除而被删除。项目操作通过统一变更入口支持当前会话撤销。

画布平移与缩放按项目 ID 保存在本机界面偏好中，不进入工程修改、自动保存或撤销历史。旧工程的 viewport 仅作首次恢复的默认值；当前视角供界面定位新内容，生成任务只保存已计算的位置。节点与卡片布局仍属于工程内容，正常自动保存。

结构化脚本使用 `screenplay` 节点与 `screenplay.script`；镜头通过 `shot.screenplayId` 和 `shot.scriptId` 关联脚本及段落。当前默认配置包含 10 个角色（审片质检默认关闭）和 11 套仓库技能。角色定义见 `frontend/src/agents/defaults.json`，技能注册见 `desktop/src/assistant/skills.rs`；用户保存的配置决定实际可用角色与权限。

## 工程工具与生成执行

`frontend/src/domain/operationContract.ts` 定义每项操作的封闭参数对象；构建时生成 Rust 使用的 schema。模型看到的字段和执行前校验使用同一份契约，`oneOf`、nullable、数组长度与数字约束均受校验；不支持的 schema 关键字拒绝执行。验证一次返回字段路径列表。生成参数属于 `parameters`，转场时长属于 `set_transition.duration`，选定模型的参数能力与预设默认值随模型规则一起返回。

生成操作按图片、视频和独立参考资产分支声明。资产角色只暴露要求 `generationPurpose=asset` 和显式 `references` 的独立图片分支；视频组合 `mode` 不出现在图片参数中。模式、引用角色、状态、比例和分辨率使用有限值约束，比例与分辨率集合和模型适配器共用来源。模型说明使用 `inputDescription` 描述输入语义，避免把 `mode:image` 混入工具参数。联合类型校验根据已知判别字段定位分支，一次返回可定位的缺失、非法值和多余字段；不纠正字段别名或静默忽略参数。

Skill 的 `CORE.md` 提供专业原则，入口文档提供工作方法与条件阅读路径。模型默认收到本角色的技能目录，规则正文按需完整读取。应用启动时安装当前包中缺少的规则；运行时读取不执行初始化或迁移。角色默认值只用于首次创建配置，已保存配置直接读取，不再维护旧默认指纹、文本替换和历史迁移。数据库中的规则仍可编辑。

`desktop/src/project_service` 管理读取、授权、对象观察、事务和回执。纯 TypeScript 领域函数在构建时由 Bun 编译为嵌入包，由受内存、栈和时间限制的 QuickJS 执行；不需要随应用携带 Node/Bun，不暴露 DOM、文件或网络。界面与后端复用纯领域规则，不维护 Rust/TypeScript 两套业务实现。已删除 WebView 工具执行事件、回调和 30 秒应答超时路径。

读取和宿主快照建立本轮对象观察，写入检查实际改变的已有对象及操作依赖，失败返回准确目标与读取参数。成功后刷新相关观察，并在同一 SQLite 事务保存工程与 `project_executions` 回执。相同执行编号的重放返回原回执；编号复用不同参数会被拒绝。重启恢复优先读取回执，避免把已保存修改误报为未知结果。工程快照带宿主来源元数据，持久化保留来源；供应商请求移除该内部元数据。用户文本前缀不参与快照识别。

`mstudio_read_image` 成功返回画面时，登记实际读取快照的素材版本；片段画面也登记片段，转场合成登记两侧素材与片段。读取失败不建立观察，解码期间发生的并发修改不会被当作已读的新版本。图片主 Agent 与委派子 Agent 共用此入口，观察按各自轮次隔离。镜头的分镜图绑定等局部更新不要求读取整个剧本；创建镜头、改变剧本或段落关联时仍校验剧本观察，已读目标被修改或删除时仍拒绝整批写入。

界面自动保存提交基线与草稿，由后端对最新工程做三方合并：独立对象/字段合并，重叠编辑明确冲突。存储版本阻止乱序通知覆盖较新内容。`project-changed` 只通知界面刷新，没有执行职责。

保存的 `READY` 制作任务就是持久待执行队列。`production_queue` 跨项目按最多两个提交槽认领任务；上传与远端提交由应用持有。上传结束再次核查任务状态，取消后不会继续提交。`jobs::reserve` 在网络请求前保存提交编号；重启仅恢复没有远端占位记录的准备任务，有占位记录的任务沿用原 job 查询，绝不因超时自动创建新付费请求。远端本身不保证 exactly-once，`UNKNOWN` 保留这种不确定性。

后台 job 状态和已下载结果通过同一领域服务写回工程，页面关闭不阻止落盘。旧的页面生成扫描器、提交器、上传恢复和结果写回器均已删除。测试覆盖参数失败零写入、回执重放、并发合并/冲突、来源识别、跨重启队列恢复、取消准备和多结果幂等收取。

素材查询按 `jobs.asset/assets` 中的实际素材 ID 关联来源，返回原始提交 Prompt、模型和引用；不解析文件名，不用当前可编辑草稿代替原始请求。只展开请求的文本字段，媒体二进制与服务凭据不进入查询结果；原始文字和批量来源继续分页。

子 Agent 的执行进度以项目、父轮次、调用 ID 和子轮次关联。公开阶段、当前工具、最近进展和耗时投影到 `agent_child_activity`，由真实事件推送刷新，展开后读取该子轮次的操作记录。取消、失败、完成和重启中断分别显示，历史轮次不会跟随同一个子会话的后续任务变化。过程文字保留在完整模型历史和 `assistant/progress` 记录中，界面可展开查看，不累计拼接进最终回答；完成回传只使用最后一条无工具调用回复的最终文本，排除显式 commentary，完整结论在输出限额内直接交给父 Agent。

编辑回执的 `savedValues[].values` 返回请求字段的实际保存值，包括图片/视频提示词、分镜图和引用；数组返回合并后的完整值。生成回执与查询共用 `taskOutcome`，提供 `status`、实际 `resultAssetIds` 和 `continuation`：等待用户、等待后台、结果可检查、已结束。委派返回时刷新任务事实，长输出卸载保留这些控制字段。Prompt 和 Skill 直接使用完整回执，等待确认时报告用户动作，不重复查询。

后续检查确实依赖后台结果时，Agent 使用 `mstudio_await_generation`。工具先订阅提交事件再读取持久任务，后台状态写入、导入结果、取消、暂停、删除都会唤醒等待；无关项目事件不触发重读。等待期间没有模型轮询或远端查询，只由原有后台 worker 管理远端状态。确认阻塞立即返回，后台任务在收到实际结果、失败或需要用户动作时返回；共享本轮取消信号与时限。重启后中断的等待从持久任务恢复事实，不重新提交生成。普通回复和独立工作仍由通用 Agent 循环处理，不按读取次数截断。

设计对照：[DSH 工具契约](https://github.com/deepseek-ai/deepseek-harness/blob/639ed015397290b3745d163aafe02ffee4aa3f84/packages/core/tools/src/schema.ts)、[观察策略](https://github.com/deepseek-ai/deepseek-harness/blob/639ed015397290b3745d163aafe02ffee4aa3f84/packages/fs/fs-observation-policy/src/index.ts)、[工具调度](https://github.com/deepseek-ai/deepseek-harness/blob/639ed015397290b3745d163aafe02ffee4aa3f84/packages/core/agent-loop/src/tool-calls.ts)。工程对象、SQLite 事务和媒体任务队列是 Mstudio 的领域实现。

## 预览与导出

桌面预览由 GES 处理解码、合成与音频。macOS 优先使用官方 `vtdec_hw`，预览范围丢弃末尾 2 帧并同步钳制定位和播放结束（至少保留 1 帧），工程和导出时长不变；使用共享 BGRA 帧和嵌入现有预览区域的 Metal 视图，WebView 仅传递控制、位置和帧元数据；Windows 保留二进制 RGBA IPC 与 Canvas。Windows 已有独立的 DLL/插件打包、NSIS 安装器与 CI 检查流程，物理声音、多屏 DPI 等仍需实机验证，见[开发指南](development.md)。浏览器界面使用 HTML 媒体预览作为开发模式。导出使用 FFmpeg 合成管线；两条管线需要分别验证。

## 外部服务

模型适配器负责请求转换、凭据和任务轮询。远程服务仅接收本次请求选择的上下文与媒体；配置的端点决定数据接收方。本地凭据数据库不应上传仓库或用于公开复现。

对话支持 OpenAI 兼容、Responses、Gemini、Claude 原生协议及本机 Codex 接入。媒体供应商当前注册 `fal`、`gemini-native`、`http-json`、`codex-image`，对话与媒体能力分别校验。后台按输出索引导入全部结果，部分输出失败可继续收取；不再只导入第一张图片。附件每轮最多 12 个，原始内联素材单个最多 12 MiB、组合序列化后最多 18 MiB；文本文件最多 120 KB。原始音视频与 PDF 的可发送范围由具体模型及接口共同决定，视频抽帧不等于完整视听理解。

Codex 在一次 Mstudio 任务执行中保留原生 app-server 会话与回合。动态工具调用交由宿主执行，文字、图片或错误结果通过同一 RPC 连接返回，模型继续该回合；不再每次工具调用都重启进程、重放历史。新输入、上下文压缩或工具配置变化时，用宿主持久化的当前历史重建原生回合。取消和完成会释放连接，工程写入、回执及恢复仍由 Mstudio 管理。

预览由单个前端会话控制器管理，连续定位和倍速请求合并，播放/暂停保留操作边界。指令应答以状态完成及新目标帧到达为准。轨道名称和画布关联不参与渲染规格比较，连续参数变动停止 120 毫秒后再重建。旧会话的回调不能更新新会话；预处理缓存写入串行，已过期的等待请求跳过。隐藏助手面板时保留任务运行时，但卸载消息视图。

## Agent 任务上下文

所有内置和自定义角色共用 `assistant/task_context.rs`、`harness/session_selection.rs` 和工具执行管线。聊天长期保留，工具明细保留七天，恢复依赖独立于日志保留；模型读取任务范围内的投影。

- 消息 attribution 保存任务 ID、角色、引用对象、工作区及原始要求。同角色的后续消息继续任务；对象或工作区变化只更新当前操作上下文。对话顶部的“新任务”显式另起任务，不删除聊天或共享约束。程序不靠关键词猜话题变化。
- 会话恢复按项目、角色、任务及模型路由匹配；重试沿用原任务和已提交操作。缺少任务上下文的记录不支持重试，需重新发起任务；重置会阻止恢复旧任务。
- 普通新消息仅读取同任务、同角色最近一个已完成问答，按 turn ID 配对，不自动重放旧工具过程；显式重试或 `/compact` 才恢复匹配会话。首轮可选上下文预算为模型预算与 12,000 估算 tokens 的较小值；当前用户输入及必要规则不会为了达标而静默删除。恢复会话继续使用已有压缩机制。
- 保留共享项目约束、格式、版本和对象数量；指定目标后省略无关节点目录。`inspect` 支持 `ids/nodeIds/fields`，返回缺失对象、已省略字段和分页信息。约束截断时通过 `section=creation` 补读；`history` 支持按 `taskId` 查询。
- 子 Agent 默认 spawn 独立历史；显式 fork 保留父会话已完成历史。子 Agent 工程快照不再重复系统消息中的角色规则与技能目录。
- 编辑回执返回状态、版本、修改对象/字段及任务信息，不重复整份工程概览。普通结果超过 6,000 Unicode 字符时卸载；受限列表、含 savedClips 的编辑回执、生成等待与委派结果使用 14,000 字符限额。Skill 正文不进入通用结果卸载，完整交给模型。模型收到其他长结果的预览和 `turnId/resultRef`，通过 `mstudio_read_result` 分页回读。精简消息持久化，重启不恢复成庞大原文；图片沿用媒体工具。
- `context/usage` 记录估算组成及供应商校准后的压力；`context/selection` 记录任务和恢复条数。普通请求及摘要分别记录供应商总量和缓存输入。估算不是账单，缺失用量不是零。

测试覆盖角色/任务隔离、连续修改、新任务重试、重置、共享约束、字段读取、Unicode 回读和跨项目拒绝。此次优化不改变原始媒体附件策略，也不承诺固定比例的 token 节省。

参考：[DSH 压缩](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/compaction)、[DSH 计量](https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/llm/token-meter)、[Deep Agents](https://www.langchain.com/blog/context-management-for-deepagents)。

编辑批次在副本上执行，镜头顺序唯一性在整批结束时校验，再一次保存；失败不提交中间结果。宿主按本轮实际提供的对象保存内容哈希，忽略节点布局与任务执行遥测；不再要求模型携带工程 revision。`inspect nodeIds` 的 `fields` 真正筛选内容，排序读取使用 `title/shot.order/shot.duration`，方案镜头目录使用 `shots`。普通工具错误交给模型纠正，不再按相同错误计数强制终止。通用执行循环不设固定轮数上限，也不按媒体任务数量扩容；工具执行后继续请求模型，直到正常完成、取消、达到整轮时限或发生不可恢复错误。

### GES 与 Metal 预览

`native_preview` 只负责会话所有权，所有权锁内不等待引擎。`preview_validate` 校验输入，`preview_prepare` 编译播放计划。`preview_process` 启动同一安装包的 `--preview-worker` 子进程，子进程在初始化 Tauri 前进入工作模式；`ges_engine` 的专用线程是 GES 管线唯一的控制入口，`ges_transport` 管理播放意图，`ges_async` 在处理 GLib 和总线的同时等待管线 ASYNC_DONE；seek 还需收到对应新画面，普通暂停不等待新的 preroll。重复位置和倍速跳过 seek，定位超时最多重建一次，整体操作期限短于 IPC 超时。

控制通过带请求编号的有界二进制协议收发，逐条确认完成。macOS appsink 将 BGRA 写入三个共享内存槽；原子状态锁定正在读取的槽，像素不经过管道或 WebView。管道只发送状态和 16 字节帧头。共享文件在子进程确认启动后立即解除目录链接，映射随进程和会话回收。Metal 使用三个受 GPU 完成回调保护的纹理，通过 sRGB 着色器缩放到 Retina 显示尺寸。Windows 状态和 RGBA 帧由独立读取线程维护为最新快照。快照与应答按发送顺序捕获，避免旧快照覆盖新应答；帧不积压。

`metalPreviewFrames` 和 `previewFrames` 均保持一个在途请求，在绘制前复查目标位置。Metal 提交还验证控制代数，丢弃定位期间及旧回包中的帧；获取 drawable 不持有会话锁或控制锁。AppKit 创建、布局和销毁全部调度到主线程。原生视图跟随 DOM 区域、窗口缩放和 backing scale，鼠标事件透传给 WebView；预览被弹窗或菜单遮挡、页面隐藏及组件卸载时隐藏或移除视图。

子进程退出会使待执行指令失败并显示重新加载入口，不会直接终止主应用。关闭给予 500 毫秒退出时间，超时强制结束并回收子进程；控制超时同样终止失去响应的进程。`ORC_CODE=backup` 仍在 macOS arm64 子进程启动前设置，规避已观测到的透明混合动态代码崩溃。

GES 首版按片段缓存 FFmpeg 调色/变速，复用转场与导出音频混音逻辑；GES 负责时间线层级、字幕、时钟和 seek。切换预览清晰度不会写入工程或改变导出路径。技术取舍是保持现有效果语义，代价是效果修改后的准备时间。未来可逐项引入 GES 原生效果，但必须先验证与现有导出的一致性。

视频效果缓存按向上覆盖片段时长的帧数输出，`fps` 保留末帧，`tpad` 延续必要的尾帧；工程片段起点和长度保持原值。缓存算法版本参与键值，旧的少帧缓存自动失效。GES 定位使用 `Timeline.frame_time`；视频层用绝对终点减起点计算持续时间，帧对齐时刻采用与 GES `get_frame_time` 一致的向上整数纳秒缩放，帧数统计使用相同网格，避免一纳秒边界分歧和额外尾帧。

### 子 Agent 完成后的父 Agent 交接

所有委派统一立即返回 childId/outputRef；`oneShot`、`continuable` 都通过后续通知交付结果，不再提供前台等待参数。主 Agent 在同一轮回复中派发互相独立、写入目标不重叠的子任务，同一专业角色可以有多个子任务。分别交付的多镜头生成在输入已存在时按镜头拆分；共用参考和连续性要求通过 artifacts 与任务约束共享，不作为合并委派的理由。需要其他任务新结果或写入目标重叠时才按依赖排序；expectedGenerations 是交付核对清单，不是批量打包指令。工具调度池和全部子任务执行共用 `MAX_CONCURRENCY=4` 的上限；新建及续接子任务都必须取得执行名额，等待名额期间支持取消和截止时间。工具调度器以滚动池执行，收齐的是启动回执；主 Agent 随即继续独立工作。只剩结果依赖时结束当前执行，收到完成通知后自动接回并汇总，不轮询子任务状态。回执仍按模型调用顺序提交。共享决策先统一确定，已有工程内容通过 artifacts 交接，主 Agent 不为派发而重复读图、复述分镜。工程直接读写仍是顺序屏障；子任务内部写入通过领域服务和 SQLite 事务提交。

模型侧内容按职责装配：统筹角色只说明范围、分工与结果整合；宿主提示保留公共执行规则，参数语义归工具，专业方法归 Skill。委派参数分别说明任务内容、源资料引用、继承上下文、可继续模式及等待选项，角色目录只出现在 `agentId` 参数中。无绑定 Skill 时不注入 Skill 加载说明；分镜板格式与媒体制作规则留在专业角色的 Skill 和工具中。已授权工具从首轮直接提供，不再经过加载步骤。

后台结算先事务性保存结果与通知，再交接给仍驻留的父 Agent。父 Agent 正在执行时，在模型步骤边界读取通知；通知在最后一次模型请求期间到达，也必须先交付再进入空闲。父 Agent 已空闲时，`parent_activation.rs` 恢复其原始 session 和模型连接，并按当前角色配置刷新上下文，继续同一任务；不伪造用户消息、不重新执行已提交的工具调用。收尾阶段重新检查已提交的通知，避免结果落在“最后一步结束”和“进入空闲”之间而丢失。取消、替换或退出中的父 Agent 只保留通知，不自动唤醒。

父任务的持久状态独立于单次 IPC 和模型运行。仍有运行中的子任务或未交付的结果通知时，父轮次保存为 `waiting` 并发布 `turn/waiting`，不会发布 `turn/end`。接回结果时在同一轮次转回 `running`，所有子任务与通知处理完毕后才进入 `completed`；失败与取消分别终止。等待状态同样保留会话、事件与历史，进程重启后标记为中断，不能把失去执行器的任务继续显示为运行中。

前端订阅 `agent-execution`，用 `agent_execution_state` 恢复权威状态；事件序号防止较晚到达的旧状态覆盖新状态。初次 IPC 返回 `waiting` 后继续监听同一轮次，接收后台续写和最终结束。重新打开面板或切回窗口也刷新历史与执行状态，等待期间显示“等待子任务完成”，保留停止入口并禁止重复发送。委派卡片的活动状态跟随子轮次投影更新，原始工具回执保留调用当时的结果。

实现对照：[DSH 工具调度与说明](https://github.com/deepseek-ai/deepseek-harness/blob/5badb15009ae1756c3afe0ae0cef1faafc290ccc/packages/subagent/tool-subagent/src/index.ts)、[DSH notifySettlement/sendWaking](https://github.com/deepseek-ai/deepseek-harness/blob/5badb15009ae1756c3afe0ae0cef1faafc290ccc/packages/subagent/subagent/src/continuation-activation.ts)。父会话恢复和界面消息沿用 Mstudio 现有 SQLite journal/history。

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

On-demand storage maintenance lives in `storage/maintenance`. The scan reports allocated
bytes for the configured media folder and app data, deduplicating hard links, and
builds a preview of unused files and records. Library membership, live project
references, job/conversation snapshots, pinned tool receipts, shared global media,
previs sources/exports and content-addressed snapshot ownership are roots; ownership
rows alone are not roots. Finished film exports, unknown user files, symlinks and
files written within five minutes are excluded from deletion. Cleanup acquires the
exclusive file lease, rejects active tasks and recomputes the reviewed plan before
committing record pruning plus a durable file deletion queue. It then reclaims files,
empty managed directories and unused SQLite pages, returning actual freed space and
any retryable failures. Missing-but-referenced media remains intact as a placeholder.

Agent handoffs accept typed `artifacts` on delegation and follow-up messages.
Generation artifacts use `{kind: "generation", id: taskKey}` and resolve to the
project’s saved task metadata and result-media references, without its prompt.
Delegated artifacts provide IDs and read instructions; specialists load needed content
with their permitted tools. Explicit user attachments still supply actual media.
Chat task references likewise carry result media, IDs, status and necessary settings.
Default task/shot/asset reads omit generation prompts and asset provenance. Only an
explicit prompt edit, unchanged retry or generation investigation reads task
`fields=["prompt"]` or asset `fields=["source"]`. Historical requests remain saved
for that explicit lookup; downstream media use does not automatically load them.
The same transport applies to image, video and timeline specialists. The
host resolves project-owned node/asset/clip references and settled agent outputs;
it supplies the original user request separately from the coordinator's assignment.
`outputRef = {kind: "agent", id, turnId}` identifies the specialist's exact journal
output and write receipts. `mstudio_read_agent_result` pages that output without
rewriting it; later continuations cannot overwrite earlier turns. Project objects
remain live and include their resolution revision. Failed results retain their
status and stop reason. These references establish provenance, not factual truth.
The coordinator still chooses tasks and dependencies; specialists must inspect
original product images before making product-geometry claims.

The result and settlement notice commit together. Durable `request/source` records
preserve the latest user-request identity through inbox consumption, session
folding and continuation. Agent messages remain explicitly attributed as relays.
This follows [Anthropic's artifact handoff approach](https://www.anthropic.com/engineering/multi-agent-research-system).
[DeepSeek Harness's subagent contract](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/subagent.md)
similarly separates model-authored relays from runtime settlement notices, retains
child output and stop reasons, and enforces optional tool restrictions and output
schemas. Its schema validates structure, not product facts; it does not eliminate
semantic changes in model-authored assignments. We retain our existing scheduling
contract and add direct artifact access rather than copying its changing API.

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
Transient stream progress, skill-read notifications and startup notifications are live-only,
classified in `event_retention.rs`; stream fragments still update the assistant-message
crash checkpoint. Settled `assistant/progress` notes are durable and appear in expandable
execution details. Unknown event kinds remain durable. Active parent/child runs are
excluded from cleanup. Cleanup runs at startup and hourly while the app remains open;
settlement immediately folds recovery state. Referenced media use the existing blob
ownership and post-commit file cleanup. Migration v4 compacts existing data without a
retained backup. No cleanup path sends model requests or replays tool side effects.

模型侧按任务注册独立工具，不再暴露 `mstudio_edit.operations`：视频/图片生成、生成草稿、任务编辑、剧本和镜头结构各自使用独立参数。角色权限同时裁剪工具与字段；宿主将参数转换为既有内部操作，复用事务、权限和回执恢复。镜头重排用 `mstudio_update_shots(items)` 原子提交。界面及恢复流程识别新工具名，并兼容历史回执。内部操作参数仍在操作分支及字段本身声明用途、层级、单位、枚举和省略语义。`mstudio_save_image_generation` / `mstudio_save_video_generation` 将 prompt 和参考输入一起保存到既有生成任务（内部 `request_generation.draftOnly=true`），状态为 `DRAFT`，自动模式也不执行。同镜头同媒体类型再次保存复用未提交草稿，生成时沿用该任务及省略的参考输入；已知任务用 `update_generation` 原子更新 prompt/references。旧 `shot.prompt/framePrompt` 只保留读取兼容，不再向 Agent 暴露写入；新图片卡片不复制 prompt。已执行任务的原始请求快照保留用于溯源。图片与视频分支分别展示参数；媒体能力查询与操作契约复用 `parameterSchema`，执行侧仍按所选模型校验，不把镜头时长或工程导出规格当作生成规格。

## Private Three.js previsualization

`desktop/src/previs` is a separate file workspace with a scoped text editor tool. It does not register project Agent tools, use Agent profiles/Skills, assemble a project snapshot, or write Agent history. `previs_files`, `previs_messages`, `previs_exports`, and `previs_media` have project ownership and cascade with project deletion. Edits replace one stable `previs/<project>/<file>/model.js` or `scene.js` source; revisions are internal concurrency checks, not separate user files. Older version directories are no longer written. Project model assets appear in their own media category; they are not ordinary `Project.assets` or canvas nodes.

A named model module exports `createModel({THREE})` and returns a fresh Object3D with named movable parts in `userData.parts`. Each shot exports `createShot({THREE,models,seed})` and returns `scene`, `camera`, `update(t)`, and optional `dispose()`. `models.create(idOrAlias)` resolves the shot's declared shared assets. Saved aliases bind to stable asset IDs, so a model rename does not break existing assembly code. Model edits invalidate renders for dependent shots; unrelated models do not invalidate them.

The Runtime serves a frozen snapshot from a token-scoped, loopback-only resource server. Three.js is vendored with its license and requires neither a CDN nor Python. The cross-origin iframe has no Tauri IPC privileges; its CSP allows only its own module resources, and parent messages check source, origin, and session identity. Runtime executes code and renders; runtime errors do not trigger an automatic repair loop. Each edit starts with the current file, selected model ID/name/usage inventory, current instruction and Runtime contract. Stored previs messages remain visible but are never sent as model context. Other shot files are unavailable. Existing model implementations can be read on demand through the editor.

Previs runs a dedicated Agent using the shared DSH-style driver and text-only provider transport. The model receives only the current file, selected shared models and current instruction, with no main Agent context or conversation history. Its `str_replace_editor` tool follows the DSH command contract: `view`, `create`, unique `str_replace`, and `insert` with `new_str`, nullable unused fields, strict line ranges and numbered, bounded reads. Errors become native tool results with `is_error`; successful source writes use the existing revision-checked, atomic persistence layer immediately, before returning success to the model. The loop ends normally when the model has no tool calls. There is no final file acceptance gate, edit envelope or whole-turn rollback. Failures and cancellation preserve completed edits, and the frontend refreshes the saved workspace in either case.

The scoped workspace contains `/scene.js` or `/model.js`, plus read-only JS sources for the shot's selected shared models. New shared models are single `/models/<name>.js` files; the host derives the display name and alias from the filename, creates a stable asset ID and binds it to the current shot during that operation. No model-authored metadata JSON or paired files are required. Existing names, descriptions and bindings stay in host-owned project storage; the shared-model UI selects additional existing models. This is Mstudio's storage adapter, not part of DSH's generic filesystem contract. The view reuses the main Agent's message, composer and activity styles with a separate request-scoped stream.

Export samples frames at `frameIndex / fps`, sends one bounded PNG at a time through binary IPC, and uses the existing bundled FFmpeg encoder. A render freezes the shot and all referenced model revisions. Each shot keeps one latest successful export under a stable result ID. A successful regeneration atomically replaces that result and retargets shot references, preserving purpose text and valid custom trims; failures or stale renders leave the existing result intact. Superseded private renders are retired only when no task, conversation or timeline snapshot still references them. Saved library copies retain their identity and membership independently. Exported videos remain private and never become shot takes automatically. Video cards expose separate “Use as reference” and “Save to project media” actions. The first atomically links an immutable private media ID to a shot without importing or copying a file. The second imports a distinct media-only copy into ordinary assets and explicitly collects it; repeated saves reuse that copy. Private media metadata is resolved from `previs_media` through the host-derived `Project.previsAssets` view, stripped before persisting the project document. Normal asset inventories and library ownership exclude this view, while reference readers, generation and uploads resolve both sources with project scoping. Export identity is stable, media snapshot identity changes per successful render; only live shot references are retargeted. Source code, models, and private chat remain private. Storage migration moves source directories and rewrites private export paths without moving the SQLite database.

Frontend task state belongs to a project session, independent of its dismissible editor dialog. Closing the dialog or opening another shot preserves the submitted request, status, retry text, and result. Previs tasks appear in the existing toolbar task list, with active and failed counts; they never place floating cards over the workspace or composer. Only the explicit Stop action cancels a model request. Video capture uses a separate offscreen Runtime with frozen inputs, so dismissing the visible preview cannot dispose the renderer. Frame capture requires the project webview to remain mounted; leaving the project releases an unfinished export. A model request already submitted to the host can still complete and save after project navigation. Loading indicators, elapsed time, phase steps, and measured frame progress use shared visual and motion tokens.

主动读取媒体由 `media-read` 单独授权，并依赖 `project-read`；素材目录、文本和状态仍使用 `project-read`。统筹默认没有原图、视频帧及历史图片的主动读取工具，专业角色保留原有权限。用户在聊天中明确发送的附件照常提供给所选模型，并可转交专业角色，不受主动读取开关影响。新建配置使用当前默认权限，保存和读取不进行自动迁移。

附件生命周期与上下文压缩分开：预算内的图片、工具返回画面、音视频、PDF 及带宿主来源标记的文本文件继续随有效历史发送，模型响应仅推进可回收的消费边界。图片字节预算或模型上下文预算触发卸载，最旧的必要输入出现位置变为可重读引用；尚未处理的新附件和助手输出保留。已省略位置通过事件和快照持久化，续接、唤醒与 fork 不会自动复活；重新打开只增加新的输入出现位置。原文件及附件存档不删除。文本附件标记只供宿主识别，发往模型前移除。同一子任务仅首次转交某条用户消息的附件，重复指令只转交要求；新的用户消息明确附带附件时可以重新提供。

普通聊天、首次委派、子任务续接和父任务唤醒共用 `assistant/turn_context.rs` 装配角色、项目状态与模型规则。首次委派和后续消息共用附件解析：用户明确发送的附件提供实际媒体并去重，Agent 选择的 artifacts 只传保存对象引用与读取入口，由专业角色按需读取。无工具的视频拆解角色仍直接收到交接原视频。原始用户要求统一读取持久化的 `production.instruction`。通知按项目、角色及任务身份筛选，同任务跨轮次可以收取，新任务不会消费旧任务通知。已移除无消费方的“待检查结果”台账；任务状态、写入回执及版本冲突检查仍由原领域服务负责。

## 媒体预算与历史卸载

### 问题与修复

截图对应的 concept 子任务读取 8 张产品图后，又调用了 14 次 reopen_image；22 次调用均成功。模型已经描述过图片，却在后续看到卸载占位后反复怀疑是否看过。旧实现每次模型响应后即卸载此前全部媒体，重新打开后又生成新的随机引用。

现在模型回复仅推进消费边界，预算内的图片及其他已读附件继续保留。请求图片超出字节预算时只省略最旧、足够满足预算的输入图片出现位置；模型上下文超预算时逐个回收最旧媒体，达到预算即停止。助手输出不参与媒体卸载。最新尚未被模型消费的整个批次受保护，超大新批次会明确报错，不静默删除证据。

卸载使用现有媒体 journal 与 blob 存储。恢复引用采用序列化内容的 SHA-256，同一内容重新打开、再次卸载不会产生新的标识。引用仅接受当前内容哈希格式，卸载统一使用 media/offload；旧随机引用及 image/offload 事件不再恢复。省略的旧出现位置在后续请求、恢复和检查点折叠后保持省略；重新打开只加入一个新的媒体输入，不自动复活旧位置。

### DSH 源码对照

固定参考提交：`571902028a75c64c1d767f4ae5fd099485f0ddf0`。

- [`content.ts` 的 requiredImageOffload/offloadedImagePrefixCount](https://github.com/deepseek-ai/deepseek-harness/blob/571902028a75c64c1d767f4ae5fd099485f0ddf0/packages/llm/llm/src/content.ts)：按请求版本实际表示字节计算出现位置，取足够释放超额的最旧前缀。Mstudio 的 `harness/image_budget.rs` 移植此最小前缀计算，Base64 按编码后长度计算，Raw 按 Base64 展开长度计算。
- [`image-offload.ts`](https://github.com/deepseek-ai/deepseek-harness/blob/571902028a75c64c1d767f4ae5fd099485f0ddf0/packages/compaction/compaction-image-offload/src/image-offload.ts)：按请求顺序选择用户输入与工具结果，跳过助手输出；持久记录所选出现位置。
- [`index.ts`](https://github.com/deepseek-ai/deepseek-harness/blob/571902028a75c64c1d767f4ae5fd099485f0ddf0/packages/compaction/compaction-image-offload/src/index.ts)：只处理明确的图片预算拒绝，不以“已回复”作为触发条件。

### Mstudio 适配边界

Mstudio 仍由 Rig SDK 与 Codex transport 构造请求。DeepSeek 官方服务的 Chat、Responses、Anthropic 协议在 HTTP 边界将内联图片投影为 Files API 引用；会话 journal 保留原图片，不保存临时远端 ID。DeepSeek 使用 200 MiB 原始图片总量和 600 个出现位置预算；OpenAI Responses 使用 512 MiB 和 1,500 个出现位置；其他服务内联图片额度为 16 MiB，保留现有 18 MiB 附件入口额度中的文字与结构余量。远程 URL、File ID 与 Codex localImage 不计作内联字节，仍受模型上下文预算约束。

Mstudio 额外保护最新尚未消费的批次。DSH 在适配器拒绝后重试；Mstudio 在请求前按现有输入格式检查预算，避免浪费一次供应商请求，不占网络重试额度。摘要来源是已选择的旧历史，仍保留原有工具配对和近期消息规则。

本次修复不把 PDF、音视频、文本文件统一改成 DSH 文件系统句柄。它们继续使用现有受权限约束的素材读取和模型能力校验，但不再因一次响应被无条件卸载。

### 验证

回归场景包含：8 张产品图分 3/3/2 批读取后在最终请求中全部存在；预算刚好满足时不卸载；重复图片按出现次数计费；超预算只卸载最旧前缀；嵌套工具图片、Raw 与 data URL；最新批次与取消保护；上下文校准后只回收足够的旧媒体；项目隔离、检查点恢复，以及重新打开后引用稳定。

### 通用 uploader

`assistant/uploader` 共用内容寻址、持久索引、并发去重、提前刷新及一次失效 ID 重试；`backends/openai.rs`、`backends/deepseek.rs` 实现平台能力、上传参数、ID 校验和请求引用格式。共同上传流程使用 `POST /files` 和 7 天有效期。DeepSeek 使用 `purpose=user_data`；OpenAI 图片使用 `purpose=vision`，普通文件使用 `user_data`。通用 `Source` 接受文件字节与媒体类型，目前会话请求自动投影的调用方是图片。请求版本按媒体类型与实际文件字节计算 SHA-256；SQLite `provider_uploads` 保存平台、端点与凭据命名空间摘要、版本摘要、file ID、字节数及有效期，凭据原文不进入索引。提前 1 小时刷新，同进程并发 agent 在共享版本锁内二次检查持久索引，避免重复上传。

仅在官方 HTTPS DeepSeek 和 OpenAI 服务启用；URL 引用不下载、不上传。OpenAI 的图片上传接入 Responses API，不套用 DeepSeek Chat 的 file 内容格式，见 [OpenAI 图片文件引用](https://developers.openai.com/api/docs/guides/images-vision)。实际对话请求继续使用当前协议：[Responses 的 input_image.file_id](https://api-docs.deepseek.com/guides/responses_api/)、Chat 的 file/file_id 或 Anthropic 的 image/source/file。Anthropic 请求附带 Files beta header。请求投影不修改原消息，工具调用配对、原始像素恢复和上下文视觉 token 计量继续有效。

上传不自动降级为 Base64。认证、限流、配额、图片格式或回执错误按实际失败返回；服务拒绝已过期、已删除或不存在的 file ID 时，精确失效当前代索引并重新上传，只重试一次，第二次失败仍移除坏映射并返回。HTTP 客户端禁止重定向，网络取消随调用 future 丢弃传播。文件到期由服务释放，不主动删除账号中的其他文件。

Codex 0.162.0-alpha.17.2 的 app-server schema 中，turn/start 的 image 支持已有 `fileId`，dynamic tool 的 inputImage 只支持 `imageUrl`；未发现返回图片 file ID 的通用上传 RPC。thread/attachment/add 保存附件类型、标识与 payload，不是 Files 上传。当前继续使用 Codex 本地图片输入，不借用 OpenAI API key 或抽取 Codex 登录凭据。

测试覆盖 OpenAI Responses 上传与复用、平台隔离、持久复用、端点和凭据隔离、并发单次上传、到期前刷新、三种请求格式及工具图片、普通与流式传输的失效恢复、错误不降级，以及新格式引用校验。
