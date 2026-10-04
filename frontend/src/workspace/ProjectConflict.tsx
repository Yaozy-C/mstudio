import { AlertDialog, Button, Flex } from "@radix-ui/themes";
import { t } from "../i18n";
import type { useProject } from "./useProject";
export function ProjectConflict({
  model,
  error,
}: {
  model: ReturnType<typeof useProject>;
  error: (message: string) => void;
}) {
  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger>
        <Button size="1" variant="soft">
          {t("处理修改冲突")}
        </Button>
      </AlertDialog.Trigger>
      <AlertDialog.Content maxWidth="440px">
        <AlertDialog.Title>{t("同一内容有新的修改")}</AlertDialog.Title>
        <AlertDialog.Description>
          {t(
            "你的未保存修改仍在当前页面。保留自己的修改会覆盖冲突内容，其他已保存修改仍保留。",
          )}
        </AlertDialog.Description>
        <Flex gap="2" justify="end" mt="4" wrap="wrap">
          <AlertDialog.Cancel>
            <Button variant="soft">{t("取消")}</Button>
          </AlertDialog.Cancel>
          {(["current", "local"] as const).map((choice) => (
            <AlertDialog.Action key={choice}>
              <Button
                variant={choice === "local" ? "solid" : "outline"}
                onClick={() =>
                  void model
                    .resolveConflict(choice)
                    .catch((e) => error(String(e)))
                }
              >
                {t(choice === "local" ? "保留我的修改" : "放弃未保存修改")}
              </Button>
            </AlertDialog.Action>
          ))}
        </Flex>
      </AlertDialog.Content>
    </AlertDialog.Root>
  );
}
