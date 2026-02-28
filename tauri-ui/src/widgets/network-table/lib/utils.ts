export const formatBytes = (bytes: number): string => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${Number.parseFloat((bytes / k ** i).toFixed(1))} ${sizes[i]}`;
};

export const getAuthority = (uri: string) => {
  try {
    // CONNECT 요청의 경우 host:port 형식이므로 직접 반환
    if (uri.includes(":") && !uri.startsWith("http")) {
      return uri;
    }

    const url = new URL(uri);
    return `${url.hostname}${url.port ? `:${url.port}` : ""}`;
  } catch (e) {
    // CONNECT 요청의 경우 host:port 형식이므로 그대로 반환
    if (uri.includes(":") && !uri.startsWith("http")) {
      return uri;
    }
    return uri.split("/")[0] || uri;
  }
};
