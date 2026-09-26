export function ParameterChoices({
  label,
  values,
  value,
  change,
  ratios = false,
}: {
  label: string;
  values: string[];
  value?: string;
  change: (value: string | undefined) => void;
  ratios?: boolean;
}) {
  return (
    <div
      className={`generation-setting-field parameter-choices ${ratios ? "ratio-choices" : ""}`}
    >
      <span>{label}</span>
      <div role="group" aria-label={label}>
        {["", ...values].map((v) => (
          <button
            type="button"
            key={v}
            aria-pressed={(value || "") === v}
            onClick={() => change(v || undefined)}
          >
            {ratios && v && (
              <i style={{ aspectRatio: v.replace(":", " / ") }} />
            )}
            <span>{v || "默认"}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
