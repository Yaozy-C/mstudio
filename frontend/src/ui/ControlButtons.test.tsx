import { expect, test } from "bun:test";
import { renderToStaticMarkup as render } from "react-dom/server";
import { Plus } from "@phosphor-icons/react";
import { ActionButton } from "./ActionButton";
import { IconButton } from "./IconButton";

test("icon actions have a name, do not submit forms and block pending clicks", () => {
  const html = render(<IconButton icon={Plus} label="Add" busy />);
  expect(html).toContain('aria-label="Add"');
  expect(html).toContain('title="Add"');
  expect(html).toContain('type="button"');
  expect(html).toContain('disabled=""');
  expect(html).toContain('aria-busy="true"');
});

test("inline pending actions retain both label widths with only one accessible label", () => {
  const html = render(
    <ActionButton icon={Plus} busy busyLabel="Adding">
      Add
    </ActionButton>,
  );
  expect(html).toContain('disabled=""');
  expect(html).toContain('aria-hidden="true" data-hidden="true">Add</span>');
  expect(html).toContain(
    'aria-hidden="false" data-hidden="false">Adding</span>',
  );
  const idle = render(
    <ActionButton icon={Plus} aria-pressed={true}>
      Add
    </ActionButton>,
  );
  expect(idle).not.toContain('disabled=""');
  expect(idle).toContain('aria-pressed="true"');
});
