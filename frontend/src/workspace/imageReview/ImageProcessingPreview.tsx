import { useContext, useEffect, useRef, useState } from "react";
import { ArrowsOut, Broom, FolderOpen, Palette } from "@phosphor-icons/react";
import { bridge, native } from "../../bridge";
import {
  isMaterial,
  isStoryAsset,
  isConfirmedStoryAsset,
} from "../assetLibrary";
import type { Asset } from "../../model";
import { t, useLanguage } from "../../i18n";
import { ActionButton } from "../../ui/ActionButton";
import { AsyncButton } from "../../ui/AsyncState";
import { DockPanel } from "../../ui/DockPanel";
import { StudioSelect } from "../../ui/StudioSelect";
import { ErrorNotice } from "../../errors/ErrorNotice";
import {
  ImageReviewContext,
  type ImageOperation,
  type ImageVersion,
} from "./ImageReviewContext";
import { ImageReview } from "./ImageReview";
import "./image-processing.css";
import {
  type ColorCorrection,
  readProcessingOptions,
  saveProcessingOptions,
} from "./processingOptions";

const tools = {
  restore: { label: "AI 修复", icon: Broom },
  upscale: { label: "超分辨率", icon: ArrowsOut },
  lut: { label: "LUT", icon: Palette },
};
type Tool = keyof typeof tools;
type Runtime = { directory: string; device: "metal" | "cpu" };

export function ImageProcessingPreview({
  asset,
  close,
  onPendingVersion,
}: {
  asset: Asset;
  close: () => void;
  onPendingVersion?: (pending: boolean) => void;
}) {
  useLanguage();
  const context = useContext(ImageReviewContext);
  const mounted = useRef(true);
  const lutInput = useRef<HTMLInputElement>(null);
  const [tool, setTool] = useState<Tool | null>(null);
  const [scale, setScale] = useState<2 | 4>(2);
  const [esrganTile, setEsrganTile] = useState<128 | 256 | 512>(256);
  const [options, setOptions] = useState(readProcessingOptions);
  useEffect(() => saveProcessingOptions(options), [options]);
  const [lut, setLut] = useState<{ name: string; cube: string }>();
  const [strength, setStrength] = useState(100);
  const [runtime, setRuntime] = useState<Runtime | null>(null);
  const [busy, setBusy] = useState<"process" | "models" | "reference" | null>(
    null,
  );
  const [error, setError] = useState<unknown>();
  const [versions, setVersions] = useState<ImageVersion[]>([]);
  const [previewVersionId, setPreviewVersionId] = useState<string>();
  const restoreInput = useRef<string | undefined>(undefined);
  const imageId = asset.imageId ?? asset.id;
  const referenceId = asset.imageVersionId ?? `original:${imageId}`;
  const currentVersion =
    versions.find((v) => v.id === (previewVersionId ?? referenceId)) ??
    versions[0];
  const current = currentVersion
    ? {
        ...currentVersion.image,
        id: asset.id,
        name: asset.name,
        inLibrary: asset.inLibrary,
        imageId,
        imageVersionId: currentVersion.id,
      }
    : asset;
  const original = versions[0];
  const pending = !!currentVersion && currentVersion.id !== referenceId;
  const confirmed = isStoryAsset(asset)
    ? isConfirmedStoryAsset(asset)
    : isMaterial(asset);
  const isReference = !pending && confirmed;
  useEffect(() => {
    onPendingVersion?.(pending);
  }, [pending, onPendingVersion]);
  const versionLabel = (v: ImageVersion, index: number) =>
    [
      v.operation
        ? `V${index} · ${t(v.operation.kind === "restore" ? "AI 修复" : v.operation.kind === "upscale" ? "超分辨率" : "LUT")}${v.operation.kind === "restore" && v.operation.denoise !== undefined ? ` · ${v.operation.denoise.toFixed(2)}` : ""}`
        : t("原图"),
      v.id === referenceId && confirmed ? t("当前参考") : "",
    ]
      .filter(Boolean)
      .join(" · ");
  useEffect(() => {
    if (!native || !context?.hasAsset(asset.id)) return;
    let active = true;
    void context.versions(imageId).then(
      (items) => {
        if (!active) return;
        setVersions(items);
        restoreInput.current ??= referenceId;
      },
      (error) => {
        if (active) setError(error);
      },
    );
    return () => {
      active = false;
    };
  }, [context?.projectId, imageId, asset.imageVersionCount, referenceId]);
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
          : tool === "upscale"
            ? { kind: "upscale", scale, tileSize: esrganTile }
            : {
                kind: "restore",
                scale: 1,
                seed: 42,
                denoise: options.denoise,
                vaeTile: options.vaeTile,
                quantize8: options.quantize8,
                colorCorrection: options.colorCorrection,
              };
      const next = await context.process(
        asset,
        operation,
        `${asset.name} · ${t(tools[tool].label)}`,
        tool === "restore" ? restoreInput.current : currentVersion?.id,
      );
      if (!mounted.current) return;
      setVersions(await context.versions(imageId));
      setPreviewVersionId(next.id);
    } catch (error) {
      setError(error);
      setTool(tool);
    } finally {
      setBusy(null);
    }
  }
  async function selectReference() {
    if (!context || !currentVersion || busy) return;
    setBusy("reference");
    setError(undefined);
    try {
      if (isReference) await context.clearReference(asset.id);
      else await context.selectReference(asset.id, currentVersion.id);
      if (!mounted.current) return;
      setVersions(await context.versions(imageId));
    } catch (error) {
      setError(error);
    } finally {
      setBusy(null);
    }
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
        {currentVersion && (
          <div className="image-version-controls">
            {versions.length > 1 && (
              <StudioSelect
                value={currentVersion?.id ?? ""}
                label={t("图片版本")}
                disabled={!!busy}
                options={versions.map((v, i) => ({
                  value: v.id,
                  label: versionLabel(v, i),
                }))}
                onValueChange={(id) => {
                  setPreviewVersionId(id);
                  restoreInput.current = id;
                  setError(undefined);
                }}
              />
            )}
            <AsyncButton
              busy={busy === "reference"}
              disabled={!!busy}
              aria-pressed={isReference}
              aria-label={isReference ? t("取消参考") : t("设为参考")}
              onClick={() => void selectReference()}
            >
              {isReference ? t("当前参考") : t("设为参考")}
            </AsyncButton>
          </div>
        )}
      </div>
      {!tool && error != null && <ErrorNotice error={error} />}
      <div className="image-processing-body">
        <div className="image-processing-stage">
          <ImageReview
            key={`${current.id}:${current.imageVersionId}:${current.path}`}
            asset={current}
            comparison={currentVersion?.operation ? original?.image : undefined}
            readOnly={pending}
            close={close}
          />
        </div>
        {tool && (
          <DockPanel
            id="image-processing"
            title={t(tools[tool].label)}
            onClose={() => setTool(null)}
          >
            {currentVersion && (
              <small>
                {t("处理输入")}：
                {versionLabel(
                  versions.find(
                    (v) =>
                      v.id ===
                      (tool === "restore"
                        ? restoreInput.current
                        : currentVersion.id),
                  ) ?? currentVersion,
                  versions.findIndex(
                    (v) =>
                      v.id ===
                      (tool === "restore"
                        ? restoreInput.current
                        : currentVersion.id),
                  ),
                )}
              </small>
            )}
            {tool === "restore" ? (
              <p>{t("使用 SeedVR2 3B 重建细节，保持原始尺寸。")}</p>
            ) : tool === "upscale" ? (
              <>
                <p>
                  {t(
                    "使用 Real-ESRGAN 原生 2× / 4× 模型进行超分辨率，不经过 SeedVR2。",
                  )}
                </p>
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
                <label>
                  {t("ESRGAN 分块大小")}
                  <StudioSelect
                    value={String(esrganTile)}
                    label={t("ESRGAN 分块大小")}
                    disabled={!!busy}
                    options={[128, 256, 512].map((n) => ({
                      value: String(n),
                      label: String(n),
                    }))}
                    onValueChange={(value) =>
                      setEsrganTile(Number(value) as 128 | 256 | 512)
                    }
                  />
                </label>
                <p>{t("超分会重建图像细节；不会额外执行降噪或 AI 修复。")}</p>
              </>
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
            {tool === "restore" && (
              <>
                <p>
                  {t("AI 修复可能改变脸部、纹理和边缘细节，不等同于纯降噪。")}
                </p>
                <label>
                  {t("Denoise（重绘强度）")} · {options.denoise.toFixed(2)}
                  <input
                    type="range"
                    min={0}
                    max={1}
                    step={0.05}
                    value={options.denoise}
                    disabled={!!busy}
                    aria-label={t("Denoise（重绘强度）")}
                    onChange={(event) =>
                      setOptions((old) => ({
                        ...old,
                        denoise: Number(event.target.value),
                      }))
                    }
                  />
                  <small>
                    {t("0 保留原图；较低值减少重绘，1 为完整重建。")}{" "}
                    {t("当前为单步采样，部分数值效果相同，例如 0.6 与 1.0。")}
                  </small>
                  {versions.length > 1 && (
                    <small>
                      {t("调整修复参数后，将从选定的输入版本重新处理。")}
                    </small>
                  )}
                </label>
                <label>
                  {t("校色方式")}
                  <StudioSelect
                    value={options.colorCorrection}
                    label={t("校色方式")}
                    disabled={!!busy}
                    options={[
                      { value: "wavelet", label: "Wavelet" },
                      { value: "lab", label: "LAB" },
                      { value: "none", label: t("关闭校色") },
                    ]}
                    onValueChange={(value) =>
                      setOptions((old) => ({
                        ...old,
                        colorCorrection: value as ColorCorrection,
                      }))
                    }
                  />
                  <small>
                    {t(
                      "使处理结果的颜色和明暗接近原图；不会还原模型改变的五官或细节。",
                    )}
                  </small>
                </label>
                <label>
                  {t("VAE 分块大小")}
                  <StudioSelect
                    value={String(options.vaeTile)}
                    label={t("VAE 分块大小")}
                    disabled={!!busy}
                    options={[256, 384, 512].map((n) => ({
                      value: String(n),
                      label: String(n),
                    }))}
                    onValueChange={(value) =>
                      setOptions((old) => ({
                        ...old,
                        vaeTile: Number(value) as 256 | 384 | 512,
                      }))
                    }
                  />
                  <small>
                    {t(
                      "较小分块可降低编解码内存，处理更慢；不会缩小整图推理。",
                    )}
                  </small>
                </label>
                <label>
                  {t("DiT 权重量化")}
                  <StudioSelect
                    value={options.quantize8 ? "q8" : "original"}
                    label={t("DiT 权重量化")}
                    disabled={!!busy}
                    options={[
                      { value: "original", label: t("原精度") },
                      { value: "q8", label: t("Q8（实验）") },
                    ]}
                    onValueChange={(value) =>
                      setOptions((old) => ({
                        ...old,
                        quantize8: value === "q8",
                      }))
                    }
                  />
                  {options.quantize8 && (
                    <small>
                      {t(
                        "Q8 可减少部分权重内存，但可能改变画质；量化时仍需加载原权重。",
                      )}
                    </small>
                  )}
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
                : tool === "restore"
                  ? t("开始 AI 修复")
                  : t("开始超分辨率")}
            </AsyncButton>
            <p className="subtle">
              {native
                ? t("处理结果只保存为图片版本，不会新增资产。")
                : t("请在桌面应用中处理图片。")}
            </p>
            <ErrorNotice error={error} />
          </DockPanel>
        )}
      </div>
    </div>
  );
}
