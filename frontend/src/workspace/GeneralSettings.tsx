import { AppearanceSettings } from "../appearance/AppearanceSettings";
import { AboutSettings } from "../updates/AboutSettings";
import { t, useLanguage } from "../i18n";
import { LanguageSettings } from "./LanguageSettings";
import { StorageSettings } from "./StorageSettings";

export function GeneralSettings({ projectId }: { projectId?: string }) {
  useLanguage();
  return (
    <div className="general-settings">
      <section className="general-language">
        <h2>{t("语言")}</h2>
        <LanguageSettings />
      </section>
      <AppearanceSettings />
      <StorageSettings projectId={projectId} />
      <AboutSettings />
    </div>
  );
}
