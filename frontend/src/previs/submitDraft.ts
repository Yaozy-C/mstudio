import type { Asset } from "../model";
import type { PrevisImages } from "./usePrevisImages";

/** Move a draft into the submitted request immediately, before model work starts. */
export async function submitPrevisDraft(
  text: string,
  images: Pick<PrevisImages, "items" | "clear" | "restore">,
  setPrompt: (text: string) => void,
  send: (text: string, images: Asset[]) => Promise<boolean>,
  setError: (error: string) => void,
) {
  const submitted = [...images.items];
  setPrompt("");
  images.clear();
  try {
    if (await send(text, submitted)) return;
  } catch (error) {
    setError(String(error));
  }
  // False means the host did not accept the request, rather than a model failure.
  setPrompt(text);
  images.restore(submitted);
}
