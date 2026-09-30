export type EffectPreset = {
  id: string;
  name: string;
  category: string;
  description: string;
  prompt: string;
  adaptation: string;
  source: { name: string; url: string };
  previewVideo?: string;
};

// Original reusable prompts informed by the linked public effect descriptions.
// They are not vendor presets or validated model-specific instructions.
export const effects: EffectPreset[] = [
  {
    id: "particles",
    previewVideo: `${import.meta.env.BASE_URL}effects/particles.mp4`,
    name: "粒子消散",
    category: "主体变化",
    description: "主体从边缘逐渐化为粒子，向画面一侧飘散。",
    prompt:
      "[主体]起初完整可辨。细小粒子从[起始部位]逐渐剥离，消散边界沿[方向]连续推进；粒子脱离的位置同步失去实体，形成[粒子材质与颜色]的流动轨迹。粒子受同一股气流带动，先聚集成形，再逐渐散开。以[节奏]完成[局部或整体]消散，结束时留下[结尾状态]。未受影响的区域、场景光照与空间关系保持连续，镜头运动与原镜头意图一致。",
    adaptation:
      "Agent 需确定消散对象、范围、方向与结尾。商品仍需可辨时采用局部消散；若全部消失，不再同时要求完整轮廓或文字始终保留。粒子颜色取自当前场景，不固定为金色。",
    source: {
      name: "Higgsfield · Disintegration",
      url: "https://higgsfield.ai/motion/4e981984-1cdc-4b96-a2b1-1a7c1ecb822d",
    },
  },
  {
    id: "melt",
    previewVideo: `${import.meta.env.BASE_URL}effects/melt.mp4`,
    name: "液态融化",
    category: "主体变化",
    description: "主体逐渐软化，流动并汇聚成液体。",
    prompt:
      "[主体]从当前状态开始，[变化范围]的表面逐渐软化为[液体材质]，高光随表面形变连续滑动。融化从[起始部位]沿重力方向推进，形成黏稠的液流与滴落；原有结构逐渐塌缩，在[承接位置]汇聚成液池，并产生轻微波纹。以[节奏]完成转变，最后停留在[结尾状态]。周围环境、光线方向和接触阴影保持连贯，镜头服务于融化过程。",
    adaptation:
      "Agent 区分保留原材质的融化与先金属化再融化。确认承接位置和允许变形的部位，结合原片运动选择机位；完整融化与保持主体形状不变不能同时要求。",
    source: {
      name: "Higgsfield · Turning Metal × Melting",
      url: "https://higgsfield.ai/motion/017ae2b7-bcff-42ef-863e-6e198f96c3ec",
    },
  },
  {
    id: "bloom",
    previewVideo: `${import.meta.env.BASE_URL}effects/bloom.mp4`,
    name: "花朵绽放",
    category: "主体变化",
    description: "花朵沿主体表面生长，逐渐绽放。",
    prompt:
      "细小芽点从[主体或区域]的[生长起点]出现，枝叶沿表面结构向[方向]延伸，花蕾依次形成并展开花瓣，生长为[花种与配色]。绽放按[疏密与节奏]逐渐推进，最终覆盖[指定范围]，保留[需要露出的区域]。花叶与主体产生自然遮挡、接触和投影，生长过程清晰连续。结尾停留在[最终构图]，镜头运动与原场景风格协调。",
    adaptation:
      "Agent 根据主体确定花种、覆盖率和留白。人像避免遮挡需保留的五官；商品保留必要识别区域。将生长与开花写成连续动作，避免仅把花束静态放在主体旁边。",
    source: {
      name: "Higgsfield · Garden Bloom",
      url: "https://higgsfield.ai/motion/b25600ef-238e-448a-bb07-1ff74fd0207f",
    },
  },
  {
    id: "freeze",
    previewVideo: `${import.meta.env.BASE_URL}effects/freeze.mp4`,
    name: "冰霜冻结",
    category: "材质变化",
    description: "霜纹沿表面蔓延，主体逐渐覆盖晶莹冰层。",
    prompt:
      "[主体]保持当前构图，细密霜纹从[起始位置]沿表面纹理向[方向]蔓延，逐渐结成[透明度与厚度]的冰层。冰晶依次生长，边缘出现冷凝雾气，反射与折射随冰层增厚连续变化。以[节奏]覆盖[指定范围]，结束时停留在[结尾状态]。冰层与主体接触自然，未冻结区域维持原材质，环境光线与镜头运动保持连续。",
    adaptation:
      "适合冷感、冰饮、冬季和材质变化画面。先确认是表面结霜还是整体冰雕化，以及是否冻结动作；商品展示保留标签与关键轮廓。冷雾和冰晶只作视觉表现，不据此宣称真实制冷或保温性能。",
    source: {
      name: "Higgsfield · Freezing",
      url: "https://higgsfield.ai/motion/777f1604-afee-406d-a711-bf1e0ea23c86",
    },
  },
  {
    id: "glow",
    previewVideo: `${import.meta.env.BASE_URL}effects/glow.mp4`,
    name: "发光描边",
    category: "光效与科技",
    description: "发光线条勾勒主体边缘，随动作留下短暂光迹。",
    prompt:
      "[主体]沿[动作路径]自然运动，[描边颜色]的柔和光线沿[指定轮廓]逐渐亮起，准确贴合表面边缘。运动处留下[长度]的短暂光迹，光迹沿实际运动方向延伸，随后平滑衰减。光线对附近表面产生轻微染色，以[强度与节奏]完成变化，结尾[保留描边或渐暗]。主体纹理清晰，曝光稳定。",
    adaptation:
      "适合穿搭、舞蹈、运动和轮廓强调。确认只描边还是同时增加拖尾；静止主体优先用描边，不凭空生成运动轨迹。减少过曝，保留面部、衣物和产品细节；示例以描边表现为主。",
    source: {
      name: "Higgsfield · Glow Trace",
      url: "https://higgsfield.ai/motion/519e724e-760f-4703-b6cd-d38223f27e53",
    },
  },
  {
    id: "wireframe",
    previewVideo: `${import.meta.env.BASE_URL}effects/wireframe.mp4`,
    name: "线框扫描",
    category: "光效与科技",
    description: "扫描沿主体推进，将实物表面逐步转为发光网格。",
    prompt:
      "一道[颜色]扫描光从[起始位置]沿[方向]扫过[主体]，扫描边界经过的位置逐渐显露贴合三维形体的细密线框网格。网格沿主体的曲面与结构连续分布，保持透视、比例与空间位置一致。以[速度]完成[扫描范围]，最终停留在[完整线框、局部剖示或恢复实物]状态。背景按[保留或渐暗]处理，镜头以[运动方式]展示形体。",
    adaptation:
      "适合鞋服、数码、工业外观和科技展示。明确是表面线框还是结构剖视，未提供内部结构资料时只表现外表网格，不虚构真实工程结构。网格变形跟随主体，不让多余肢体或零件凭空出现。",
    source: {
      name: "Higgsfield · Wireframe",
      url: "https://higgsfield.ai/motion/ea67acab-a7bf-4fff-b098-a0f1f1a6796c",
    },
  },
  {
    id: "orbit",
    previewVideo: `${import.meta.env.BASE_URL}effects/orbit.mp4`,
    name: "物体环绕",
    category: "空间运动",
    description: "指定物体悬浮在主体周围，沿空间轨迹缓慢环绕。",
    prompt:
      "[数量与种类]的[环绕物]从[出现位置]平缓进入画面，悬浮在[主体]周围，沿[轨道形状与方向]连续运动。前景物体从主体前方经过，后景物体被主体自然遮挡，大小、景深与投影符合距离变化。主体保持[动作与姿态]，环绕物保持各自形状，以[速度]完成运动，最后停留在[陈列或散开状态]。",
    adaptation:
      "适合配件、食材、产品组合和超现实陈列。确认环绕物的种类、数量、大小与出入画方式，不照搬示例中的苹果。保留主体识别区域，避免物体穿透身体或相互融合。悬浮环绕不等于产品拆解。",
    source: {
      name: "Higgsfield · Objects Around",
      url: "https://higgsfield.ai/motion/0275bb9f-7e50-4d45-84e4-3cb9dcbdd126",
    },
  },
  {
    id: "portal",
    previewVideo: `${import.meta.env.BASE_URL}effects/portal.mp4`,
    name: "传送门",
    category: "空间运动",
    description: "发光入口在场景中展开，主体靠近并穿过门内空间。",
    prompt:
      "[场景位置]出现一道[形状、颜色与材质]的光环，光环逐步扩展成与环境透视一致的入口，门内显现[目标空间]。[主体]以[动作]靠近并穿过入口，身体与门沿产生连续遮挡，入口光线自然照亮附近表面。穿越完成后镜头[停留原场景或跟随进入目标场景]，入口以[结束方式]收束。",
    adaptation:
      "适合奇幻叙事、异空间亮相和场景切换。确认门的位置、目标场景、主体是否穿过以及镜头是否跟随；需要准确目的地时提供目标画面。区分卡通示例风格与当前项目风格，避免人物瞬移、重复或在门沿融化。",
    source: {
      name: "Higgsfield · Portal",
      url: "https://higgsfield.ai/motion/2391dbfb-edc3-4982-b5b0-bf3746a25359",
    },
  },
  {
    id: "splash",
    previewVideo: `${import.meta.env.BASE_URL}effects/splash.mp4`,
    name: "水浪转场",
    category: "场景转场",
    description: "水浪冲入并遮满镜头，退去后露出下一场景。",
    prompt:
      "从[起始画面]开始，一股[方向与形态]的水浪迅速进入前景，水花和水幕逐渐覆盖整个镜头。在画面被水幕完全遮挡时完成场景衔接，水流沿同一运动方向退去，清晰露出[目标画面]。两侧镜头的[主体位置、动作或镜头方向]相互衔接，水滴反光与折射符合各自场景光照，以[节奏]结束在稳定的目标构图。",
    adaptation:
      "适合夏日、饮品、户外及强动感场景切换。必须确定起始与目标画面；需要保留两段原片时优先使用可支持双端画面或视频编辑的流程。水幕要完成遮挡再揭示，不把主体直接融成水；示例中的火焰不属于必需效果。",
    source: {
      name: "Higgsfield · Splash Transition",
      url: "https://higgsfield.ai/motion/105c744b-05bf-44ae-ba34-5a7144f4ec46",
    },
  },
  {
    id: "smoke",
    previewVideo: `${import.meta.env.BASE_URL}effects/smoke.mp4`,
    name: "烟雾转场",
    category: "场景转场",
    description: "烟雾卷入并遮住画面，散开后显现新的场景。",
    prompt:
      "从[起始画面]开始，[颜色与浓度]的烟雾由[方向]缓缓卷入，烟层前后交叠，逐步填满画面。在完全遮挡阶段衔接到[目标画面]，烟雾沿连续的气流方向散开，依次显露目标场景的前景、主体和背景。烟雾的受光、透明度与两端环境协调，整体以[速度与情绪]完成，结尾恢复清晰稳定的构图。",
    adaptation:
      "适合氛围片、舞台、换装或场景揭示。先确定两端画面、烟色和遮挡时长。连续人物需对齐身份、构图与动作；不要默认人物随场景变化而换脸。烟雾用于视觉遮挡，不自动给画面添加火源或爆炸。",
    source: {
      name: "Higgsfield · Smoke Transition",
      url: "https://higgsfield.ai/motion/efb77740-741f-419d-ae8d-cf487e01a6e9",
    },
  },
  {
    id: "earth",
    previewVideo: `${import.meta.env.BASE_URL}effects/earth.mp4`,
    name: "地球拉远",
    category: "空间运动",
    description: "镜头从主体快速拉远，跨越地景与云层直至地球全景。",
    prompt:
      "镜头从[主体与起始地点]出发，沿连续的后退上升轨迹平滑加速拉远，依次显露周围环境、城市或地形、大范围地表、云层与地球弧面，最终在[太空视角与构图]减速停稳。各尺度间保持方向与空间衔接，主体随距离增大自然缩小，地面细节逐步简化，日照方向与云层透视协调，整个过程在[时长]内完成。",
    adaptation:
      "适合旅行、地域开场、全球叙事及片尾。确认起点、拉远终点与时长；不要求进入太空后主体仍清晰可辨。需要真实地理连续性时提供地图或卫星参考，否则作为风格化示意，不宣称地形与位置精确。",
    source: {
      name: "Higgsfield · Earth Zoom Out",
      url: "https://higgsfield.ai/motion/70e490b9-26b7-4572-8d9c-2ac8dcc9adc0",
    },
  },
];

export const effectCover = (effect: EffectPreset) =>
  `${import.meta.env.BASE_URL}effects/${effect.id}.jpg`;
