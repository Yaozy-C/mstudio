export const initialPanels = (hasClips: boolean) => ({
  media: true,
  preview: false,
  agent: true,
  inspector: false,
  timeline: hasClips,
});

type Panels = ReturnType<typeof initialPanels>;
export function toggleStudioPanel(p: Panels, key: keyof Panels): Panels {
  if (key === "agent" && p.inspector)
    return { ...p, inspector: false, agent: true };
  return { ...p, [key]: !p[key] };
}
