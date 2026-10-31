import { Dialog } from "@radix-ui/themes";
import { Check, FilmSlate, X } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import type { Asset, Project } from "../model";
import { IconButton } from "../ui/IconButton";
import { productionShots } from "../production/items";
import { editReferences } from "../production/editReferences";

export function AssetShotLinks({
  asset,
  project,
  change,
  close,
  restoreFocus,
}: {
  asset?: Asset;
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  close: () => void;
  restoreFocus: () => void;
}) {
  useLanguage();
  const shots = productionShots(project);
  return (
    <Dialog.Root
      open={!!asset}
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <Dialog.Content
        className="story-assets-picker"
        maxWidth="640px"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocus();
        }}
      >
        <header>
          <Dialog.Title>{t("关联镜头")}</Dialog.Title>
          <Dialog.Close>
            <IconButton icon={X} label={t("关闭")} />
          </Dialog.Close>
        </header>
        <Dialog.Description>
          {t("选择使用此资产的镜头：{name}", { name: asset?.name })}
        </Dialog.Description>
        <div className="story-assets-choices asset-shot-choices">
          {shots.map((shot) => {
            const selected =
              shot.references?.some((r) => r.assetId === asset?.id) ?? false;
            return (
              <button
                key={shot.id}
                aria-pressed={selected}
                disabled={asset?.missing}
                onClick={() => {
                  if (!asset) return;
                  change((p) => {
                    const before =
                      p.nodes.find((n) => n.id === shot.id)?.references ?? [];
                    return editReferences(
                      p,
                      shot.id,
                      before,
                      before.some((r) => r.assetId === asset.id)
                        ? before.filter((r) => r.assetId !== asset.id)
                        : [
                            ...before,
                            { assetId: asset.id, purpose: asset.name },
                          ],
                    );
                  });
                }}
              >
                <FilmSlate />
                <span>
                  {String(shot.shot!.order).padStart(2, "0")} · {shot.title}
                </span>
                {selected && <Check />}
              </button>
            );
          })}
          {!shots.length && (
            <p>{t("还没有镜头。先在脚本中设计镜头，再关联资产。")}</p>
          )}
        </div>
        <footer>
          <Dialog.Close>
            <button>{t("完成")}</button>
          </Dialog.Close>
        </footer>
      </Dialog.Content>
    </Dialog.Root>
  );
}
