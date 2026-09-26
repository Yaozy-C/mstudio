import type { KeyboardEvent } from "react";

// WebKit can end composition before dispatching the confirming Enter. That
// keydown still carries legacy keyCode 229 even when isComposing is false.
export function compositionGuard() {
  let composing = false;
  return {
    onCompositionStart: () => {
      composing = true;
    },
    onCompositionEnd: () => {
      composing = false;
    },
    onBlur: () => {
      composing = false;
    },
    onKeyDownCapture: (
      event: Pick<KeyboardEvent, "nativeEvent" | "stopPropagation">,
    ) => {
      if (
        composing ||
        event.nativeEvent.isComposing ||
        event.nativeEvent.keyCode === 229
      ) {
        // Stop composer/mention shortcuts, including the library's own send
        // handler, without preventing the input method's native text commit.
        event.stopPropagation();
      }
    },
  };
}
