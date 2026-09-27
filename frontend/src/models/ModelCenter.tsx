import { t, useLanguage } from "../i18n";
import { useState } from "react";
import {
  ChatText,
  Image,
  FilmStrip,
  PlugsConnected,
} from "@phosphor-icons/react";
import { TextModels } from "./TextModels";
import { MediaModels } from "./MediaModels";
import { ServiceConnections } from "./ServiceConnections";
import { useMediaModels, type MediaKind } from "./mediaRegistry";
import "../styles/model-center.css";
import "../styles/model-hub.css";
import "../styles/model-cards.css";
import "../styles/agent-center.css";
const tabs = [
  { id: "text", name: "对话", icon: ChatText },
  { id: "image", name: "图像", icon: Image },
  { id: "video", name: "视频", icon: FilmStrip },
  { id: "providers", name: "服务连接", icon: PlugsConnected },
] as const;
export function ModelCenter() {
  useLanguage();
  const [tab, setTab] = useState<"text" | MediaKind | "providers">("text");
  const media = useMediaModels();
  return (
    <div className="model-hub">
      <nav className="hub-categories" aria-label={t("模型能力分类")}>
        {tabs.map(({ id, name, icon: Icon }) => (
          <button key={id} aria-pressed={tab === id} onClick={() => setTab(id)}>
            <Icon />
            {t(name)}
          </button>
        ))}
      </nav>
      {tab === "text" ? (
        <TextModels />
      ) : tab === "providers" ? (
        <section>
          <ServiceConnections />
          {media.models.some((m) => m.kind === "audio") && (
            <button
              onClick={() => {
                setTab("audio");
              }}
            >
              {t("管理音频连接")}
            </button>
          )}
        </section>
      ) : (
        <MediaModels key={tab} kind={tab} hub={media} />
      )}
    </div>
  );
}
