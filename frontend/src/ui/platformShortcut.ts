/** Match existing shortcut labels to the active desktop platform. */
export function platformShortcut(label: string): string {
  return /Mac|iPhone|iPad/.test(navigator.platform)
    ? label
    : label.replace(/⇧⌘/g, "Ctrl+Shift+").replace(/⌘/g, "Ctrl+");
}
