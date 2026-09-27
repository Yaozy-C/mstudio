# Mstudio

本地画布视频工作室：用 Agent 编写脚本和分镜，在画布组织素材，在多轨时间线剪辑，并用本机 FFmpeg 导出视频。

项目处于开发阶段。当前主要开发与验证平台为 macOS；Windows 原生预览已有实现，但尚未完成构建和实机验证。Linux 暂不支持桌面原生预览。

## 功能

- 脚本、分镜、参考图片和生成结果在同一项目中管理。
- 画布图片、视频可以通过右键菜单、预览窗口或拖放加入时间线。
- 多轨剪辑、变速、混音、字幕、MLT 原生预览和 FFmpeg 导出。
- 可配置模型服务与 Agent，支持媒体生成任务及本地 Skills。
- SQLite 自动保存项目、对话和任务状态。

模型服务需要自行配置账号或 API Key，可能产生服务商费用。浏览器开发页面只支持界面预览，文件导入、原生播放、生成任务等功能需要桌面应用。

## 从源码运行（macOS）

安装 Rust 1.97.1（由 `rust-toolchain.toml` 固定）、Bun、Python 3.12+ 和 Xcode Command Line Tools。原生依赖可通过 Homebrew 安装：

```sh
xcode-select --install
brew install cmake pkgconf ffmpeg sdl2 libxml2
```

```sh
git clone https://github.com/Yaozy-C/mstudio.git
cd mstudio
(cd frontend && bun install --frozen-lockfile)
python3 scripts/build-mlt.py
python3 scripts/bundle-mlt.py
sh scripts/dev.sh
```

首次构建需要网络下载依赖及固定版本的 MLT 源码，原生构建可能需要几分钟。开发服务器使用 `127.0.0.1:1430`。详细环境、检查与平台限制见 [开发指南](docs/development.md)。

## 检查与打包

```sh
sh scripts/check.sh
sh scripts/bundle.sh
```

打包完成后生成仓库根目录的 `Mstudio.app`，可直接打开或运行 `Mstudio.command`。应用包、依赖和构建缓存不纳入 Git。

## 仓库结构

| 目录 | 内容 |
| --- | --- |
| `frontend/src/` | React / TypeScript 界面、画布、时间线和 Agent 交互 |
| `src/` | Rust 媒体探测、合成、代理和导出核心 |
| `desktop/src/` | Tauri IPC、SQLite、模型连接和任务管理 |
| `desktop/native/` | C++ / Objective-C++ 原生预览源码 |
| `skills/` | 应用随包发布的八个创作技能及资源 |
| `tests/`、`frontend/tests/` | 自动测试及开发测试页面 |
| `scripts/`、`examples/mlt_graph.rs` | 检查、原生构建和合成验证工具 |

仅提交源码、必要的应用资源、测试、锁文件及维护文档。设计原型、review 截图、用户样片、工程数据库、生成媒体和密钥由 [.gitignore](.gitignore) 排除。Skills 是运行资源，需要随源码和应用一起分发。

更多实现说明见 [架构说明](docs/architecture.md)。

## 转场与调色 Agent

底层画面轨上，两段相邻片段的接缝会出现转场按钮。点击选择叠化、黑白场、推移、擦除等 12 种效果，调整时长，或点击「让转场 Agent 设计」。对话也可选择「转场设计」与「调色师」，引用片段后描述目标。调色面板的自然语言请求交给调色师处理。

转场由 FFmpeg 实际渲染并缓存，桌面 MLT 预览与导出使用相同效果。当前支持全幅、不透明的底层画面片段；时长 0.05–3 秒，不超过任一相邻片段。转场保持剪辑、声音与字幕时间，素材余量不足时延展边缘帧；不自动做光流、蒙版遮挡或音频交叉淡化。浏览器预览暂时显示直接切换。

技能依据 CapCut、Blackmagic Design 和 FFmpeg 官方资料整理，强调镜头匹配与画面验证。当前调色提供亮度、对比度、饱和度与冷暖调整，不等同于完整专业色彩管理系统，也不保证自动达到大师级效果。运行 `python3 scripts/test-transitions.py` 可验证真实渲染、缓存和预览／导出一致性（需本地 MLT runtime）。

## 数据与隐私

macOS 默认数据目录为 `~/Library/Application Support/local.mstudio.canvas/`，包括数据库、导入素材、预览和导出。备份时先退出应用，再复制整个目录。自动保存不等同于历史版本备份。

模型凭据保存在本机数据库中，当前不是系统钥匙串加密存储。不要分享数据库、完整应用数据目录或包含凭据的日志。调用远程模型时，所选提示词和参考媒体会发送给所配置的服务商。

## 参与贡献

请阅读 [贡献指南](CONTRIBUTING.md)。报告问题时提供系统版本、复现步骤和经过脱敏的错误信息；不要附带私人素材或密钥。安全问题请按 [安全说明](SECURITY.md) 私下报告。

## 许可证

本项目采用 [MIT License](LICENSE)。第三方依赖和品牌图标保留各自的许可证与商标权，见 [第三方声明](THIRD_PARTY_NOTICES.md)。
