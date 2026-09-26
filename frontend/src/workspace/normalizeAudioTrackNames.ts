import type { Project } from "../model";

export function normalizeAudioTrackNames(project: Project): Project {
  let changed = false;
  let audioIndex = 0;
  const tracks = project.tracks.map((track) => {
    if (track.kind !== "audio") return track;
    audioIndex += 1;
    const legacyDefault =
      (track.id === "a1" && track.name === "配音") ||
      (track.id === "a2" && track.name === "音乐");
    if (!legacyDefault) return track;
    changed = true;
    return { ...track, name: `音轨 ${audioIndex}` };
  });
  return changed ? { ...project, tracks } : project;
}
