import { t, useLanguage } from "../i18n";
import type { ReactElement } from "react";
import { ContextMenu } from "@radix-ui/themes";
export type ObjectAction = {
  label: string;
  run: () => void;
  disabled?: boolean;
  danger?: boolean;
};
export function ObjectMenu({
  children,
  actions,
}: {
  children: ReactElement;
  actions: ObjectAction[];
}) {
  useLanguage();
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger
        onContextMenuCapture={(e) => {
          if (
            (e.target as HTMLElement).closest(
              "input, textarea, [contenteditable=true]",
            )
          )
            e.stopPropagation();
        }}
      >
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Content
        className="studio-menu object-menu"
        aria-label={t("对象操作")}
      >
        {actions.map((action) => (
          <ContextMenu.Item
            key={action.label}
            disabled={action.disabled}
            color={action.danger ? "red" : undefined}
            onSelect={action.run}
          >
            {action.label}
          </ContextMenu.Item>
        ))}
      </ContextMenu.Content>
    </ContextMenu.Root>
  );
}
