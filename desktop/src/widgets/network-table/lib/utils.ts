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
