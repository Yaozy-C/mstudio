import { AppearanceSettings } from "../appearance/AppearanceSettings";
import { AboutSettings } from "../updates/AboutSettings";
import { t, useLanguage } from "../i18n";
import { LanguageSettings } from "./LanguageSettings";
import { StorageSettings } from "./StorageSettings";
import "../styles/general-settings.css";

export function GeneralSettings({ projectId }: { projectId?: string }) {
  useLanguage();
  return (
    <div className="general-settings">
      <div className="settings-card settings-display">
        <section className="general-language">
          <h2>{t("语言")}</h2>
          <LanguageSettings />
        </section>
        <AppearanceSettings />
        <AboutSettings />
      </div>
      <div className="settings-card settings-storage">
        <StorageSettings projectId={projectId} />
      </div>
    </div>
  );
}
