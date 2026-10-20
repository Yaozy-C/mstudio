export type BrowseWork = {
  id: string;
  title: string;
  description: string;
  category: string;
  duration: number;
  poster: string;
  video: string;
  source: string;
  credit: string;
  licenseSource: string;
};

// External films, not mstudio output. Full films stream from Blender's own site.
export const browseWorks: BrowseWork[] = [
  {
    id: "coffee-run",
    credit: "(CC) Blender Foundation | studio.blender.org",
    title: "Coffee Run",
    description: "跟随一杯咖啡，穿行于一段关系的甜蜜与失落。",
    category: "风格化 · 叙事",
    duration: 184,
    poster: "./browse/coffee-run.webp",
    video:
      "https://video.blender.org/object-storage/web_videos/ff8fe61b-026f-4f07-b66b-2a790d6f6ab1-720.mp4",
    source: "https://studio.blender.org/projects/coffee-run/",
    licenseSource:
      "https://studio.blender.org/projects/coffee-run/pages/licensing/",
  },
  {
    id: "charge",
    credit: "© Blender Foundation | studio.blender.org",
    title: "Charge",
    description: "一座电池工厂，一场追逐，在紧张的节奏中揭晓行动的理由。",
    category: "科幻 · 动作",
    duration: 263,
    poster: "./browse/charge.jpg",
    video:
      "https://video.blender.org/object-storage/web_videos/70528f27-fd51-479f-8146-abb5a7a81fff-720.mp4",
    source: "https://studio.blender.org/projects/charge/",
    licenseSource: "https://studio.blender.org/terms-and-conditions/",
  },
  {
    id: "spring",
    credit: "© Blender Foundation | cloud.blender.org/spring",
    title: "Spring",
    description: "牧羊女孩与小狗唤醒山林，用光影、尺度与声音讲述季节的轮回。",
    category: "奇幻 · 氛围",
    duration: 464,
    poster: "./browse/spring.webp",
    video:
      "https://video.blender.org/object-storage/web_videos/3d95fb3d-c866-42c8-9db1-fe82f48ccb95-720.mp4",
    source: "https://studio.blender.org/projects/spring/",
    licenseSource: "https://studio.blender.org/projects/spring/pages/about/",
  },
];
