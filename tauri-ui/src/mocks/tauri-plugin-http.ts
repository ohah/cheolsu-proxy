export async function fetch(url: string | URL | Request, options?: RequestInit): Promise<Response> {
  return globalThis.fetch(url, options);
}
