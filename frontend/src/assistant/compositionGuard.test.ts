import { expect, test } from "bun:test";
import { compositionGuard } from "./compositionGuard";

function key(
  guard: ReturnType<typeof compositionGuard>,
  isComposing = false,
  keyCode = 13,
) {
  let stopped = false;
  guard.onKeyDownCapture({
    nativeEvent: { isComposing, keyCode } as KeyboardEvent,
    stopPropagation: () => {
      stopped = true;
    },
  });
  return stopped;
}

test("IME confirmation does not reach send or mention selection", () => {
  const guard = compositionGuard();
  guard.onCompositionStart();
  expect(key(guard)).toBe(true);
  expect(key(guard, true)).toBe(true);
  guard.onCompositionEnd();
  // WebKit reports compositionend before the Enter used to commit p in 480p.
  expect(key(guard, false, 229)).toBe(true);
  // A separate Enter can still send immediately, without a time-based lockout.
  expect(key(guard)).toBe(false);
});

test("native composition and focus reset are respected", () => {
  const guard = compositionGuard();
  expect(key(guard, true)).toBe(true);
  guard.onCompositionStart();
  guard.onBlur();
  expect(key(guard)).toBe(false);
});
