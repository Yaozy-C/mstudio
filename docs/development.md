# 开发指南

## 环境

- macOS，Xcode Command Line Tools，Rust 1.97.1（由 `rust-toolchain.toml` 固定，Rust 2024 edition）。
- Bun：前端开发及测试；提交 `frontend/bun.lock`，安装时使用 `--frozen-lockfile`。
- Python 3.12+、pkg-config，以及 FFmpeg、GStreamer（含 GES）开发库。
- `ffmpeg` 和 `ffprobe` 必须在 PATH 中，渲染测试使用合成媒体，不需要个人样片。

根目录和 `desktop/` 是两个 Cargo package，各自维护 `Cargo.lock`。

## GES 运行时

```sh
brew install pkgconf ffmpeg gstreamer
python3 scripts/bundle-ges.py
```

GES 是桌面预览的必需依赖。开发插件链接位于 `desktop/native/ges-dev-plugins/`；打包副本位于 `desktop/native/ges-bundle/`，均不提交。构建前需完成资源准备。

## 开发与验证

```sh
(cd frontend && bun install --frozen-lockfile)
sh scripts/dev.sh
```

完整检查（先准备 GES）：

```sh
sh scripts/check.sh
python3 scripts/test-detached-audio.py
python3 scripts/test-visual-effects.py
python3 scripts/test-transitions.py
python3 scripts/test-circle-transitions.py
```

## 可编辑调色、转场与抽帧

`Visual.grade` 保存独立的 SDR 调色参数；`src/grading.rs` 将 mlight 的逐像素明暗、八色 HSL、固定控制点曲线、分区色轮思路提取为无图像/桌面依赖的颜色函数。`grade_lut.rs` 按算法版本和参数生成 33³ `.cube` 临时缓存。项目只保存参数，不保存机器相关 LUT 路径；缓存丢失时可重建。`visual.rs` 为 GES 和 FFmpeg 提供同一滤镜定义，旧 visual 参数仍在新方案之后执行，保证已有工程不被静默重调。该链路是显示 RGB 创意调整，不是 Log/HDR 输入转换、RAW 显影或局部跟踪蒙版。

`Transition.design` 是结构化组合方案：进度控制点、整体/方向/径向混合、羽化、中心、两侧缩放和位移。`transition_design.rs` 校验并编译 perspective 逐帧运动与 xfade 混合表达式，`transition_render.rs` 用 FFmpeg 渲染接缝；原生播放与导出共用已有转场缓存机制。方案随工程保存、可撤销，未开放任意 shell、脚本或滤镜字符串。移动采样不是光流、真实摄影机运动或运动模糊；画幅外采样延展边缘像素，过度偏移可能拉出条带，需要看预览修正。

现有 `mstudio_read_image` 支持视频 `time`；带 `clipId` 时按片段内时间换算源裁切和速度，应用已保存调色。带 `transition:true` 时以指定后片段的转场开始为时间零点，读取与原生预览共用的实际接缝合成。输出最多 960 像素边长，沿用原有图片工具结果、历史与恢复通道，不新增 agent 调度层。抽帧不含字幕、叠加轨或声音，不等于完整播放验收。只支持图片的模型可以接收视频引用并按需抽帧；原生视频模型仍可接收原视频。

时间线增加 `move_clip`、`retime_clip`、`slip_clip`；都走现有 revision 校验、原子编辑和撤销。变速保持源区间，ripple 只顺移同轨后续片段；跨轨音频/字幕同步须显式安排。默认拒绝新重叠、非法轨道和不足的源余量。前端/后端、抽帧、真实 GES/FFmpeg 像素与缓存测试分别覆盖这些边界。`scripts/check.sh` 中本地 HTTP 模拟测试需要允许监听回环端口，不使用外部模型账号。

独立检查：

```sh
(cd frontend && bun run build && bun test)
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo clippy --manifest-path desktop/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path desktop/Cargo.toml --locked
```

`ges_composites` 测试使用真实 GES 管线验证透明字幕、定位、暂停、末尾重播与销毁重开。圆形展开/收拢使用像素坐标圆形遮罩与 2 像素羽化，进度覆盖到最后一个编码帧；短转场按帧数输出，素材尾部先解码预留帧再补帧，避免空输入。`test-circle-transitions.py` 检查横竖画幅、24/30/60 fps、0.05–1 秒转场的每帧单调性和首尾画面。`ges_playback_frames` 示例可记录实际播放的连续帧，补充暂停寻帧测试。`ges_seek_stress plan.json` 连续执行 800 次暂停、定位、播放；原生播放器在回执前等待 GES 状态切换和寻帧完成，避免快速定位相互冲突。macOS 预览禁用 `vtdec` / `vtdec_hw`，改用随包软件解码器：VideoToolbox 在转场叠层寻帧时可返回不透明黑帧。此回归需要在与桌面应用相同的系统权限下检查；隔离环境可能直接回退软件解码而掩盖问题。预览错误后可点击「重新加载预览」恢复。前端在目标帧实际绘制后才显示新预览，寻帧期间及暂停时丢弃不属于目标位置的旧帧；`ges_seek_pixels plan.json first-frame last-frame` 可在已知非黑画面区间反复定位并检查像素，避免只检验命令是否成功。

`test-detached-audio.py` 对比 GES 使用的混音缓存中分离前后的实际 PCM，验证音量、起点和静音行为。物理显示流畅度、声画同步、文件选择器和真实模型服务仍需桌面人工验证。

`test-visual-effects.py` 使用合成素材验证黑白、复古、柔焦、暗角和调色的原生预览与 FFmpeg 导出，比较实际输出像素。音轨波形在后台按素材采样并缓存，裁切、变速和时间线缩放复用缓存。

双击视频片段，在右侧「调色与特效」选择效果或调整亮度、对比度、饱和度、色温。「创作 → 字幕 / 配音」可选择字幕字体、颜色、字号和底色；拖动位置示意框中的文字调整成片位置。字幕样式随项目保存，并烘焙到导出视频；SRT 仅保存文字与时间。

所有维护的源码、测试、规则和配置文件不得超过 300 行，检查不再提供历史豁免。前端按组件与交互职责拆分，后端按业务职责拆分；依赖锁文件和第三方生成文件由工具管理。

格式化：

```sh
cargo fmt --all
cargo fmt --manifest-path desktop/Cargo.toml
(cd frontend && bun run format)
```

## 打包

```sh
sh scripts/bundle.sh
```

生成 `Mstudio.app`。此脚本更新本地生成的应用包，不发布 GitHub Release。仓库的 MIT 许可不替代 FFmpeg、GES 及其传递依赖的许可；发布二进制前需核对实际依赖构建配置、保留其完整许可和所需材料。当前 bundler 复制 SDK 中可用的许可文件，不代表已经收集所有第三方分发材料。

## Windows 状态

Windows 的 GES 运行时打包、安装器与实机验证尚未完成。当前打包脚本面向 macOS，不能据此宣称 Windows 已支持。

## Agent 调色与属性栏

### 对话上下文与工具往返

普通用户消息只携带同任务、同角色最近一轮已完成问答，另加当前要求、项目摘要和记忆；不重放旧工具过程，旧聊天仍可用 history 查询。显式“新任务”不携带上一轮。中断恢复仍重放原执行日志及写入回执，`/compact` 仍可处理原会话；显式 fork 按原约定取历史，独立 spawn 不继承父历史。工程摘要不再把很早的 originalInstruction 注入为当前任务目标。

inspect 的 clips/assets/tracks/captions 每页最多12项，按12KB内容预算提前分页，单项超限仍由原结果卸载机制处理；生成任务保持5项分页。小组编辑可一批读取，fields 用于省略无关字段。edit 在保存成功后返回 savedClips，列出实际变化（包括顺移邻居、失效转场和删除），最多12项，complete=false 时按需补读。它是参数回执，不代表画面或声音验收；调色/转场仍要抽帧复查。工具输出层允许这些受限页面/回执直接返回，避免刚合并又被卸载为额外读取。

上下文回收以最近一次 assistant 响应为消费边界：最新一批工具结果及附件在至少一次模型请求前，不能被图片卸载、长结果裁剪或历史压缩提前移除。批量读取和 reopen_image 共用此规则。委派返回必须区分 applied（保存了修改）与 ok/stopReason（任务是否完整完成）；本轮存在未完整完成的子任务时，最终答复由执行器追加状态说明，防止模型只报保存成功。

依据：[Anthropic 工具设计实践](https://www.anthropic.com/engineering/writing-tools-for-agents)建议按实际工作流合并操作、提供有用的返回值并根据冗余调用调整分页；[Gemini function calling](https://ai.google.dev/gemini-api/docs/function-calling)支持一轮返回多个独立调用。此处复用已有批量 operations 和多调用调度，不新增代码执行沙箱或改变角色分工。轮次减少需以模型实际执行验证，不能通过省略抽帧验收获得。

选中视频后，属性栏按画面、调色、声音、时间分类。调色页填写要求并交给 Agent，应用将当前片段作为引用加入对话草稿；发送前可修改要求。模型必须支持引用素材的输入类型。Agent 通过 `update_clip.visual` 合并结构化调色参数，`null` 清除画面效果；工具拒绝未知字段、越界值、音轨调色和过期 revision。它复用 FFmpeg/GES 渲染，不执行模型提供的任意命令。

「对比原片」仅临时移除当前片段的预览调色；退出调色页后恢复，不写入项目，也不改变导出。Agent 对话中保留选中片段与返回属性入口。

## 界面多语言

应用支持 `zh-CN`（简体中文）和 `en`（English）。通用设置 → 语言可即时切换；选择保存在本机 `mstudio-language` 偏好中。首次打开跟随系统语言：中文使用简体中文，其余语言使用英文。无法保存偏好时，本次会话仍可切换。

The interface supports Simplified Chinese and English. General → Language applies changes immediately and remembers the choice locally. On first launch, Chinese system locales select Simplified Chinese; other locales select English. Switching still works for the current session when local storage is unavailable.

## Adding interface copy

- Use `useLanguage()` in components that display localized copy. It subscribes to language changes without remounting components or resetting drafts.
- Use `t("中文原文")` for application copy and add its English translation to `frontend/src/i18n/en.json`. Chinese source text is also the fallback.
- Use complete messages with named placeholders for dynamic values: `t("{kind}模型", { kind: t(mediaLabels[kind]) })`. Avoid composing sentences from separately translated fragments.
- Translate static catalog labels when displaying them, not when loading the catalog or saving data. Keep model IDs, task status codes, API payload keys and comparisons independent of language.
- Do not translate project names, scripts, prompts, media names, chat messages or model responses. The `agentLabel` helper translates built-in Agent names and descriptions only if their values still match the shipped defaults. Edited labels remain verbatim.
- Keep original diagnostic details available. Error summaries and recovery actions use localized application copy; unknown provider messages remain in the details.
- Use the selected language when formatting dates. Update `document.documentElement.lang` through the shared language store.

## Verification

Run `cd frontend && bun run build && bun test`. Localization tests cover persisted choice, unavailable storage, both rendering languages, preservation of user content, translation coverage and placeholder parity.

Manually check General → Language in both directions, reload to confirm persistence, and inspect long English labels in the project library, settings, creation canvas and timeline. Browser preview cannot verify native file dialogs, native playback or actual generation services.

## Canvas 预览与 GES 后端

桌面预览统一使用 GES，可在成片预览标题旁选择「流畅」（长边 640）或「清晰」（长边 1280）。这只改变预览，不改变工程分辨率或 FFmpeg 导出。播放器通过有界的最新 RGBA 帧交给 WKWebView Canvas；没有逐帧图像编码。弹窗、菜单和画面处于同一 WebView，不再需要原生视图坐标和遮挡检测。二进制包头为 4 个 little-endian u32：序号、宽、高、时间线帧号，之后为连续 RGBA。前端只允许一个在途请求，过期会话不能取帧或控制新播放器。

GES 使用官方 Rust 绑定和独立工作线程，所有 GES 对象与 GLib 默认上下文归该线程；appsink 在暂停时也接收 preroll。GES 排布逐片段视频层、透明字幕、黑场，并播放同一管线内的音频。为了与现有 FFmpeg/GES 效果保持一致，调色及变速视频按片段预计算缓存；转场复用原缓存，声音使用导出混音定义预混（保调变速、淡入淡出、静音、音量和限幅）。因此首次准备或修改效果可能有准备时间，本版不是 GES 原生实时特效实现。缓存按参数、尺寸、源文件路径/大小/修改时间区分，项目原始素材不变。

macOS 开发需要 `brew install gstreamer`，随后 `python3 scripts/bundle-ges.py`。GES 作为必需依赖，不再提供后端切换。GES 库和插件打包到 `Resources/gstreamer`；开发插件链接与打包副本分开，避免同一进程加载两份 GLib/GStreamer。`scripts/bundle.sh` 会完成资源打包与可执行文件链接修正，不依赖目标机器安装 Homebrew。Windows GES 运行时打包尚未实现和验证。

```sh
cargo test --test preview_ges
cargo test preview_validation
GST_PLUGIN_SYSTEM_PATH_1_0="$PWD/desktop/native/ges-dev-plugins" GST_PLUGIN_PATH_1_0= GST_REGISTRY_FORK=no cargo test --manifest-path desktop/Cargo.toml --bin mstudio-desktop ges_composites
```

GES 集成测试使用合成素材，验证透明字幕、准确定位、暂停、两轮播放和销毁重开；准备测试检查变速时长、灰度像素、缓存复用与源文件不变。`examples/ges_plan.rs` 和 `desktop/examples/ges_play.rs` 可验证只读工程快照，快照不应提交。播放器计数不等于屏幕 vsync 或精确音画同步测量。

## 播放器隔离回归

打包后运行 `python3 scripts/test-preview-worker.py`，验证实际安装包子进程的音频管线、全部八个倍速档位、定位、播放中切换速度及关闭。`preview_process_tests` 使用受控子进程模拟退出、挂起和初始化中关闭；前端 `nativePreviewController.test.ts` 验证旧状态回包、加载期间最新意图与旧会话失效。实际应用验收还应确认预览子进程被终止后主窗口仍可操作，且“重新加载预览”能恢复播放。

完整检查为 GES 测试进程设置 `ORC_CODE=backup`，与应用的 macOS arm64 回退路径保持一致；Rust 测试入口不会执行桌面 `main()` 的运行时配置。
