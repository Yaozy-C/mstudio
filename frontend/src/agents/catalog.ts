import builtins from "./defaults.json";
import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import { listen } from "@tauri-apps/api/event";
export type AgentProfile = {
  id: string;
  name: string;
  description: string;
  instructions: string;
  skillIds: string[];
  toolIds: string[];
  enabled: boolean;
  revision: number;
};
export const skills = [
  {
    id: "ad-script",
    name: "创意与声画脚本",
    description: "内容优先的创意、观看回报、声画脚本与本地研究",
    kind: "可编辑规则",
  },
  {
    id: "creative-ad-director",
    name: "创意导演",
    description: "观看主张、分镜设计、真人表演、摄影外观与节奏",
    kind: "可编辑规则",
  },
  {
    id: "image-production",
    name: "图片制作",
    description: "图片提示词、分镜画格、参考素材与画面检查",
    kind: "可编辑规则",
  },
  {
    id: "product-video-production",
    name: "视频制作",
    description: "视频提示词、输入用途、生成修复与成片判断",
    kind: "可编辑规则",
  },
  {
    id: "video-editing",
    name: "剪辑与后期",
    description: "选段、节奏、调色、转场与声音",
    kind: "可编辑规则",
  },
];
export const defaultAgents: AgentProfile[] = builtins;
export const agentsChanged = () =>
  window.dispatchEvent(new Event("agents-changed"));
export function useAgents() {
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    let sequence = 0;
    let unlisten: (() => void) | undefined;
    const refresh = () => {
      const request = ++sequence;
      void (
        native
          ? bridge<AgentProfile[]>("agent_catalog")
          : Promise.resolve(defaultAgents)
      )
        .then((v) => {
          if (active && request === sequence) {
            setAgents(v);
            setError("");
          }
        })
        .catch((e) => {
          if (active && request === sequence) setError(String(e));
        })
        .finally(() => {
          if (active && request === sequence) setLoading(false);
        });
    };
    refresh();
    window.addEventListener("agents-changed", refresh);
    window.addEventListener("focus", refresh);
    if (native)
      void listen("mstudio-agents-changed", refresh)
        .then((off) => {
          if (active) {
            unlisten = off;
            refresh(); // Cover saves between the initial read and event registration.
          } else off();
        })
        .catch((e) => {
          if (active) setError(String(e));
        });
    return () => {
      active = false;
      unlisten?.();
      window.removeEventListener("agents-changed", refresh);
      window.removeEventListener("focus", refresh);
    };
  }, []);
  async function save(profile: AgentProfile) {
    await bridge("save_agent", { profile });
    agentsChanged();
  }
  return { agents, loading, error, save };
}
