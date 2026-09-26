import { ErrorNotice } from "../errors/ErrorNotice";
import { inputSummary } from "../models/inputCapabilities";
import { useState } from "react";
import { DropdownMenu } from "@radix-ui/themes";
import { CaretDown, GearSix } from "@phosphor-icons/react";
import { ModelMark } from "../ui/Identity";
import { readyModel, selectedModel } from "../models/types";
import "../styles/conversation-model.css";
import { bridge } from "../bridge";
import { useModels, modelsChanged } from "../models/useModels";
export function ConversationModel({
  projectId,
  disabled,
  settings,
}: {
  projectId: string;
  disabled: boolean;
  settings: () => void;
}) {
  const { catalog } = useModels(projectId);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const current = selectedModel(catalog);
  const defaultModel = catalog.profiles.find((m) => m.id === catalog.defaultId);
  async function choose(id: string) {
    setBusy(true);
    setError("");
    try {
      await bridge("select_conversation_model", { projectId, id: id || null });
      modelsChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="conversation-model">
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <button
            type="button"
            className="conversation-model-trigger"
            aria-label={`对话模型：${current?.name || "选择模型"}`}
            title={current?.name || "选择对话模型"}
            disabled={disabled || busy}
          >
            {current && (
              <ModelMark
                identity={`${current.endpoint} ${current.model}`}
                size={16}
              />
            )}
            <span>{current?.name.split(" · ").at(-1) || "选择模型"}</span>
            <CaretDown size={12} aria-hidden="true" />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content
          className="conversation-model-menu"
          side="top"
          align="start"
          sideOffset={10}
        >
          <DropdownMenu.Label>对话模型</DropdownMenu.Label>
          <DropdownMenu.RadioGroup
            value={catalog.selectedId || ""}
            onValueChange={(id) => void choose(id)}
          >
            {defaultModel && (
              <DropdownMenu.RadioItem
                value=""
                disabled={!readyModel(defaultModel)}
              >
                <span className="model-menu-copy">
                  <span>使用默认模型</span>
                  <small>{defaultModel.name}</small>
                </span>
              </DropdownMenu.RadioItem>
            )}
            {catalog.profiles.map((m) => (
              <DropdownMenu.RadioItem
                value={m.id}
                key={m.id}
                disabled={!readyModel(m)}
              >
                <ModelMark identity={`${m.endpoint} ${m.model}`} size={18} />
                <span className="model-menu-copy">
                  <span>{m.name}</span>
                  <small>可读取：{inputSummary(m)}</small>
                  {!readyModel(m) && <small>待配置密钥</small>}
                </span>
              </DropdownMenu.RadioItem>
            ))}
          </DropdownMenu.RadioGroup>
          {!catalog.profiles.length && (
            <p className="model-menu-empty">尚未添加对话模型</p>
          )}
          <DropdownMenu.Separator />
          <DropdownMenu.Item onSelect={settings}>
            <GearSix size={18} aria-hidden="true" />
            管理模型
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      {error && <ErrorNotice error={error} />}
    </div>
  );
}
