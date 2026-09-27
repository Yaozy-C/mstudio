# 调色决策与工具边界

Mstudio 的 SDR 视频链路提供：brightness（-0.5..0.5，默认0）、contrast（0.5..1.5，默认1）、saturation（0..2，默认1）、temperature（-1..1，默认0），以及一个可选 effect。它们由 FFmpeg eq/colorbalance 等执行，MLT 预览使用相同参数。这里没有宣称 RAW 显影、HDR 母版、校准监看、限定器、跟踪蒙版、矢量示波器、完整节点树或色彩管理。素材已经裁掉的高光细节不能靠降低亮度恢复。

决策顺序由画面问题决定：
- 校正：判断黑白点与中性物是否偏色，区分拍摄光线意图与技术偏差。全局提亮可能抬灰黑位，不可自动解决背光脸部。
- 匹配：先确定参考镜头，再比较切点两侧的主体亮度与色相。避免室内外场景全部强行同一白平衡。
- 风格：选主色倾向与对比关系，保留商品和肤色的辨认。黑白/复古是有目的的风格，不默认添加暗角。
- 验证：亮部是否剪切、暗部是否压死、肤色与包装是否偏色、镜头间是否跳亮、压缩后是否色带。没有看到最终渲染就标为未检查，不用参数截图代替画面验收。

例：用户要略冷但保持商品色，可从小幅负 temperature 与近似原值的 saturation 开始；具体值需以实际素材比较为依据，不能把例子变成固定配方。修改原有风格时读取当前参数而非清空后重做。

## 官方学习来源（2026-09-27 核验）
- [Blackmagic Design 官方培训](https://www.blackmagicdesign.com/products/davinciresolve/training)：Color、Color Management 教程及 Colorist Guide，学习色彩管理、镜头匹配与调色工作顺序；教程涉及的高级能力不等于本应用已经支持。
- [DaVinci Resolve Color 功能说明](https://www.blackmagicdesign.com/products/davinciresolve/color)：了解区域调整与专业监看能力边界。
- [FFmpeg 官方滤镜文档](https://ffmpeg.org/ffmpeg-filters.html)：核对 eq、colorbalance 的真实参数意义。

这些是学习与核验入口，本技能为应用操作规范，不复制整本课程，也不承诺读完即达到大师水准。
