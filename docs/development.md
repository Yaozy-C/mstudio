# 开发指南

## 环境

- macOS，Xcode Command Line Tools，Rust 1.97.1（由 `rust-toolchain.toml` 固定，Rust 2024 edition）。
- Bun：前端开发及测试；提交 `frontend/bun.lock`，安装时使用 `--frozen-lockfile`。
- Python 3.12+、CMake、pkg-config，以及 FFmpeg、SDL2、libxml2 开发库。
- `ffmpeg` 和 `ffprobe` 必须在 PATH 中，渲染测试使用合成媒体，不需要个人样片。

根目录和 `desktop/` 是两个 Cargo package，各自维护 `Cargo.lock`。

## MLT 原生 SDK

```sh
python3 scripts/build-mlt.py
python3 scripts/bundle-mlt.py
```

构建脚本使用 `scripts/mlt-source.json` 固定 MLT 官方提交与 SHA-256，再构建所需模块。当前固定提交 `06c4785f951c087c700de942362d1d1c68ffe500` 包含 [FFmpeg 9 支持修复 #1281](https://github.com/mltframework/mlt/pull/1281)，该修复尚未包含在 7.40.0 正式版中。构建目录按提交隔离，避免旧 CMake 缓存引用另一份源码。源码下载到 `desktop/native/vendor/`，SDK 安装到 `desktop/native/runtime/`；这些目录不提交。打包脚本将运行库及其依赖复制到 `desktop/native/bundle/`，不修改系统安装。

可设置 `MLT_SDK` 使用已有兼容 SDK。原生构建和 Tauri 资源需要 SDK 与 staged bundle，因此直接跳过这两步运行桌面 Cargo 检查会失败。

## 开发与验证

```sh
(cd frontend && bun install --frozen-lockfile)
sh scripts/dev.sh
```

完整检查（先准备 MLT）：

```sh
sh scripts/check.sh
python3 scripts/test-mlt.py
python3 scripts/test-native-player.py
python3 scripts/test-detached-audio.py
```

独立检查：

```sh
(cd frontend && bun run build && bun test)
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo clippy --manifest-path desktop/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path desktop/Cargo.toml --locked
```

`test-mlt.py` 使用合成视频验证实际解码、叠加、不透明度、时长与变速音高。`test-detached-audio.py` 对比原速及变速片段分离前后的实际 PCM，验证音量、起点和静音行为。`test-native-player.py` 在 macOS 使用真实 MLT 和 SDL dummy 音频设备，验证定位、暂停、连续定位、末尾重播以及后台回收后重开；不接触用户工程。物理显示流畅度、声画同步、文件选择器和真实模型服务仍需桌面人工验证。

新代码建议拆分为不超过 300 行的模块。`scripts/source_size_baseline.json` 记录首次开源时已有的超长文件；检查禁止新增超长文件或扩大已有文件，重构时应降低或移除对应额度。

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

生成 `Mstudio.app`。此脚本更新本地生成的应用包，不发布 GitHub Release。仓库的 MIT 许可不替代 FFmpeg、MLT 及其传递依赖的许可；发布二进制前需核对实际依赖构建配置、保留其完整许可和所需材料。当前 bundler 只复制 MLT 的许可文件，不代表已经收集所有第三方分发材料。

## Windows 状态

原生 HWND 播放表面已有源码，但尚未完成 Windows 编译、安装器、DPI、声画同步与退出流程的实机验证。需要匹配的 MSVC MLT SDK，配置 `MLT_SDK` 后运行 `scripts/bundle-mlt.py`，使用生成的 `desktop/native/windows-resources.json` 作为 Tauri 附加配置。根目录 `.command` 和 shell 启动入口面向 macOS。
