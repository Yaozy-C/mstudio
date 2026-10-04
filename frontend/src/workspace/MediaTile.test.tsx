import { expect, test } from "bun:test";
import { createRef, type MouseEvent } from "react";
import { MediaTile } from "./MediaTile";

test("media tiles forward menu trigger props and select before opening the menu", () => {
  const calls: string[] = [];
  const ref = createRef<HTMLElement>();
  const capture = () => {};
  const tile = MediaTile({
    asset: {
      id: "image-1",
      name: "test.png",
      kind: "image",
      path: "/test.png",
      preview: "",
      duration: 1,
      width: 100,
      height: 100,
      hasAudio: false,
    },
    scope: "project",
    selected: false,
    select: (id) => calls.push(id),
    preview: () => {},
    ref,
    className: "menu-trigger",
    onContextMenuCapture: capture,
    onContextMenu: (event) => {
      event.preventDefault();
      calls.push("open-menu");
    },
  });
  let prevented = false;
  tile.props.onContextMenu({
    preventDefault: () => {
      prevented = true;
    },
  } as MouseEvent<HTMLElement>);
  expect(calls).toEqual(["image-1", "open-menu"]);
  expect(prevented).toBe(true);
  expect(tile.props.ref).toBe(ref);
  expect(tile.props.onContextMenuCapture).toBe(capture);
  expect(tile.props.className).toBe("media-item menu-trigger");
});
