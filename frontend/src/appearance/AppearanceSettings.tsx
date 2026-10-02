import { useState } from "react";
import { ArrowCounterClockwise } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { ActionButton } from "../ui/ActionButton";
import {
  defaultAppearance,
  setAppearance,
  useAppearance,
  type Appearance,
} from "./preferences";
import "./appearance.css";
import { AppearancePreview } from "./AppearancePreview";
export function AppearanceSettings() {
  useLanguage();
  const value = useAppearance();
  const [notice, setNotice] = useState("");
  const update = (next: Appearance) =>
    setNotice(
      setAppearance(next)
        ? ""
        : "本次外观已生效，但未能保存偏好，下次打开可能恢复默认。",
    );
  return (
    <section className="appearance-settings">
      <div className="appearance-heading">
        <h2>{t("外观与显示")}</h2>
        <ActionButton
          icon={ArrowCounterClockwise}
          disabled={value.surface === "paper" && value.contrast === "standard"}
          onClick={() => update(defaultAppearance)}
        >
          {t("恢复默认外观")}
        </ActionButton>
      </div>
      <fieldset className="appearance-options">
        <legend>{t("界面底色")}</legend>
        {(
          [
            ["paper", t("暖纸色")],
            ["white", t("中性浅色")],
            ["slate", t("冷灰色")],
          ] as const
        ).map(([surface, title]) => (
          <label className="appearance-option" key={surface}>
            <AppearancePreview surface={surface} contrast={value.contrast} />
            <span className="appearance-option-label">
              <input
                type="radio"
                name="appearance-surface"
                checked={value.surface === surface}
                onChange={() => update({ ...value, surface })}
              />
              <strong>{title}</strong>
            </span>
          </label>
        ))}
      </fieldset>
      <label className="appearance-contrast">
        <span>
          <strong>{t("增强界面对比度")}</strong>
        </span>
        <input
          type="checkbox"
          checked={value.contrast === "high"}
          onChange={(event) =>
            update({
              ...value,
              contrast: event.target.checked ? "high" : "standard",
            })
          }
        />
      </label>
      {notice && (
        <p className="general-description" role="status">
          {t(notice)}
        </p>
      )}
    </section>
  );
}
