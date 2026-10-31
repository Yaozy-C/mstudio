import {
  ChatText as MenuChatText,
  Eye as MenuEye,
  SquaresFour,
  FolderPlus as MenuFolderPlus,
  Trash as MenuTrash,
} from "@phosphor-icons/react";
import { isStoryAsset } from "./assetLibrary";
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
  onReference,
  onPlace,
  onUncollect,
  onUsage,
}: {
  a: Asset;
  scope: "project" | "global";
  disabled: boolean;
  library: ReturnType<typeof useAssetLibrary>;
  projectAssets: Asset[];
  setPreview: (id: string) => void;
  setRemoving: (asset: Asset) => void;
  onReference: (asset: Asset) => void;
  onPlace?: (asset: Asset) => void;
  onUncollect: (asset: Asset) => void;
  onUsage: (asset: Asset, usage: "asset" | "material") => void;
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
            icon: MenuFolderPlus,
            label: isStoryAsset(a) ? t("已建立镜头资产") : t("建立镜头资产"),
            run: () => onUsage(a, "asset"),
            disabled:
              disabled ||
              isStoryAsset(a) ||
              !!a.missing ||
              !["image", "video", "audio"].includes(a.kind),
          },
          ...(onPlace
            ? [
                {
                  icon: SquaresFour,
                  label: t("放到草稿画布"),
                  run: () => onPlace(a),
                  disabled: !!a.missing,
                },
              ]
            : []),
          {
            icon: MenuChatText,
            label: t("用于创意或脚本"),
            run: () => onReference(a),
            disabled: a.missing,
          },
          {
            icon: MenuFolderPlus,
            label: t("取消收录"),
            run: () => onUncollect(a),
            disabled,
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
