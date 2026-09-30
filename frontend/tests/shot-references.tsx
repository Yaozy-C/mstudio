import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { newProject } from "../src/model";
import { CanvasPreview } from "../src/production/CanvasPreview";
import { productionItems } from "../src/production/items";
import "@radix-ui/themes/styles.css";
import "../src/styles/base.css";
import "../src/styles/dialogs.css";
import "../src/styles/preview.css";
import "../src/styles/production-desk.css";
import "../src/styles/production-surfaces.css";
import "../src/styles/frame-theme.css";
const initial = newProject("Reference editor fixture");
initial.assets = ["商品外观", "人物服装"].map((name, i) => ({
  id: `asset-${i}`,
  name,
  kind: "image",
  path: `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="${i ? "#9b7" : "#579"}"/></svg>`)}`,
  duration: 0,
  width: 100,
  height: 100,
  hasAudio: false,
}));
initial.nodes = [
  {
    id: "shot",
    kind: "shot",
    title: "打开保温包",
    text: "展示内部空间",
    x: 0,
    y: 0,
    shot: { screenplayId: "script", order: 1, duration: 5, dialogue: "" },
    references: [{ assetId: "asset-0", purpose: "保持商品结构" }],
  },
];
function App() {
  const [project, setProject] = useState(initial);
  const [open, setOpen] = useState(true);
  return (
    <Theme>
      <button onClick={() => setOpen(true)}>打开镜头</button>
      <output>{JSON.stringify(project.nodes[0].references)}</output>
      {open && (
        <CanvasPreview
          item={productionItems(project)[0]}
          project={project}
          change={(fn) => setProject(fn(project))}
          close={() => setOpen(false)}
          onAdd={() => {}}
        />
      )}
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
