// Base64 변환 Web Worker
// 메인 스레드를 블로킹하지 않고 이미지 데이터를 Base64로 변환

interface WorkerMessage {
  id: string;
  data: Uint8Array | number[];
  dataType: string;
}

interface WorkerResponse {
  id: string;
  success: boolean;
  dataUrl?: string;
  error?: string;
}

self.onmessage = function (e: MessageEvent<WorkerMessage>) {
  const { data, dataType, id } = e.data;

  try {
    if (dataType === "Image") {
      const base64 = uint8ArrayToBase64(data);
      const mimeType = getImageMimeType(data);
      const dataUrl = `data:${mimeType};base64,${base64}`;

      const response: WorkerResponse = {
        id,
        success: true,
        dataUrl,
      };

      self.postMessage(response);
    } else {
      const response: WorkerResponse = {
        id,
        success: false,
        error: "Not an image",
      };

      self.postMessage(response);
    }
  } catch (error) {
    const response: WorkerResponse = {
      id,
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
    };

    self.postMessage(response);
  }
};

/**
 * Uint8Array를 Base64 문자열로 변환 (청크 단위 처리)
 */
function uint8ArrayToBase64(data: Uint8Array | number[]): string {
  if (!data || data.length === 0) {
    return "";
  }

  const uint8Array = data instanceof Uint8Array ? data : new Uint8Array(data);

  // 청크 단위로 바이너리 문자열을 구성(대용량 입력의 String.fromCharCode 스택 오버플로 방지).
  // 단, btoa는 전체 문자열에 "한 번만" 호출해야 한다. 청크마다 btoa를 호출하면 각 청크가
  // 4의 배수로 '=' 패딩되어, 청크 크기(8192)가 3의 배수가 아닌 경우 결과 base64가 손상된다.
  const chunkSize = 8192; // 8KB 청크
  const binaryParts: string[] = [];

  for (let i = 0; i < uint8Array.length; i += chunkSize) {
    const chunk = uint8Array.slice(i, i + chunkSize);
    binaryParts.push(Array.from(chunk, (byte) => String.fromCharCode(byte)).join(""));
  }

  return btoa(binaryParts.join(""));
}

/**
 * 이미지 데이터의 MIME 타입 결정
 */
function getImageMimeType(data: Uint8Array | number[]): string {
  const uint8Array = data instanceof Uint8Array ? data : new Uint8Array(data);

  if (uint8Array.length >= 2) {
    // PNG 시그니처
    if (
      uint8Array.length >= 8 &&
      uint8Array[0] === 0x89 &&
      uint8Array[1] === 0x50 &&
      uint8Array[2] === 0x4e &&
      uint8Array[3] === 0x47
    ) {
      return "image/png";
    }
    // JPEG 시그니처
    if (uint8Array[0] === 0xff && uint8Array[1] === 0xd8) {
      return "image/jpeg";
    }
    // GIF 시그니처
    if (
      uint8Array.length >= 6 &&
      ((uint8Array[0] === 0x47 &&
        uint8Array[1] === 0x49 &&
        uint8Array[2] === 0x46 &&
        uint8Array[3] === 0x38 &&
        uint8Array[4] === 0x37 &&
        uint8Array[5] === 0x61) ||
        (uint8Array[0] === 0x47 &&
          uint8Array[1] === 0x49 &&
          uint8Array[2] === 0x46 &&
          uint8Array[3] === 0x38 &&
          uint8Array[4] === 0x39 &&
          uint8Array[5] === 0x61))
    ) {
      return "image/gif";
    }
    // WebP 시그니처
    if (
      uint8Array.length >= 12 &&
      uint8Array[0] === 0x52 &&
      uint8Array[1] === 0x49 &&
      uint8Array[2] === 0x46 &&
      uint8Array[3] === 0x46 &&
      uint8Array[8] === 0x57 &&
      uint8Array[9] === 0x45 &&
      uint8Array[10] === 0x42 &&
      uint8Array[11] === 0x50
    ) {
      return "image/webp";
    }
    // SVG 시그니처
    if (
      uint8Array.length >= 4 &&
      uint8Array[0] === 0x3c &&
      uint8Array[1] === 0x73 &&
      uint8Array[2] === 0x76 &&
      uint8Array[3] === 0x67
    ) {
      return "image/svg+xml";
    }
  }

  return "image/png"; // 기본값
}
