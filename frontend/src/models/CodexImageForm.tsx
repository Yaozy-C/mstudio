import { CodexServiceForm } from "./CodexServiceForm";
import type { MediaModel } from "./mediaRegistry";
import { t, useLanguage } from "../i18n";
export function CodexImageForm({
  initial,
  cancel,
}: {
  initial: MediaModel;
  save: (model: MediaModel) => Promise<void>;
  cancel: () => void;
}) {
  useLanguage();
  return (
    <section className="model-form">
      <h3>{t("连接 Codex")}</h3>
      <CodexServiceForm
        connection={{
          id: initial.connectionId || crypto.randomUUID(),
          name: "Codex",
          kind: "codex",
          endpoint: "codex://local",
          hasKey: false,
          modelCount: 0,
        }}
        saved={cancel}
        cancel={cancel}
      />
    </section>
  );
}
