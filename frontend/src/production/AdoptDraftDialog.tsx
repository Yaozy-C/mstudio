import { Dialog } from "@radix-ui/themes";
import type { Asset, Project } from "../model";
import { t, useLanguage } from "../i18n";
import { productionShots } from "./items";
import { adoptDraft } from "./adoptDraft";
import type { ChangeProject } from "./types";

export function AdoptDraftDialog({
  asset,
  project,
  change,
  close,
}: {
  asset: Asset;
  project: Project;
  change: ChangeProject;
  close: () => void;
}) {
  useLanguage();
  const shots = productionShots(project);
  return (
    <Dialog.Root
      open
      onOpenChange={(v) => {
        if (!v) close();
      }}
    >
      <Dialog.Content className="story-assets-picker" maxWidth="640px">
        <Dialog.Title>{t("用于正式镜头")}</Dialog.Title>
        <Dialog.Description>
          {t("选择镜头，将草稿加入它的分镜图或视频版本。已有结果会保留。")}
        </Dialog.Description>
        <div className="story-assets-choices">
          {shots.map((shot) => (
            <button
              key={shot.id}
              onClick={() => {
                change((p) => adoptDraft(p, asset.id, shot.id));
                close();
              }}
            >
              {shot.title}
            </button>
          ))}
          {!shots.length && (
            <p>{t("还没有镜头。先在脚本中设计镜头，再关联资产。")}</p>
          )}
        </div>
        <Dialog.Close>
          <button>{t("取消")}</button>
        </Dialog.Close>
      </Dialog.Content>
    </Dialog.Root>
  );
}
