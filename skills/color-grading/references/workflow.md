# 调色决策与工具边界

新调色使用 `update_clip.visual.grade`：曝光 ±3 EV；色温、色调、对比、饱和度、自然饱和度、高光、阴影、白/黑色阶与分区平衡 ±100；八色 HSL、总/R/G/B 曲线、阴影/中间调/高光色轮。只覆盖提交字段，未提交值保留；grade:null 只清除自定义调色。源图像不重绘，方案烘焙为 33³ LUT 并缓存，抽帧、原生预览、转场和导出使用同一方案。

HSL 按红橙黄绿青蓝紫洋红顺序，每行 [色相偏移,饱和度,明度]；curves 按总/R/G/B，每行三个 y 值对应 x=.25/.5/.75，范围0–1且单调递增；wheels 按阴影/中间调/高光，每行 [色相0–360,饱和度0–100,明度-100–100]。提交数组时整组覆盖，先读取当前值。曲线是固定点分段线性，不宣称自由贝塞尔曲线。

旧 brightness、contrast、saturation、temperature 与 effect 仍用于兼容已有工程，在自定义调色之后执行；新方案优先使用 grade，勿无意叠加两套对比/曝光调整。这里是 SDR 显示 RGB 创意调色，不是相机 Log/HDR 输入转换、RAW 显影、校准监看、跟踪蒙版或完整色彩管理。素材已经裁掉的高光细节不能靠降低曝光恢复。

决策顺序由画面问题决定：
- 校正：判断黑白点与中性物是否偏色，区分拍摄光线意图与技术偏差。全局提亮可能抬灰黑位，不可自动解决背光脸部。
- 匹配：先确定参考镜头，再比较切点两侧的主体亮度与色相。避免室内外场景全部强行同一白平衡。
- 风格：选主色倾向与对比关系，保留商品和肤色的辨认。黑白/复古是有目的的风格，不默认添加暗角。
- 验证：亮部是否剪切、暗部是否压死、肤色与包装是否偏色、镜头间是否跳亮、压缩后是否色带。没有看到最终渲染就标为未检查，不用参数截图代替画面验收。

例：用户要略冷但保持商品色，先比较中性物与商品色，再在 grade 中小幅调整 temperature；必要时用 HSL 修正受影响色段。HSL 按颜色选择，会影响画面里同色的其他物体，不是语义商品蒙版。具体值取决于抽帧结果，不能把例子变成固定配方。

## 官方学习来源（2026-09-27 核验）
- [Blackmagic Design 官方培训](https://www.blackmagicdesign.com/products/davinciresolve/training)：Color、Color Management 教程及 Colorist Guide，学习色彩管理、镜头匹配与调色工作顺序；教程涉及的高级能力不等于本应用已经支持。
- [DaVinci Resolve Color 功能说明](https://www.blackmagicdesign.com/products/davinciresolve/color)：了解区域调整与专业监看能力边界。
- [FFmpeg 官方滤镜文档](https://ffmpeg.org/ffmpeg-filters.html)：核对 eq、colorbalance 的真实参数意义。
- [Shotcut 明暗滤镜实现](https://github.com/mltframework/shotcut/blob/master/src/qml/filters/brightness/ui.qml)：参考将用户参数与底层滤镜绑定、保留可编辑状态的做法。本应用的 LUT 调色不照搬该滤镜公式。

这些是学习与核验入口，本技能为应用操作规范，不复制整本课程，也不承诺读完即达到大师水准。
