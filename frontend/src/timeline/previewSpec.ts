import { orderedCaptions } from "./captionTracks";
import type { Project } from "../model";

/** UI labels and canvas associations do not change rendered media. */
export function previewSpec(project: Project): string {
  return JSON.stringify({
    width: project.width,
    height: project.height,
    fps: project.fps,
    clips: project.clips.map(({ shotId: _shot, ...clip }) => clip),
    tracks: project.tracks.map(
      ({ name: _name, sourceTrackId: _source, ...track }) => ({
        ...track,
        name: "",
      }),
    ),
    captions: orderedCaptions(project),
  });
}
