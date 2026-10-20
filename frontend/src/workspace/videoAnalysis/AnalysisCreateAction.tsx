import { useRef, useState } from "react";
import { ArrowUpRight } from "@phosphor-icons/react";
import { t } from "../../i18n";
import { AsyncButton } from "../../ui/AsyncState";
import { ErrorNotice } from "../../errors/ErrorNotice";
export function AnalysisCreateAction({
  onCreate,
  continuing = false,
}: {
  onCreate?: () => Promise<void>;
  continuing?: boolean;
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
          {continuing ? t("继续创作") : t("去创作")}
          <ArrowUpRight size={17} aria-hidden="true" />
        </AsyncButton>
        <span>
          {continuing
            ? t("回到项目，接着完成这条视频")
            : t("自动创建项目，带入完整拆解")}
        </span>
      </div>
      <ErrorNotice error={error} />
    </div>
  );
}
