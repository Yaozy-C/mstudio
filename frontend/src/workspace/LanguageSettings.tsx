import { setLanguage, t, useLanguage } from "../i18n";
import "../styles/language-settings.css";

export function LanguageSettings() {
  const language = useLanguage();
  return (
    <section className="language-settings">
      <fieldset className="language-options" aria-label={t("界面语言")}>
        {(
          [
            ["zh-CN", "简体中文"],
            ["en", "English"],
          ] as const
        ).map(([value, name]) => (
          <label className="language-option" key={value}>
            <input
              type="radio"
              name="interface-language"
              value={value}
              checked={language === value}
              onChange={() => setLanguage(value)}
            />
            <span lang={value}>{name}</span>
          </label>
        ))}
      </fieldset>
    </section>
  );
}
