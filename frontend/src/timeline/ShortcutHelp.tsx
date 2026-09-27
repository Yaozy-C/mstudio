import { t, useLanguage } from "../i18n";
import { Popover } from "@radix-ui/themes";
import { Keyboard, X } from "@phosphor-icons/react";
import "../styles/shortcut-help.css";
export function ShortcutHelp() {
  useLanguage();
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  const cmd = mac ? "⌘" : "Ctrl+",
    alt = mac ? "⌥" : "Alt+";
  return (
    <Popover.Root>
      <Popover.Trigger>
        <button title={t("查看时间线快捷键")} aria-label={t("时间线快捷键")}>
          <Keyboard size={17} />
        </button>
      </Popover.Trigger>
      <Popover.Content
        className="shortcut-help"
        width="360px"
        side="top"
        align="end"
        sideOffset={12}
      >
        <header>
          <strong>{t("时间线快捷键")}</strong>
          <Popover.Close>
            <button aria-label={t("关闭快捷键")}>
              <X size={16} />
            </button>
          </Popover.Close>
        </header>
        <p className="shortcut-help-intro">
          {t("选中片段后操作，点击时间尺定位播放头。")}
        </p>
        <dl>
          {[
            [t("复制片段"), `${cmd}C`],
            [t("剪切片段"), `${cmd}X`],
            [t("粘贴到播放头"), `${cmd}V`],
            [t("分割片段"), `${cmd}B`],
            [t("裁掉播放头之前"), `${alt}[`],
            [t("裁掉播放头之后"), `${alt}]`],
            [t("删除片段"), mac ? "⌫" : "Delete"],
            [t("撤销 / 重做"), mac ? "⌘Z / ⇧⌘Z" : "Ctrl+Z / Ctrl+Shift+Z"],
            [t("播放 / 暂停"), "Space"],
            [t("逐帧移动"), "← / →"],
          ].map(([label, keys]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>
                <kbd>{keys}</kbd>
              </dd>
            </div>
          ))}
        </dl>
        <footer>
          {t("裁剪保留速度，剪切与删除保留空隙。输入文字时使用文字快捷键。")}
        </footer>
      </Popover.Content>
    </Popover.Root>
  );
}
