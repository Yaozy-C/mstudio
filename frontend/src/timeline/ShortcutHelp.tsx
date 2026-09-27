import { Popover } from "@radix-ui/themes";
export function ShortcutHelp() {
  return (
    <Popover.Root>
      <Popover.Trigger>
        <button title="查看时间线快捷键">快捷键</button>
      </Popover.Trigger>
      <Popover.Content width="340px">
        <strong>时间线快捷键</strong>
        <p>先选中片段，点击时间尺定位播放头。</p>
        <table>
          <tbody>
            {[
              ["复制 / 剪切 / 粘贴", "⌘C / ⌘X / ⌘V"],
              ["在播放头分割", "⌘B"],
              ["裁掉播放头之前 / 之后", "⌥[ / ⌥]"],
              ["删除选中片段", "Delete / Backspace"],
              ["撤销 / 重做", "⌘Z / ⇧⌘Z"],
              ["播放 / 暂停", "Space"],
              ["前后逐帧 / 10 帧", "← → / ⇧← →"],
            ].map(([label, keys]) => (
              <tr key={label}>
                <td>{label}</td>
                <td>
                  <kbd>{keys}</kbd>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p>
          Windows 使用 Ctrl 替代 ⌘、Alt 替代
          ⌥。粘贴到播放头和原轨道；剪切、删除、裁剪保留空隙。片段剪贴板限当前工程，编辑文字时保留文字快捷键。
        </p>
      </Popover.Content>
    </Popover.Root>
  );
}
