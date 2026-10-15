import { t, useLanguage } from "../i18n";
import { Fragment, useRef, useState } from "react";
import { DropdownMenu } from "@radix-ui/themes";
import { DotsThree } from "@phosphor-icons/react";
import { IconButton } from "./IconButton";
import type { ObjectAction } from "./ObjectMenu";

/** Share Radix keyboard navigation, dismissal and focus restoration. */
export function ObjectActions({ actions }: { actions: ObjectAction[] }) {
  useLanguage();
  const trigger = useRef<HTMLButtonElement>(null);
  const [container, setContainer] = useState<HTMLElement | null>(null);
  return (
    <DropdownMenu.Root
      onOpenChange={(open) => {
        if (open) setContainer(trigger.current?.closest("dialog") ?? null);
      }}
    >
      <DropdownMenu.Trigger>
        <IconButton ref={trigger} icon={DotsThree} label={t("对象操作")} />
      </DropdownMenu.Trigger>
      <DropdownMenu.Content
        className="studio-menu object-menu"
        container={container}
        align="end"
        sideOffset={6}
        collisionPadding={8}
        aria-label={t("对象操作")}
      >
        {actions.map((action, index) => (
          <Fragment key={action.label}>
            {index > 0 && !!action.danger !== !!actions[index - 1].danger && (
              <DropdownMenu.Separator />
            )}
            <DropdownMenu.Item
              key={action.label}
              disabled={action.disabled}
              color={action.danger ? "red" : undefined}
              onSelect={action.run}
            >
              {action.icon && <action.icon size={16} aria-hidden="true" />}
              {action.label}
            </DropdownMenu.Item>
          </Fragment>
        ))}
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
}
