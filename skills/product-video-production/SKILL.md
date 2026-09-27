---
name: product-video-production
description: 在 Mstudio 中将已定方案转为媒体任务、有效素材选择与时间线剪辑，检查动作来源、连续性、节奏和声音。
---

# 视频制作、剪辑与检查

按本轮目标读取相关工程内容和用户要求；承接制作时参考 [制作交接](../product-storyboard/assets/production-brief-template.md)。沿用指定创意和模型，不把新委托当旧片续作，不以技术困难删核心事件。受众、风格、禁用项及授权均来自当前项目。

## 局部提示词修改

引用任务卡时，以 referencedTask.taskKey 为目标，用 inspect(section=generation, taskKey=该任务键) 读取当前任务；已有精确任务键时不翻查全项目任务列表。需要补充依据才按 nodeIds/fields 读取对应镜头、所属段落或素材；只有影响衔接时补读必要相邻镜头。

用 update_generation(taskKey,text) 保存该任务提示词，已提交任务保存为下次生成描述。任务提示词与分镜草稿分别维护，除非用户要求同步，否则不顺带覆盖 shot.prompt/framePrompt。未引用任务、明确修改镜头草稿时才用 update_node 保存对应字段。只改提示词不生成。

历史对话和记忆只作背景，旧待办不是本轮工作，也不能当成当前已核实状态。回复只说明目标任务、实际保存和影响本次修改的具体缺口，不附带其他镜头待办或修改邀约。修改文字不代表生成问题已经消除。

## 生成

按 [生成授权](../product-storyboard/SKILL.md#视频生成前的用户审核) 和用户选择的任务执行方式工作。只要求提示词则保存草稿；明确要求生成才用 request_generation，不另开外部 API 或自动购买服务。

1. 用 mstudio_models 查询可用模型，按指定模型读取相关规则。不把服务失败说成所有路线均不可用，也不访问或输出密钥。
   视频制作角色负责实际模式、参考角色、参数和最终提示词，统筹交接不代替这些判断。已有分镜出现机位、可见范围或动作时长矛盾时，带具体问题交统筹转分镜导演；保留用户目标，不机械照抄或擅改镜头。静态画格由画手负责，成片剪辑与声音由剪辑角色负责。
2. 按 [受控生成](references/generation.md) 区分商品事实、场景、动作状态和真正首尾帧。参考处于动作中段时不能仅靠“不是首帧”保证正确起点。
3. 图片读 [图片提示词](../creative-ad-director/references/image-prompt-writing.md)，视频读 [视频提示词](../creative-ad-director/references/video-prompt-writing.md)；选 H3 再读专用适配。编写镜头草稿时按角色权限保存 shot.framePrompt/shot.prompt；修改已有任务时只更新目标任务。提交 text 为最终模型正文，核对实际 references 用途/顺序与媒体模式。
   用户要求的比例、分辨率和时长落实到模型与工具支持的实际参数；文字描述不等于参数已设置。区分素材时长与成片时长，需要裁切时明确交给剪辑。能力不支持或工具无法表达时返回具体缺口与可行选项，不静默按默认值提交。普通参考不能靠一句“作为尾帧”变成尾帧控制。常规实现选择自行完成，不增加逐阶段确认。
4. 用 inspect section=generation 查询真实状态及结果 assetId，复用已有任务，不能重复提交。生成完成不等于视觉通过；从来源/初始支撑检查到路径、接触与去向。仅后半段正确不算整段通过。
5. 按 [动作与剪辑](references/motion-and-editing.md) 标记可用源区间、弃用原因和最小补做范围。同一问题两轮有依据的修复无进展时调整路线；已到用户次数/预算限制则停止额外生成并报告。

## 剪辑与速度

剪辑角色使用真实素材和已有时间线操作。trimIn/trimOut 为源区间，start 为成片时间，时长为 (trimOut-trimIn)/speed。先删无作用的等待和重复，保留理解因果的动作；素材缺失不能用快切隐藏。

编辑前用 `mstudio_read_image(assetId,clipId,time)` 查看动作开始、关键事件和结束，time 为片段内秒数，自动换算裁切和速度；省略 clipId 则按源时间取帧。每次编辑先 inspect 最新 revision，使用以下操作而非猜测源时间：
- `move_clip(id,start,trackId?)`：移动到成片秒数，对齐项目帧率；保持源区间与速度。
- `retime_clip(id,speed,ripple?)`：0.25–4 倍绝对速度，保持源区间和起点。ripple:true 仅顺移同轨原尾点及之后的片段，其他音轨/字幕保持原位；跨轨同步需显式安排，不能称为已自动同步。
- `slip_clip(id,sourceOffset)`：按源秒数同时偏移入/出点，保持成片位置与时长，适合换用动作阶段。
移动/变速默认拒绝同轨重叠，仅明确需要叠加时指定 allowOverlap:true；超出源余量直接失败，不截断、不补造画面。失败后重新读取状态和余量，不盲目重试。执行顺序先剪辑与变速，再处理接缝与调色复查。

这些语义参考 [Shotcut 编辑操作](https://www.shotcut.org/howtos/keyboard-shortcuts/) 中移动、顺移与滑移的区分，以及 [DaVinci Resolve 编辑培训](https://www.blackmagicdesign.com/products/davinciresolve/training) 的素材区间与时间线组织方法；它们不表示本应用具备教程的所有工具。

按 [节奏与速度反馈](../creative-ad-director/references/rhythm.md) 区分固定长度、上限、局部提速和明确整片倍速。绝对设速与在当前基础再提速不同。整片变速同步相关轨道的起点、速度、字幕及淡入淡出时间，源裁切区间不随成片时间同比缩放；核对工具范围，不能静默截断。

修改后 inspect 核实时间线，未提供的导出/播放/音频工具不能假称已执行。通过片段及无关配音、字幕尽量保留。切换到另一版本后不自动沿用旧版验收。

## 审片与交付

按 [成片审查](references/review-and-delivery.md) 对照实际媒体，按 [验收证据](references/evidence-contract.md) 保存观察。原始部件、关键因果、动作来源和去向、额外重复镜头均要检查。动态与声音需要实际观看和听取，抽帧和元数据不能证明。

交付真实素材或已保存时间线，准确区分已完成、待执行和未验收。已知核心动作错误先修，不能改名“低清试片限制”放行。不因本技能要求文件记录而假称能运行 shell；工程和交接消息承担现有能力内的记录。
