# MiniMax H3：从分镜到声画提示

仅在选用 H3、准备交接生成提示或排查 H3 输出时读。官方依据见文末；下面格式是提示文本组织，不自动等同 API 参数。执行前核对实际服务的模型、模式、输入映射和提示重写设置，不把本地模型功能推定为所有托管接口可用。模型未知时不要静默改成 H3。

先读 [通用镜头提示词方法](video-prompt-writing.md)，再用本文件适配 H3。六字段是组织形式，不能替代导演设计；不要为填满字段重复全部部件和操作。本文只用于 H3，不将专用语法套给其他模型。

## 先选模式，再写正文

| 实际输入用途 | 提示组织 |
|---|---|
| 文生视频 T2VA | 三核心字段，无图片对齐前缀 |
| 首帧 I2VA | 首帧对齐说明＋三核心字段；图片必须实际作为首帧提交 |
| 首尾帧 FL2VA / 尾帧 L2VA | 按官方基础指南写对应帧对齐说明与有效结束时间，描述通向目标帧的变化 |
| 全参考 | 六段结构，定义商品、构图、动作、声音参考各自角色；不能把商品外观参考误当实际首帧 |

基础三字段：`integrated_multimodal_description`、`overall_soundscape`、`non_diegetic_music`。
全参考六字段依次为：`subject_definitions`、`summary`、`retention_analysis`、`detailed_description`、`overall_soundscape`、`non_diegetic_music`。正文按官方要求写英文；用户对白、歌词及画内文字保留原语言与内容。交用户说明可用中文。

全参考使用稳定的 `<Subject N>`、`<Picture N>`、`<Video N>`、`<Audio N>`，只定义实际使用且可追踪的输入。保留分析中的 `fully_preserved` 是要求而非完成证明。音频 `fully_copy`、`partially_copy` 与 `reference` 分别表达完整复用、部分复用和特征参考；不能把音色/节奏参考说成原信号复制。全参考完整语法和保留标记从官方指南核对，不凭印象拼造。

## 动作提示的修改原则

1. 有实际首帧输入时，以该帧为起点写“起始状态→动作启动→可见变化→结果”。首帧可以处在动作已开始、关键事件尚未发生的阶段；不能让文字要求再次执行图片中已经完成的动作。构图须留出运动空间。
2. 用方向、加减速、脱离支撑、接触和符合材质的响应表达力度；只加入对本镜必要的物理细节，不夸张商品性能，不堆砌身体机制。
3. 主体动作与相机分句。运镜按类型、必要幅度和速度写入镜内句子；相机快速跟拍不能代替主体加速。
4. H3 支持多镜头，不设“一生成只能一个动作”的硬规则。难动态关系先以最小必要片段验证，片段可包含用于表达关系的切镜；不通过塞入无关动作或不可能的时间表强迫提速。单片长度不改变已定广告总时长。
5. `[Shot 1]` 不加切镜时间戳；后续镜头以 `[Shot N] At MM:SS.mmm, ...` 标切点，时间递增且在片长内。镜内变化按发生次序和事件关系写清，精确时间要求仍须实际检查。
6. 不沿用“所有模型都禁止否定词”的规则，也不假定存在 `negative_prompt` 接口字段。优先写希望发生的具体状态；用户禁用项保留在制作约束与检查中，按实际端点支持传递。

## 声音与动作绑定

同步接触声写进主描述的对应事件：什么物体接触什么材质、在何时响起、如何衰减。`overall_soundscape` 用简短英文段落汇总环境声、物理声及非语言人声；对白和演唱在主描述中，不在此重复。全程明确要求静音时此字段才用 `N/A`。

没有画外配乐时，`non_diegetic_music: N/A`。需要配乐则写乐器、节奏、速度和动态，不仅写“震撼”。这些字段不保证静音或逐帧同步；也不能把 `N/A` 当 API 音频关闭开关。音乐、对白、音效是否分开后期制作沿用当前方案，不默认替用户取消。

## I2VA 示例：接触事件与声音一起写

以下仅示范提示粒度，不是默认商品或通用故事。使用前须确认实际首帧为杯子接触桌面之前的状态；具体运动与材质符合输入。

```text
For the target video, at 0.00 seconds into the target video, <Picture 1> (from [Shot 1]) is fully referenced.

integrated_multimodal_description: [Shot 1] Live-action, a close shot starts from the hand, ceramic cup and wooden table shown in <Picture 1>. The hand lowers the cup in one decisive movement. The cup base meets the wooden surface with one short, dry tap, then the fingers release the handle. The camera holds a static shot through the contact and release. The cup retains its shape and surface pattern.

overall_soundscape: A quiet indoor room tone underlies a single dry ceramic-on-wood tap at contact and faint sleeve rustle as the hand withdraws.

non_diegetic_music: N/A
```

不要把“果断放杯”升级成摔杯；声音不能先于实际接触，保持动作目的和物体完整性。声音字段结构来自官方，示例是未生成验证的自拟测试文本。

## 交接与故障定位

每个提交单元附：方案/镜头版本、实际模式与端点、素材用途及上传顺序、最终提示原文、片长与计划剪辑区间、动作/声音失败条件。若服务改写提示且能取得改写结果，记录实际结果；不能假定送入文本未变。

- 动作匀速漂浮：先查首帧与动作路径，再修改可见速度变化；调试时可暂简化相机，以定位问题，最终仍兑现已定运镜。
- 接触缺失或物体穿透：修参考/动作控制，不靠倍速和重音效掩盖。
- 同类问题有依据修正仍失败：按既有尝试上限改用可用动作参考、预演或其他已授权路线，不无限增长提示或盲抽。
- 动作成立但前后等待长：挑选有效区间或局部变速，再查连续性；不缩短整条广告来代替修剪。
- 画面合格但声音材质/同步错误：优先保留画面，修剪、替换或重做声音；混合音轨无法干净拆分时重建相关声音，不保证无损分离。

实际观看并听取输出后，分别记录商品部件、动作路径与力度、运镜、声音材质、接触同步的结果。无法播放/听取则标未验证，提示合规不能替代输出验收。

## 依据与边界

- [MiniMax 官方基础提示指南](https://huggingface.co/MiniMaxAI/MiniMax-H3/blob/main/docs/VIDEO_PROMPT_WRITING_GUIDE_base_en.md)
- [MiniMax 官方全参考指南](https://huggingface.co/MiniMaxAI/MiniMax-H3/blob/main/docs/VIDEO_PROMPT_WRITING_GUIDE_ref_en.md)
- [Reddit：H3 动作慢的尝试与局限](https://www.reddit.com/r/StableDiffusion/comments/1w3ks27/how_do_i_make_character_actions_in_minimax_h3/)

此参考记录提示组织方法，不代表本轮已核对最新官方文本。社区的“多加动作逼快”“堆速度同义词”等是互有矛盾的个人经验，不写成硬规则。实际服务的当前支持能力仍在调用前检查。
