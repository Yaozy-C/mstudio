import { t } from "../i18n";
import type { TransitionDesign } from "./transitionDesign";
export function TransitionDesignFields({
  design: d,
  change,
}: {
  design: TransitionDesign;
  change: (d: TransitionDesign) => void;
}) {
  const numeric = (
    label: string,
    value: number,
    min: number,
    max: number,
    set: (n: number) => void,
    step = 0.05,
  ) => (
    <label>
      {t(label)}
      <input
        aria-label={t(label)}
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(e) => {
          if (e.target.value !== "") set(+e.target.value);
        }}
      />
    </label>
  );
  return (
    <div>
      <label>
        {t("画面交接")}
        <select
          value={d.mask}
          onChange={(e) =>
            change({ ...d, mask: e.target.value as TransitionDesign["mask"] })
          }
        >
          <option value="uniform">{t("整体混合")}</option>
          <option value="linear">{t("方向蒙版")}</option>
          <option value="radial">{t("径向蒙版")}</option>
        </select>
      </label>
      {numeric(
        "蒙版角度",
        d.angle,
        -180,
        180,
        (angle) => change({ ...d, angle }),
        1,
      )}
      {numeric(
        "边缘柔化",
        d.feather,
        0.001,
        1,
        (feather) => change({ ...d, feather }),
        0.01,
      )}
      {(["center", "outgoingOffset", "incomingOffset"] as const).map(
        (key, i) => (
          <div key={key}>
            {[0, 1].map((axis) => (
              <div key={axis}>
                {numeric(
                  `${["中心", "前镜移动", "后镜移动"][i]} ${axis === 0 ? "X" : "Y"}`,
                  d[key][axis],
                  key === "center" ? 0 : -1,
                  1,
                  (n) => {
                    const pair: [number, number] = [...d[key]];
                    pair[axis] = n;
                    change({ ...d, [key]: pair });
                  },
                )}
              </div>
            ))}
          </div>
        ),
      )}
      {numeric("前镜结束缩放", d.outgoingZoom, 1, 4, (outgoingZoom) =>
        change({ ...d, outgoingZoom }),
      )}
      {numeric("后镜开始缩放", d.incomingZoom, 1, 4, (incomingZoom) =>
        change({ ...d, incomingZoom }),
      )}
      <span>{t("进度曲线")}</span>
      {d.curve.slice(1, -1).map((point, i) => (
        <div key={i}>
          {numeric(
            "时间比例",
            point[0],
            d.curve[i][0] + 0.001,
            d.curve[i + 2][0] - 0.001,
            (n) => {
              const curve = structuredClone(d.curve);
              curve[i + 1][0] = n;
              change({ ...d, curve });
            },
          )}
          {numeric(
            "完成比例",
            point[1],
            d.curve[i][1],
            d.curve[i + 2][1],
            (n) => {
              const curve = structuredClone(d.curve);
              curve[i + 1][1] = n;
              change({ ...d, curve });
            },
          )}
          <button
            onClick={() =>
              change({ ...d, curve: d.curve.filter((_, j) => j !== i + 1) })
            }
          >
            {t("移除控制点")}
          </button>
        </div>
      ))}
      <button
        disabled={d.curve.length >= 8}
        onClick={() => {
          const curve = structuredClone(d.curve);
          const a = curve.at(-2)!;
          curve.splice(curve.length - 1, 0, [(a[0] + 1) / 2, (a[1] + 1) / 2]);
          change({ ...d, curve });
        }}
      >
        {t("添加控制点")}
      </button>
    </div>
  );
}
