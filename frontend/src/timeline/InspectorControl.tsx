import { t, useLanguage } from "../i18n";
import { useId, type ComponentType } from "react";
import type { IconProps } from "@phosphor-icons/react";
export function InspectorControl({
  label,
  icon: Icon,
  value,
  unit = "",
  min,
  max,
  step = 1,
  slider = false,
  endpoints,
  change,
}: {
  label: string;
  icon?: ComponentType<IconProps>;
  value: number;
  unit?: string;
  min: number;
  max: number;
  step?: number;
  slider?: boolean;
  endpoints?: [string, string];
  change: (value: number) => void;
}) {
  useLanguage();
  const id = useId();
  const update = (number: number) => {
    if (Number.isFinite(number) && number >= min && number <= max)
      change(number);
  };
  return (
    <div className="inspector-control">
      <div className="inspector-control-row">
        <label htmlFor={id}>
          {Icon && <Icon size={20} aria-hidden="true" />}
          {label}
        </label>
        <div className="inspector-number">
          <input
            id={id}
            type="number"
            value={Number(value.toFixed(3))}
            min={min}
            max={max}
            step={step}
            onChange={(e) => update(e.target.valueAsNumber)}
          />
          <span>{unit}</span>
        </div>
      </div>
      {slider && (
        <input
          className="inspector-range"
          aria-label={t("{v0}滑杆", { v0: label })}
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => update(e.target.valueAsNumber)}
        />
      )}
      {endpoints && (
        <div className="inspector-endpoints">
          <span>{endpoints[0]}</span>
          <span>{endpoints[1]}</span>
        </div>
      )}
    </div>
  );
}
