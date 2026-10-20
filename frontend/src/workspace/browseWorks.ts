export type BrowseWork = {
  id: string;
  title: string;
  description: string;
  category: string;
  poster: string;
  embed: string;
  source: string;
  credit: string;
};

// AI concept ads for fictional products, published by Runway. Stream from the
// publisher's Mux player; media is not copied into the app or offered as assets.
const source = "https://runway.com/AnotherBigAdContest#winners";
export const browseWorks: BrowseWork[] = [
  {
    id: "mora-mora",
    title: "Mora Mora",
    credit: "Herinarivo Rakotomanana",
    category: "AI 短视频 · 音乐叙事",
    description: "JAUNT 音箱概念广告，把日常人物与音乐情绪连接起来。",
    poster:
      "https://image.mux.com/olm02IZGbsQcr02FwePKtJkycLzksPF6DdEPe2021pIKXA/thumbnail.webp",
    embed:
      "https://player.mux.com/olm02IZGbsQcr02FwePKtJkycLzksPF6DdEPe2021pIKXA",
    source,
  },
  {
    id: "vero-factory",
    title: "Vero Factory",
    credit: "Mathery",
    category: "视觉创意 · 商品世界",
    description: "Vero 汽水概念广告，用夸张空间和统一色彩构建商品的世界。",
    poster:
      "https://image.mux.com/dV4LLRV9A0100n6kRJYvwDcrn669n3OpUQ8Sp025mkbG3U/thumbnail.webp",
    embed:
      "https://player.mux.com/dV4LLRV9A0100n6kRJYvwDcrn669n3OpUQ8Sp025mkbG3U",
    source,
  },
  {
    id: "small-disasters",
    title: "Small Disasters",
    credit: "Albert Montero Vera",
    category: "商品广告 · 生活喜剧",
    description: "JACK 多功能工具概念广告，从生活中的小意外切入商品用途。",
    poster:
      "https://image.mux.com/3j00ilqt02qw02eovORkXO5FHV5FnGPc1Uc67uF00MPQSXo/thumbnail.webp",
    embed:
      "https://player.mux.com/3j00ilqt02qw02eovORkXO5FHV5FnGPc1Uc67uF00MPQSXo",
    source,
  },
  {
    id: "solace-vlad",
    title: "Solace x Vlad",
    credit: "László Gaál",
    category: "商品广告 · 反差创意",
    description: "Solacé 防晒概念广告，把吸血鬼放进阳光假日的场景。",
    poster:
      "https://image.mux.com/Qht5PFlbd025FWjRs38wyNGolsCvR02I2FXXDQyuRgXg4/thumbnail.webp",
    embed:
      "https://player.mux.com/Qht5PFlbd025FWjRs38wyNGolsCvR02I2FXXDQyuRgXg4",
    source,
  },
  {
    id: "not-norway",
    title: "This is not Norway",
    credit: "Ciro Kavouras",
    category: "AI 短视频 · 荒诞叙事",
    description: "以荒诞场景展开的 AI 概念广告，借鉴设定、反差与信息揭晓。",
    poster:
      "https://image.mux.com/todqVbVRatvWi34s5qYP8m3vY585c00kMUc02fjRefcT00/thumbnail.webp",
    embed:
      "https://player.mux.com/todqVbVRatvWi34s5qYP8m3vY585c00kMUc02fjRefcT00",
    source,
  },
  {
    id: "eau-de-paw",
    title: "CORN CHIPS — Eau de Paw",
    credit: "XAZINGA",
    category: "视觉创意 · 幽默广告",
    description: "虚构香氛概念广告，把宠物、气味与商品表达组合成一个玩笑。",
    poster:
      "https://image.mux.com/KAfII0000KRAkVCxNskkb7y1tOKvnFQi3a00AfPm78fDVE/thumbnail.webp",
    embed:
      "https://player.mux.com/KAfII0000KRAkVCxNskkb7y1tOKvnFQi3a00AfPm78fDVE",
    source,
  },
];
