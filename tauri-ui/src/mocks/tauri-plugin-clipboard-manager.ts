export async function writeText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    console.warn("[Mock] clipboard writeText fallback:", text.slice(0, 50));
  }
}

export async function writeImage(_image: Uint8Array): Promise<void> {
  console.warn("[Mock] clipboard writeImage: not supported in browser mock");
}

export async function readText(): Promise<string> {
  try {
    return await navigator.clipboard.readText();
  } catch {
    return "";
  }
}
