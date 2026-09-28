import { ActionButton } from "../ui/ActionButton";
import { useState } from "react";
import { Sun, ArrowCounterClockwise } from "@phosphor-icons/react";
import { t } from "../i18n";
import { defaultGrade, gradeControls, type Grade } from "./grading";
import { InspectorControl } from "./InspectorControl";

export function GradeFields({
  grade,
  change,
}: {
  grade?: Grade;
  change: (grade?: Grade) => void;
}) {
  const [band, setBand] = useState(0);
  const [zone, setZone] = useState(0);
  const [channel, setChannel] = useState(0);
  const value = grade ?? defaultGrade();
  const point = (
    key: "hsl" | "curves" | "wheels",
    row: number,
    column: number,
    n: number,
  ) => {
    const next = structuredClone(value);
    next[key][row][column] = n;
    change(next);
  };
  const selector = (
    label: string,
    values: string[],
    selected: number,
    select: (n: number) => void,
  ) => (
    <label className="inspector-select-row">
      <span>{label}</span>
      <select
        aria-label={label}
        value={selected}
        onChange={(e) => select(+e.target.value)}
      >
        {values.map((name, i) => (
          <option key={name} value={i}>
            {t(name)}
          </option>
        ))}
      </select>
    </label>
  );
  return (
    <details className="inspector-section" open={grade ? true : undefined}>
      <summary>{t("自定义调色")}</summary>
      <ActionButton
        icon={ArrowCounterClockwise}
        disabled={!grade}
        onClick={() => change(undefined)}
      >
        {t("重置调色方案")}
      </ActionButton>
      {(
        Object.entries(gradeControls) as [
          keyof typeof gradeControls,
          readonly [string, number, number],
        ][]
      ).map(([key, [label, min, max]]) => (
        <InspectorControl
          key={key}
          label={t(label)}
          icon={Sun}
          value={value[key]}
          min={min}
          max={max}
          unit={key === "exposure" ? "EV" : ""}
          step={key === "exposure" ? 0.05 : 1}
          slider
          change={(n) => change({ ...value, [key]: n })}
        />
      ))}
      {selector(
        t("颜色混合"),
        ["红", "橙", "黄", "绿", "青", "蓝", "紫", "洋红"],
        band,
        setBand,
      )}
      {["色相", "饱和度", "明度"].map((label, i) => (
        <InspectorControl
          key={label}
          label={t(label)}
          icon={Sun}
          value={value.hsl[band][i]}
          min={-100}
          max={100}
          slider
          change={(n) => point("hsl", band, i, n)}
        />
      ))}
      {selector(t("分区调色"), ["阴影", "中间调", "高光"], zone, setZone)}
      {["色相", "饱和度", "明度"].map((label, i) => (
        <InspectorControl
          key={label}
          label={t(label)}
          icon={Sun}
          value={value.wheels[zone][i]}
          min={i < 2 ? 0 : -100}
          max={i === 0 ? 360 : 100}
          slider
          change={(n) => point("wheels", zone, i, n)}
        />
      ))}
      {selector(t("曲线通道"), ["RGB", "红", "绿", "蓝"], channel, setChannel)}
      {[25, 50, 75].map((x, i) => (
        <InspectorControl
          key={x}
          label={`${x}%`}
          icon={Sun}
          value={value.curves[channel][i] * 100}
          min={i === 0 ? 0 : value.curves[channel][i - 1] * 100}
          max={i === 2 ? 100 : value.curves[channel][i + 1] * 100}
          slider
          change={(n) => point("curves", channel, i, n / 100)}
        />
      ))}
    </details>
  );
}
