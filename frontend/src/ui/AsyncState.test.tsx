import { expect, test } from "bun:test";
import { renderToStaticMarkup as render } from "react-dom/server";
import { AsyncButton, LoadingState, StatusMessage } from "./AsyncState";
import { ErrorNotice } from "../errors/ErrorNotice";
import { RunStatus } from "../production/RunStatus";
import type { ProductionTask } from "../production/types";
import { progressRatio } from "../workspace/exportProgress";

test("pending actions disable resubmission and expose only the current label", () => {
  const busy = render(
    <AsyncButton busy busyLabel="Saving">
      Save
    </AsyncButton>,
  );
  expect(busy).toContain('disabled=""');
  expect(busy).toContain('aria-busy="true"');
  expect(busy).toContain('class="async-button-label" aria-hidden="true"');
  expect(busy).toContain('class="async-button-pending" aria-hidden="false"');
  const idle = render(<AsyncButton busyLabel="Saving">Save</AsyncButton>);
  expect(idle).not.toContain('disabled=""');
  expect(idle).toContain('class="async-button-pending" aria-hidden="true"');
});
test("loading has one announced status and hides decorative placeholders", () => {
  const html = render(<LoadingState label="Loading connections" />);
  expect(html.match(/role="status"/g)).toHaveLength(1);
  expect(html).toContain('class="loading-skeleton" aria-hidden="true"');
  expect(html).not.toContain('role="alert"');
  expect(render(<StatusMessage kind="error">Failed</StatusMessage>)).toContain(
    'role="alert"',
  );
});
test("failures retain recovery and collapsed diagnostics while cancellation is not an alert", () => {
  const html = render(
    <ErrorNotice error="network unavailable" fallback="NETWORK_ERROR">
      <button>Retry</button>
    </ErrorNotice>,
  );
  expect(html).toContain('role="alert"');
  expect(html).toContain("<button>Retry</button>");
  expect(html).toContain("<details>");
  expect(html).not.toContain("<details open");
  const cancelled = render(
    <ErrorNotice error={JSON.stringify({ code: "CHAT_STOPPED" })} />,
  );
  expect(cancelled).toContain('role="status"');
  expect(cancelled).not.toContain('role="alert"');
});
test("paused, unknown and final tasks never imply ongoing work", () => {
  const task: ProductionTask = {
    key: "test",
    kind: "image",
    mode: "single",
    prompt: "test",
    inputs: [],
    modelId: "test",
    status: "IN_PROGRESS",
  };
  expect(render(<RunStatus task={task} />)).toContain("status-spinner");
  for (const status of ["UNKNOWN", "FAILED", "COMPLETED", "CANCELLED"])
    expect(render(<RunStatus task={{ ...task, status }} />)).not.toContain(
      "status-spinner",
    );
  expect(
    render(<RunStatus task={{ ...task, trackingPaused: true }} />),
  ).not.toContain("status-spinner");
});
test("export progress uses real counts and rejects malformed events", () => {
  expect(progressRatio({ done: 25, total: 100 })).toBe(0.25);
  expect(progressRatio({ done: 105, total: 100 })).toBe(1);
  for (const input of [
    { done: 0, total: 0 },
    { done: -1, total: 100 },
    { done: Infinity, total: 100 },
    { done: 0, total: NaN },
  ])
    expect(progressRatio(input)).toBeNull();
});
