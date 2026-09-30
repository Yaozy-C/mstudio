import { expect, test } from "bun:test";
import { newProject } from "../model";
import {
  addCaptionTrack,
  captionTrackId,
  captionTracksOf,
  removeCaptionTrack,
  orderedCaptions,
} from "./captionTracks";

test("legacy captions stay on the first lane when more lanes are added and round-tripped", () => {
  const p = newProject("Caption tracks");
  p.captions = [{ id: "old", text: "old", start: 0, end: 3 }];
  const next = addCaptionTrack(addCaptionTrack(p));
  expect(captionTracksOf(next)).toHaveLength(3);
  expect(captionTrackId(next, next.captions[0])).toBe("captions");
  expect(JSON.parse(JSON.stringify(next)).captionTracks).toEqual(
    next.captionTracks,
  );
});
test("removing a lane deletes only its captions, and upper lanes composite last", () => {
  const p = addCaptionTrack(newProject("Caption tracks"));
  const second = captionTracksOf(p)[1].id;
  p.captions = [
    { id: "a", text: "a", start: 0, end: 3 },
    { id: "b", text: "b", trackId: second, start: 0, end: 3 },
  ];
  expect(orderedCaptions(p).map((c) => c.id)).toEqual(["b", "a"]);
  const next = removeCaptionTrack(p, second);
  expect(next.captions.map((c) => c.id)).toEqual(["a"]);
  expect(p.captions).toHaveLength(2);
  expect(removeCaptionTrack(p, "missing")).toBe(p);
});
