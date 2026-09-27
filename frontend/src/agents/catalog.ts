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
    id: "color-grading",
    name: "专业调色",
    description: "色彩校正、镜头匹配与工具边界",
    kind: "内置规则",
  },
  {
    id: "transition-design",
    name: "转场设计",
    description: "动作匹配、节奏与接缝转场",
    kind: "内置规则",
  },
  {
    id: "ad-script",
    name: "创意与声画脚本",
    description: "广告事件、画面文字与声画脚本",
    kind: "内置规则",
  },
  {
    id: "storyboard-art",
    name: "分镜画格",
    description: "静态构图、图片提示词与修图",
    kind: "内置规则",
  },
  {
    id: "ad-team",
    name: "团队统筹",
    description: "任务交接、专业角色协作与局部修订",
    kind: "内置规则",
  },
  {
    id: "product-storyboard",
    name: "脚本与分镜",
    description: "故事结构、叙事节奏和镜头拆解",
    kind: "内置规则",
  },
  {
    id: "creative-ad-director",
    name: "创意导演",
    description: "创意方向、摄影与视觉语言",
    kind: "内置规则",
  },
  {
    id: "product-video-production",
    name: "视频制作",
    description: "生成描述、参考素材与制作检查",
    kind: "内置规则",
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
