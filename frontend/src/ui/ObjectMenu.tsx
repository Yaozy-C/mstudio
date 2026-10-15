import type { Icon } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { Fragment, useRef, useState, type ReactElement } from "react";
import { ContextMenu } from "@radix-ui/themes";
export type ObjectAction = {
  label: string;
  icon?: Icon;
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
  const trigger = useRef<HTMLSpanElement>(null);
  const [container, setContainer] = useState<HTMLElement | null>(null);
  return (
    <ContextMenu.Root
      onOpenChange={(open) => {
        if (open) setContainer(trigger.current?.closest("dialog") ?? null);
      }}
    >
      <ContextMenu.Trigger
        ref={trigger}
        onContextMenuCapture={(e) => {
          if (
            (e.target as HTMLElement).closest(
              "input, textarea, [contenteditable=true]",
            )
          ) {
            e.preventDefault();
            e.stopPropagation();
          }
        }}
      >
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Content
        className="studio-menu object-menu"
        container={container}
        collisionPadding={8}
        aria-label={t("对象操作")}
      >
        {actions.map((action, index) => (
          <Fragment key={action.label}>
            {index > 0 && !!action.danger !== !!actions[index - 1].danger && (
              <ContextMenu.Separator />
            )}
            <ContextMenu.Item
              key={action.label}
              disabled={action.disabled}
              color={action.danger ? "red" : undefined}
              onSelect={action.run}
            >
              {action.icon && <action.icon size={16} aria-hidden="true" />}
              {action.label}
            </ContextMenu.Item>
          </Fragment>
        ))}
      </ContextMenu.Content>
    </ContextMenu.Root>
  );
}
