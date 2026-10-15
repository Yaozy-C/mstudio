import { TrackHeading } from "../src/timeline/TrackHeading";
import { CaptionTrackHeadings } from "../src/timeline/CaptionTrackHeading";
import { newProject } from "../src/model";
import "../src/styles/timeline.css";
import "../src/styles/editing.css";
import "../src/styles/timeline-states.css";
import { useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { Plus, Eye, Trash, Stack } from "@phosphor-icons/react";
import { ActionButton } from "../src/ui/ActionButton";
import { AsyncButton } from "../src/ui/AsyncState";
import { IconButton } from "../src/ui/IconButton";
import { ObjectActions } from "../src/ui/ObjectActions";
import { ObjectMenu } from "../src/ui/ObjectMenu";
import { StudioSelect } from "../src/ui/StudioSelect";
import "@radix-ui/themes/styles.css";
import "../src/styles/color-tokens.css";
import "../src/styles/base.css";
import "../src/styles/production-desk.css";
import "../src/styles/frame-theme.css";
import "../src/styles/motion.css";
import "../src/styles/visual-system.css";

function Fixture() {
  const [project, change] = useState(() => newProject("轨道图标验收"));
  const [selected, setSelected] = useState(false);
  const [value, setValue] = useState("one");
  const [result, setResult] = useState("—");
  const dialog = useRef<HTMLDialogElement>(null);
  const actions = [
    { icon: Eye, label: "打开 Open", run: () => setResult("opened") },
    {
      icon: Plus,
      label: "不可用 Disabled",
      disabled: true,
      run: () => setResult("ERROR"),
    },
    {
      icon: Trash,
      label: "移除 Remove",
      danger: true,
      run: () => setResult("removed"),
    },
  ];
  return (
    <Theme appearance="light" accentColor="tomato">
      <main style={{ padding: 40, maxWidth: 920, display: "grid", gap: 28 }}>
        <h1>组件状态 · Component states</h1>
        <section style={{ display: "flex", gap: 16, alignItems: "center" }}>
          <ActionButton icon={Plus}>添加 Add</ActionButton>
          <ActionButton
            icon={Plus}
            aria-pressed={selected}
            onClick={() => setSelected(!selected)}
          >
            选中 Selected
          </ActionButton>
          <ActionButton icon={Plus} disabled>
            不可用 Disabled
          </ActionButton>
          <ActionButton icon={Plus} busy busyLabel="处理中…">
            添加 Add
          </ActionButton>
          <IconButton icon={Plus} label="图标操作 Icon action" />
        </section>
        <section style={{ display: "flex", gap: 16 }}>
          <button>普通按钮</button>
          <button className="primary">主要操作</button>
          <button className="primary danger">危险操作</button>
          <AsyncButton className="primary" busy busyLabel="保存中…">
            保存
          </AsyncButton>
        </section>
        <section style={{ display: "flex", gap: 20, alignItems: "center" }}>
          <ObjectActions actions={actions} />
          <ObjectMenu actions={actions}>
            <button>右键菜单 Context menu</button>
          </ObjectMenu>
          <div style={{ width: 240 }}>
            <StudioSelect
              label="选择项目"
              value={value}
              onValueChange={setValue}
              options={[
                { value: "one", label: "选项一 Option one" },
                { value: "two", label: "选项二 Option two" },
                { value: "three", label: "不可用", disabled: true },
              ]}
            />
          </div>
          <button onClick={() => dialog.current?.showModal()}>原生弹窗</button>
        </section>
        <section className="settings-page" style={{ display: "flex", gap: 16 }}>
          <input aria-label="正常输入" placeholder="正常输入" />
          <input
            aria-label="错误输入"
            aria-invalid="true"
            defaultValue="错误状态"
          />
          <input aria-label="不可用输入" disabled placeholder="不可用" />
        </section>
        <section className="track-labels">
          <div title="轨道 · 上层覆盖下层">
            <Stack aria-hidden="true" />
          </div>
          {project.tracks.map((track) => (
            <TrackHeading
              key={track.id}
              track={track}
              project={project}
              onChange={change}
              onRemove={() => {}}
            />
          ))}
          <CaptionTrackHeadings project={project} change={change} />
        </section>
        <output aria-label="操作结果">{result}</output>
        <dialog ref={dialog}>
          <h2>原生弹窗中的菜单</h2>
          <ObjectActions actions={actions} />
          <StudioSelect
            label="弹窗内选择"
            value={value}
            onValueChange={setValue}
            options={[
              { value: "one", label: "选项一 Option one" },
              { value: "two", label: "选项二 Option two" },
            ]}
          />
          <ObjectMenu actions={actions}>
            <button>弹窗内右键菜单</button>
          </ObjectMenu>
          <button onClick={() => dialog.current?.close()}>关闭弹窗</button>
        </dialog>
      </main>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
