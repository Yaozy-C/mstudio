import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { Plus, Trash, ArrowClockwise } from "@phosphor-icons/react";
import { bridge, native } from "../bridge";
import "../styles/project-memory.css";
type Entry = {
  id: string;
  title: string;
  content: string;
  source: string;
  turnId: string | null;
  updated: number;
};
type Memory = {
  revision: number;
  enabled: boolean;
  autoUpdate: boolean;
  entries: Entry[];
};
export function ProjectMemory({
  projectId,
  name,
}: {
  projectId: string;
  name: string;
}) {
  const [memory, setMemory] = useState<Memory | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [dirty, setDirty] = useState(false);
  async function load() {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      setMemory(await bridge<Memory>("project_memory", { projectId }));
      setDirty(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    let active = true;
    setMemory(null);
    if (!native) {
      setMemory({ revision: 0, enabled: true, autoUpdate: true, entries: [] });
      return;
    }
    void bridge<Memory>("project_memory", { projectId })
      .then((v) => {
        if (active) setMemory(v);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [projectId]);
  function change(value: Memory) {
    setMemory(value);
    setDirty(true);
    setNotice("");
  }
  function edit(id: string, value: Partial<Entry>) {
    if (memory)
      change({
        ...memory,
        entries: memory.entries.map((e) =>
          e.id === id ? { ...e, ...value } : e,
        ),
      });
  }
  async function save() {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      setMemory(
        await bridge<Memory>("save_project_memory", { projectId, memory }),
      );
      setDirty(false);
      setNotice("项目记忆已保存");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="project-memory">
      <div className="hub-intro">
        <div className="eyebrow">{name} · 项目记忆</div>
        <h2>让后续创作记得已确定的事。</h2>
        <p>
          保存目标、约束与关键决定。同一项目的 Agent
          共享，切换模型或清空聊天后保留。
        </p>
      </div>
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
      {!memory ? (
        <p>正在读取项目记忆…</p>
      ) : (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <fieldset disabled={busy || !native}>
            <label className="memory-option">
              <span>
                <strong>使用项目记忆</strong>
                <small>
                  允许 Agent 读取已有记忆。关闭后保留内容，暂停读取和自动整理。
                </small>
              </span>
              <input
                type="checkbox"
                checked={memory.enabled}
                onChange={(e) =>
                  change({ ...memory, enabled: e.target.checked })
                }
              />
            </label>
            <label className="memory-option">
              <span>
                <strong>自动整理</strong>
                <small>
                  允许有记忆写入权限的 Agent 保存新增或变化的长期偏好与约束，
                  保留原话来源；不重复记录脚本、镜头或时间线。
                </small>
              </span>
              <input
                type="checkbox"
                disabled={!memory.enabled}
                checked={memory.autoUpdate}
                onChange={(e) =>
                  change({ ...memory, autoUpdate: e.target.checked })
                }
              />
            </label>
            <div className="hub-section-head">
              <h3>
                已记住的事 <small>{memory.entries.length} / 40</small>
              </h3>
              <button
                type="button"
                disabled={memory.entries.length >= 40}
                onClick={() =>
                  change({
                    ...memory,
                    entries: [
                      ...memory.entries,
                      {
                        id: crypto.randomUUID(),
                        title: "",
                        content: "",
                        source: "用户添加",
                        turnId: null,
                        updated: 0,
                      },
                    ],
                  })
                }
              >
                <Plus />
                添加记忆
              </button>
            </div>
            {!memory.entries.length && (
              <div className="memory-empty">
                <strong>还没有项目记忆</strong>
                <p>
                  可以告诉 Agent「记住，这个项目……」，也可以在这里手动添加。
                </p>
              </div>
            )}
            {memory.entries.map((entry) => (
              <article className="memory-entry" key={entry.id}>
                <div>
                  <input
                    aria-label="记忆主题"
                    placeholder="主题，例如：画面风格"
                    required
                    maxLength={60}
                    value={entry.title}
                    onChange={(e) => edit(entry.id, { title: e.target.value })}
                  />
                  <button
                    type="button"
                    aria-label={`删除记忆 ${entry.title || "新记忆"}`}
                    onClick={() =>
                      change({
                        ...memory,
                        entries: memory.entries.filter(
                          (e) => e.id !== entry.id,
                        ),
                      })
                    }
                  >
                    <Trash />
                  </button>
                </div>
                <textarea
                  aria-label="记忆内容"
                  placeholder="只保留之后仍然有用的结论…"
                  required
                  rows={3}
                  maxLength={1200}
                  value={entry.content}
                  onChange={(e) => edit(entry.id, { content: e.target.value })}
                />
                <small>
                  {entry.updated
                    ? new Date(entry.updated * 1000).toLocaleString()
                    : "尚未保存"}{" "}
                  · {entry.turnId ? "Agent 整理" : "手动编辑"}
                </small>
                <details>
                  <summary>查看来源</summary>
                  <p>{entry.source}</p>
                </details>
              </article>
            ))}
            <footer className="memory-footer">
              <span role="status">
                {notice || (dirty ? "有未保存的修改" : "仅用于当前项目")}
              </span>
              <button type="button" onClick={() => void load()}>
                <ArrowClockwise />
                {dirty ? "放弃修改并刷新" : "刷新"}
              </button>
              <button className="primary" disabled={!dirty}>
                保存记忆
              </button>
            </footer>
          </fieldset>
        </form>
      )}
    </section>
  );
}
