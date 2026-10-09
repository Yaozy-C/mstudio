import {
  Graph,
  PencilSimple,
  FilmReel,
  Faders,
  Cube,
  type IconProps,
} from "@phosphor-icons/react";
import openai from "../assets/providers/openai.svg";
import minimax from "../assets/providers/minimax.svg";
import deepseek from "../assets/providers/deepseek.svg";
import google from "../assets/providers/google.svg";
import fal from "../assets/providers/fal.svg";
import bytedance from "../assets/providers/bytedance.svg";
export function AgentMark({
  id = "coordinator",
  ...props
}: IconProps & { id?: string }) {
  const Icon =
    {
      coordinator: Graph,
      concept: PencilSimple,
      writer: PencilSimple,
      director: FilmReel,
      image: Cube,
      production: FilmReel,
      editor: Faders,
    }[id] ?? Graph;
  return <Icon aria-hidden="true" weight="regular" {...props} />;
}
export function ModelMark({
  identity,
  size = 24,
}: {
  identity: string;
  size?: number;
}) {
  const key = identity.toLowerCase();
  const src = /google|gemini|banana|veo/.test(key)
    ? google
    : /minimax|hailuo/.test(key)
      ? minimax
      : /deepseek/.test(key)
        ? deepseek
        : /bytedance|seedance/.test(key)
          ? bytedance
          : /fal/.test(key)
            ? fal
            : /openai|gpt/.test(key)
              ? openai
              : null;
  return src ? (
    <img
      className="provider-mark"
      src={src}
      width={size}
      height={size}
      alt=""
      aria-hidden="true"
    />
  ) : (
    <Cube aria-hidden="true" size={size} />
  );
}
