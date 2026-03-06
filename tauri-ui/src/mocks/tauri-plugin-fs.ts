export const BaseDirectory = {
  Audio: 1,
  Cache: 2,
  Config: 3,
  Data: 4,
  Document: 6,
  Download: 7,
  Home: 18,
  Desktop: 5,
  Picture: 8,
  Public: 9,
  Runtime: 10,
  Temp: 11,
  Video: 12,
  Resource: 13,
  AppConfig: 14,
  AppData: 15,
  AppLocalData: 16,
  AppCache: 17,
  AppLog: 19,
} as const;

export async function readFile(
  _path: string,
  _options?: { baseDir?: number },
): Promise<Uint8Array> {
  return new Uint8Array(0);
}

export async function writeFile(
  _path: string,
  _data: Uint8Array,
  _options?: { baseDir?: number },
): Promise<void> {}

export async function exists(_path: string, _options?: { baseDir?: number }): Promise<boolean> {
  return false;
}
