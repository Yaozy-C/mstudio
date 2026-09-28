import { duration, type Project } from "../model";

// Stable timeline order, independent of the storage order. Preserve exact
// durations so non-frame-aligned retimes do not leave tiny gaps at every seam.
export function packClips(project: Project, trackId?: string): Project {
  const starts = new Map<string, number>();
  for (const track of project.tracks) {
    if (trackId !== undefined && track.id !== trackId) continue;
    let end = 0;
    const clips = project.clips
      .filter((clip) => clip.trackId === track.id)
      .sort((a, b) => a.start - b.start);
    for (const clip of clips) {
      starts.set(clip.id, end);
      end += duration(clip);
    }
  }
  const clips = project.clips.map((clip) => {
    const start = starts.get(clip.id);
    return start === undefined || start === clip.start
      ? clip
      : { ...clip, start };
  });
  return clips.every((clip, i) => clip === project.clips[i])
    ? project
    : { ...project, clips };
}
