import {
  ChatText as MenuChatText,
  Cube as MenuCube,
  FilmStrip as MenuFilmStrip,
  FolderPlus as MenuFolderPlus,
  PencilSimple as MenuPencilSimple,
  Trash as MenuTrash,
  VideoCamera as MenuVideoCamera,
} from "@phosphor-icons/react";
import { t } from "../i18n";
import { openPrevis } from "../previs/context";
import { native } from "../bridge";
import { isLibraryAsset } from "../workspace/assetLibrary";
import type { Asset } from "../model";
import type { ProductionItem } from "./types";
import type { ProductionController } from "./useProduction";
import type { ObjectAction } from "../ui/ObjectMenu";
export function canvasCardActions({
  item,
  asset,
  canvas,
  preview,
  collect,
  onAdd,
}: {
  item: ProductionItem;
  asset?: Asset;
  canvas: ProductionController;
  preview: (item: ProductionItem) => void;
  collect: (asset: Asset) => void;
  onAdd: (asset: Asset) => void;
}): ObjectAction[] {
  return [
    ...(item.kind === "script" && item.nodeId
      ? [
          {
            icon: MenuCube,
            label: t("三维预演"),
            run: () => openPrevis(item.nodeId!),
            disabled: !native,
          },
        ]
      : []),
    {
      icon: MenuPencilSimple,
      label: t("预览 / 编辑"),
      run: () => preview(item),
    },
    {
      icon: MenuChatText,
      label: t("引用到对话"),
      run: () => canvas.reference([item.key]),
    },
    ...(asset &&
    (asset.kind === "image" || asset.kind === "video" || asset.kind === "audio")
      ? [
          {
            icon: MenuFilmStrip,
            label: t("加入时间线"),
            run: () => onAdd(asset),
            disabled: !!asset.missing,
          },
        ]
      : []),
    ...(asset
      ? [
          {
            icon: MenuFolderPlus,
            label: isLibraryAsset(asset)
              ? t("已在项目素材中")
              : t("保存为项目素材"),
            run: () => collect(asset),
            disabled: isLibraryAsset(asset),
          },
        ]
      : []),
    {
      icon: MenuVideoCamera,
      label: t("制作视频"),
      run: () => canvas.act("video", [item.key]),
    },
    { icon: MenuTrash, label: t("从画布移除"), run: () => canvas.remove(item) },
  ];
}
