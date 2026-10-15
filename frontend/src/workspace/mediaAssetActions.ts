import {
  ChatText as MenuChatText,
  Eye as MenuEye,
  FilmStrip as MenuFilmStrip,
  FolderPlus as MenuFolderPlus,
  SquaresFour as MenuSquaresFour,
  Trash as MenuTrash,
} from "@phosphor-icons/react";
import { t } from "../i18n";
import type { Asset } from "../model";
import type { useAssetLibrary } from "./useAssetLibrary";
import type { ObjectAction } from "../ui/ObjectMenu";
export function mediaAssetActions({
  a,
  scope,
  disabled,
  library,
  projectAssets,
  setPreview,
  setRemoving,
  onPlace,
  onAdd,
  onReference,
}: {
  a: Asset;
  scope: "project" | "global" | "effects";
  disabled: boolean;
  library: ReturnType<typeof useAssetLibrary>;
  projectAssets: Asset[];
  setPreview: (id: string) => void;
  setRemoving: (asset: Asset) => void;
  onPlace: (asset: Asset) => void;
  onAdd: (asset: Asset) => void;
  onReference: (asset: Asset) => void;
}): ObjectAction[] {
  return [
    {
      icon: MenuEye,
      label: t("预览素材"),
      run: () => setPreview(a.id),
    },
    ...(scope === "project"
      ? [
          {
            icon: MenuSquaresFour,
            label: t("放到画布"),
            run: () => onPlace(a),
            disabled: a.missing,
          },
          {
            icon: MenuFilmStrip,
            label: t("加入时间线"),
            run: () => onAdd(a),
            disabled: a.missing || a.kind === "text" || a.kind === "document",
          },
          {
            icon: MenuChatText,
            label: t("引用到对话"),
            run: () => onReference(a),
            disabled: a.missing,
          },
          {
            icon: MenuFolderPlus,
            label: t("添加到公共素材库"),
            run: () => void library.promote(a),
            disabled: disabled || library.assets.some((g) => g.id === a.id),
          },
        ]
      : [
          {
            icon: MenuFolderPlus,
            label: t("加入项目素材"),
            run: () => void library.use(a),
            disabled: disabled || projectAssets.some((p) => p.id === a.id),
          },
        ]),
    {
      icon: MenuTrash,
      label: scope === "project" ? t("删除项目素材") : t("从公共素材移除"),
      danger: true,
      disabled,
      run: () => setRemoving(a),
    },
  ];
}
