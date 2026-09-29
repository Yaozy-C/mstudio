// Development-only fixture, deliberately outside the production entry point.
import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { newProject, type Project } from "../src/model";
import { ProductionCanvas } from "../src/production/ProductionCanvas";
import { productionItems } from "../src/production/items";
import type { ProductionController } from "../src/production/useProduction";
import "@radix-ui/themes/styles.css";
import "../src/styles/base.css";
import "../src/styles/dialogs.css";
import "../src/styles/preview.css";
import "../src/styles/object-menu.css";
function fixture(count: number) {
  const p = newProject("Canvas stress fixture");
  p.production = { viewport: { x: 20, y: 20, scale: 1 } };
  p.assets = Array.from({ length: count }, (_, i) => ({
    id: `asset-${i}`,
    kind: "video" as const,
    name: `Video ${i + 1}`,
    duration: 5,
    width: 480,
    height: 270,
    hasAudio: false,
    path: "",
    preview: `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="480" height="270"><rect width="480" height="270" fill="hsl(${(i * 23) % 360} 30% 35%)"/><text x="30" y="145" font-size="52" fill="white">Video ${i + 1}</text></svg>`)}`,
  }));
  p.nodes = p.assets.map((a, i) => ({
    id: `node-${i}`,
    kind: "asset",
    assetId: a.id,
    title: a.name,
    text: "",
    x: (i % 20) * 300,
    y: Math.floor(i / 20) * 380,
    width: 260,
    height: 330,
  }));
  return p;
}
function Stress() {
  const [project, setProject] = useState(() => fixture(500));
  const [selected, choose] = useState<string[]>([]);
  const [counts, setCounts] = useState({ cards: 0, images: 0, videos: 0 });
  useEffect(() => {
    const observe = () => {
      const next = {
        cards: document.querySelectorAll(".canvas-card").length,
        images: document.querySelectorAll(".canvas-card img").length,
        videos: document.querySelectorAll("video").length,
      };
      setCounts((old) =>
        JSON.stringify(old) === JSON.stringify(next) ? old : next,
      );
    };
    const observer = new MutationObserver(observe);
    observer.observe(document.getElementById("canvas-test")!, {
      childList: true,
      subtree: true,
    });
    observe();
    return () => observer.disconnect();
  }, []);
  const items = useMemo(
    () => productionItems(project),
    [project.nodes, project.assets, project.production?.positions],
  );
  const canvas = useMemo(
    () =>
      ({
        items,
        selected,
        choose,
        focus: { tick: 0 },
        referenced: new Set<string>(),
        select: (id: string, multiple: boolean) =>
          choose((old) => (multiple ? [...new Set([...old, id])] : [id])),
        reference: () => {},
        act: () => {},
        remove: () => {},
        move: (key: string, x: number, y: number) =>
          setProject((p) => ({
            ...p,
            production: {
              ...p.production,
              positions: { ...p.production?.positions, [key]: { x, y } },
            },
          })),
      }) as unknown as ProductionController,
    [items, selected],
  );
  return (
    <>
      <header style={{ padding: 16, display: "flex", gap: 24 }}>
        <label>
          测试视频数{" "}
          <select
            aria-label="测试视频数"
            value={project.assets.length}
            onChange={(e) => {
              setProject(fixture(Number(e.target.value)));
              choose([]);
            }}
          >
            {[100, 300, 500].map((count) => (
              <option key={count}>{count}</option>
            ))}
          </select>
        </label>
        <output>
          挂载卡片 {counts.cards} · 封面 {counts.images} · 播放器{" "}
          {counts.videos}
        </output>
        <span>合成封面，无真实视频或付费请求</span>
      </header>
      <div id="canvas-test" style={{ height: "calc(100vh - 65px)" }}>
        <ProductionCanvas
          project={project}
          change={(fn: (p: Project) => Project) => setProject(fn)}
          canvas={canvas}
          onAdd={() => {}}
        />
      </div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(
  <Theme>
    <Stress />
  </Theme>,
);
