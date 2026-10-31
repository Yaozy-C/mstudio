import {
  ChatText as MenuChatText,
  FilmStrip as MenuFilmStrip,
  FolderPlus as MenuFolderPlus,
  PencilSimple as MenuPencilSimple,
  Trash as MenuTrash,
  VideoCamera as MenuVideoCamera,
} from "@phosphor-icons/react";
import { t } from "../i18n";
import { isShotAsset } from "../workspace/resourceRoles";
import type { Asset } from "../model";
import type { DraftItem } from "./types";
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
  item: DraftItem;
  asset?: Asset;
  canvas: ProductionController;
  preview: (item: DraftItem) => void;
  collect: (asset: Asset) => void;
  onAdd: (asset: Asset) => void;
}): ObjectAction[] {
  return [
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
    ...(asset && (asset.kind === "image" || asset.kind === "video")
      ? [
          {
            icon: MenuFilmStrip,
            label: t("用于正式镜头"),
            run: () => onAdd(asset),
            disabled: !!asset.missing,
          },
        ]
      : []),
    ...(asset && ["image", "video", "audio"].includes(asset.kind)
      ? [
          {
            icon: MenuFolderPlus,
            label: isShotAsset(asset) ? t("已确认资产") : t("确认为镜头资产"),
            run: () => collect(asset),
            disabled: isShotAsset(asset) || !!asset.missing,
          },
        ]
      : []),
    {
      icon: MenuPencilSimple,
      label: t("试生成图片"),
      run: () => canvas.act("image", [item.key]),
    },
    {
      icon: MenuVideoCamera,
      label: t("试生成视频"),
      run: () => canvas.act("video", [item.key]),
    },
    { icon: MenuTrash, label: t("从画布移除"), run: () => canvas.remove(item) },
  ];
}
