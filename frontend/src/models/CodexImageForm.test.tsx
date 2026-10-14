import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { CodexImageForm } from "./CodexImageForm";
import { catalogMediaModel } from "./catalogMediaModel";
import { modelLibrary } from "./catalogSpecs";

test("Codex configuration exposes the selected model and a save action", () => {
  for (const id of [
    "gpt-image-2.5-sunburst",
    "gpt-image-2.5-flare",
    "codex-image",
  ]) {
    const initial = catalogMediaModel(
      modelLibrary.find((s) => s.id === "codex-image")!,
      "image",
    );
    initial.connectionId = "codex-service";
    initial.params.model = id;
    const html = renderToStaticMarkup(
      <Theme>
        <CodexImageForm
          initial={initial}
          save={async () => {}}
          cancel={() => {}}
        />
      </Theme>,
    );
    expect(html).toContain(
      `value="${id === "codex-image" ? "gpt-image-2" : id}"`,
    );
    expect(html).toContain('type="submit"');
    expect(html).toContain("模型 ID");
  }
});
