import { useContext, useEffect, useRef, useState } from "react";
import { ArrowsOut, Broom, FolderOpen, Palette } from "@phosphor-icons/react";
import { bridge, native } from "../../bridge";
import type { Asset } from "../../model";
import { t, useLanguage } from "../../i18n";
import { ActionButton } from "../../ui/ActionButton";
import { AsyncButton } from "../../ui/AsyncState";
import { DockPanel } from "../../ui/DockPanel";
import { StudioSelect } from "../../ui/StudioSelect";
import { ErrorNotice } from "../../errors/ErrorNotice";
import { ImageReviewContext, type ImageOperation } from "./ImageReviewContext";
import { ImageReview } from "./ImageReview";
import "./image-processing.css";

const tools = {
  denoise: { label: "降噪", icon: Broom },
  upscale: { label: "超分辨率", icon: ArrowsOut },
  lut: { label: "LUT", icon: Palette },
};
type Tool = keyof typeof tools;
type Runtime = { directory: string; device: "metal" | "cpu" };

export function ImageProcessingPreview({
  asset,
  close,
  onAssetChange,
}: {
  asset: Asset;
  close: () => void;
  onAssetChange?: (asset: Asset) => void;
}) {
  useLanguage();
  const context = useContext(ImageReviewContext);
  const mounted = useRef(true);
  const lutInput = useRef<HTMLInputElement>(null);
  const [tool, setTool] = useState<Tool | null>(null);
  const [scale, setScale] = useState<2 | 4>(2);
  const [lut, setLut] = useState<{ name: string; cube: string }>();
  const [strength, setStrength] = useState(100);
  const [runtime, setRuntime] = useState<Runtime | null>(null);
  const [busy, setBusy] = useState<"process" | "models" | null>(null);
  const [error, setError] = useState<unknown>();
  const [result, setResult] = useState<Asset>();
  const [original, setOriginal] = useState(false);
  const current = result && !original ? result : asset;
  useEffect(
    () => () => {
      mounted.current = false;
    },
    [],
  );
  useEffect(() => {
    if (!native || !context) return;
    let active = true;
    void bridge<Runtime | null>("image_processing_runtime").then(
      (value) => {
        if (active) setRuntime(value);
      },
      (error) => {
        if (active) setError(error);
      },
    );
    return () => {
      active = false;
    };
  }, [context?.projectId]);
  if (!context || !context.hasAsset(asset.id))
    return <ImageReview asset={asset} close={close} />;

  async function configure() {
    setBusy("models");
    setError(undefined);
    try {
      const selected = await bridge<Runtime | null>(
        "configure_image_processing",
        { runtime: null },
      );
      if (selected) setRuntime(selected);
    } catch (error) {
      setError(error);
    } finally {
      setBusy(null);
    }
  }
  async function process() {
    if (!tool || !context || busy) return;
    setBusy("process");
    setError(undefined);
    try {
      const operation: ImageOperation =
        tool === "lut"
          ? { kind: "lut", cube: lut!.cube, strength: strength / 100 }
          : {
              kind: "restore",
              scale: tool === "denoise" ? 1 : scale,
              seed: 42,
              vaeTile: 512,
            };
      const next = await context.process(
        current,
        operation,
        `${current.name} · ${t(tools[tool].label)}`,
      );
      if (!mounted.current) return;
      setResult(next);
      setOriginal(false);
      onAssetChange?.(next);
    } catch (error) {
      setError(error);
      setTool(tool);
    } finally {
      setBusy(null);
    }
  }
  function showOriginal(value: boolean) {
    setOriginal(value);
    onAssetChange?.(value ? asset : result!);
  }
  return (
    <div className="image-processing-preview" aria-busy={!!busy}>
      <div
        className="image-processing-toolbar"
        role="toolbar"
        aria-label={t("图像处理")}
      >
        {(Object.keys(tools) as Tool[]).map((key) => (
          <ActionButton
            key={key}
            icon={tools[key].icon}
            aria-pressed={tool === key}
            disabled={!!busy}
            onClick={() => {
              setTool(tool === key ? null : key);
              setError(undefined);
            }}
          >
            {t(tools[key].label)}
          </ActionButton>
        ))}
        {busy === "process" && <span role="status">{t("处理中…")}</span>}
        {result && (
          <div
            className="image-processing-compare"
            role="group"
            aria-label={t("对比图片")}
          >
            <button
              aria-pressed={original}
              disabled={!!busy}
              onClick={() => showOriginal(true)}
            >
              {t("原图")}
            </button>
            <button
              aria-pressed={!original}
              disabled={!!busy}
              onClick={() => showOriginal(false)}
            >
              {t("处理结果")}
            </button>
          </div>
        )}
      </div>
      <div className="image-processing-body">
        <div className="image-processing-stage">
          <ImageReview key={current.id} asset={current} close={close} />
        </div>
        {tool && (
          <DockPanel
            id="image-processing"
            title={t(tools[tool].label)}
            onClose={() => setTool(null)}
          >
            {tool === "denoise" ? (
              <p>{t("使用修复模型降噪，保持原始尺寸。")}</p>
            ) : tool === "upscale" ? (
              <label>
                {t("放大倍数")}
                <StudioSelect
                  value={String(scale)}
                  label={t("放大倍数")}
                  disabled={!!busy}
                  options={[
                    { value: "2", label: "2×" },
                    { value: "4", label: "4×" },
                  ]}
                  onValueChange={(value) => setScale(Number(value) as 2 | 4)}
                />
                <small>
                  {current.width * scale} × {current.height * scale}
                </small>
              </label>
            ) : (
              <>
                <label>
                  {t("LUT 文件")}
                  <input
                    ref={lutInput}
                    type="file"
                    accept=".cube"
                    disabled={!!busy}
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      if (!file) return;
                      setLut(undefined);
                      setError(undefined);
                      if (file.size > 16_000_000) {
                        setError(t("LUT 文件不能超过 16 MB"));
                        return;
                      }
                      void file.text().then((cube) => {
                        if (lutInput.current?.files?.[0] === file)
                          setLut({ name: file.name, cube });
                      }, setError);
                    }}
                  />
                  {lut && <small>{lut.name}</small>}
                </label>
                <label>
                  {t("LUT 强度")} · {strength}%
                  <input
                    type="range"
                    min={0}
                    max={100}
                    value={strength}
                    disabled={!!busy}
                    aria-label={t("LUT 强度")}
                    onChange={(event) =>
                      setStrength(Number(event.target.value))
                    }
                  />
                </label>
              </>
            )}
            {tool !== "lut" && (
              <div className="image-processing-models">
                <ActionButton
                  icon={FolderOpen}
                  busy={busy === "models"}
                  disabled={!native || !!busy}
                  onClick={() => void configure()}
                >
                  {runtime ? t("更换模型文件夹") : t("选择模型文件夹")}
                </ActionButton>
                {runtime && (
                  <small title={runtime.directory}>{runtime.directory}</small>
                )}
              </div>
            )}
            <AsyncButton
              className="primary"
              busy={busy === "process"}
              busyLabel={t("处理中…")}
              disabled={!native || !!busy || (tool === "lut" ? !lut : !runtime)}
              onClick={() => void process()}
            >
              {tool === "lut"
                ? t("应用 LUT")
                : tool === "denoise"
                  ? t("开始降噪")
                  : t("开始超分辨率")}
            </AsyncButton>
            <p className="subtle">
              {native
                ? t("处理结果自动保存为新素材，原图保留。")
                : t("请在桌面应用中处理图片。")}
            </p>
            <ErrorNotice error={error} />
          </DockPanel>
        )}
      </div>
    </div>
  );
}
