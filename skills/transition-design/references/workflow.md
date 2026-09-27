# 接缝设计

## 判断依据
- 硬切：同一动作接续、信息明确、节奏紧凑时优先比较硬切。
- 叠化 fade：时间流逝或情绪过渡可以使用；两张复杂画面叠在一起可能形成重影，别让关键动作被混合遮蔽。
- 黑场 fadeblack：段落结束/开始；白场 fadewhite 需要光线或叙事动机，避免连续高亮闪烁。
- 推移 slideleft/right、smoothleft/right：运动方向和新主体出现位置应协调；不把滤镜推移叫作真实摄影机连续运动。
- 擦除 wipeleft/right、circleopen/close：适用于明确风格化分段；真实前景遮挡转场需要实际遮挡与蒙版，当前工具不提供自动物体分割。
- dissolve 为颗粒溶解，区别于 fade 的平滑叠化；有意的图形风格才使用。

时长根据运动与辨认需要决定。短视频可先比较数帧到约半秒的不同长度，不把它作为统一规定。长叠化适用于慢节奏和时间流逝；不能掩盖错误动作、缺失事件、跳轴或商品身份变化。

## 本应用的执行语义

自定义使用 `set_transition(kind="custom",design={...})`，而不是把设计理由硬塞进类型名称。design 可组合 mask（uniform整体混合、linear方向蒙版、radial径向蒙版）、angle、center:[x,y]、feather、curve、outgoingZoom/incomingZoom、outgoingOffset/incomingOffset。位置和偏移为画幅比例，缩放1–4；前镜从原构图运动至结束状态，后镜从开始状态回到原构图。超出画幅的采样会延展边缘像素，过度偏移可能拉出条带；缩放采样不等于光学运动模糊，以实际预览判断是否合适。

curve 是2–8个 [时间比例,完成比例] 点，必须从[0,0]到[1,1]，时间严格递增，进度不倒退。通过点位安排加速、停留和减速，结合主体位置选择推进中心；默认线性只是起点，不是所有素材的推荐配方。按动作和构图确定参数，不把自定义功能变成另一组固定套路。

转场位于切点两侧，各占一半时长。当前支持底层画面轨上的相邻、全幅、不透明片段；叠加轨保持原顺序。时长 0.05–3 秒，不能超过两侧任一片段时长。接缝不能有第三个重叠片段。已移动或删除的邻接关系不会继续渲染旧接缝。

优先读取源素材切点外的余量；到达源头尾时延展边缘帧，可能造成短暂停顿。快速动作建议保留足够源余量或选择更短转场/硬切，不能声称延展是光流补帧。应用不偷偷挪动镜头、缩短成片或改字幕。转场只处理画面，音轨保持原剪辑；J/L-cut 与音频交叉淡化需单独按用户范围安排，不能把视觉叠化视为音频已平滑。

修改后检查接缝前、中、后：余量是否出现不需要内容，主体是否双影，方向是否跳变，字幕是否保持可读，声音是否断裂，转场后是否出现回跳。参数已保存不等于已观看最终效果。

## 官方学习来源（2026-09-27 核验）
- [CapCut 转场入口与相邻片段要求](https://www.capcut.com/help/transitions-in-capcut)：接缝添加、调整时长、预览的交互依据。
- [CapCut 叠化教程](https://www.capcut.com/resource/dissolve-transition-in-video)：学习叠化、时间/场景转换的表达用途。
- [Blackmagic Design 编辑培训](https://www.blackmagicdesign.com/products/davinciresolve/training)：Editor Guide 与剪辑训练，学习叙事连续性和声音衔接。
- [FFmpeg xfade 文档](https://ffmpeg.org/ffmpeg-filters.html#xfade)：具体滤镜及输入帧率、尺寸、时间基一致要求。高级视觉设计需结合素材，不能由滤镜列表保证。
- [FFmpeg xfade 源码](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/vf_xfade.c)：自定义像素表达式与进度方向的实现依据；应用把结构化方案编译为表达式，agent 不提交原始滤镜代码。
- [FFmpeg perspective 源码](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/vf_perspective.c)：逐帧几何变换与双线性采样，避免在每个像素的混合表达式里重复计算运动。
- [Shotcut 编辑操作](https://www.shotcut.org/howtos/keyboard-shortcuts/)：先通过滑移、移动和顺移修正剪辑，再考虑转场；效果不能替代素材选择。
