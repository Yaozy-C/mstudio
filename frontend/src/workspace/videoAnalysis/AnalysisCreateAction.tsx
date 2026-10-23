import { useRef, useState } from "react";
import { ArrowUpRight } from "@phosphor-icons/react";
import { t } from "../../i18n";
import { AsyncButton } from "../../ui/AsyncState";
import { ErrorNotice } from "../../errors/ErrorNotice";
export function AnalysisCreateAction({
  onCreate,
}: {
  onCreate?: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const locked = useRef(false);
  const create = async () => {
    if (!onCreate || locked.current) return;
    locked.current = true;
    setBusy(true);
    setError("");
    try {
      await onCreate();
    } catch (error) {
      setError(String(error));
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };
  return (
    <div className="analysis-create-action">
      <div>
        <AsyncButton
          className="primary"
          busy={busy}
          busyLabel={t("正在准备…")}
          disabled={!onCreate}
          onClick={() => void create()}
        >
          {t("创建新项目")}
          <ArrowUpRight size={17} aria-hidden="true" />
        </AsyncButton>
        <span>{t("带入完整拆解，可用于多个独立项目")}</span>
      </div>
      <ErrorNotice error={error} />
    </div>
  );
}
