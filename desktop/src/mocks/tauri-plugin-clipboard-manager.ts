export async function writeText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // browser clipboard API not available
  }
}

export async function writeImage(_image: Uint8Array): Promise<void> {
  // not supported in browser mock
}

export async function readText(): Promise<string> {
  try {
    return await navigator.clipboard.readText();
  } catch {
    return "";
  }
}
