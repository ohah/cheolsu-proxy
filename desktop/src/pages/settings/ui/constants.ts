// Throttle preset configurations shared between UI and save logic
export const THROTTLE_PRESETS = [
  { value: "none", label: "None", config: null },
  {
    value: "gprs",
    label: "GPRS (50 KB/s)",
    config: { enabled: true, download_rate: 50 * 1024, upload_rate: 20 * 1024, latency_ms: 500 },
  },
  {
    value: "slow3g",
    label: "Slow 3G (500 KB/s)",
    config: { enabled: true, download_rate: 500 * 1024, upload_rate: 500 * 1024, latency_ms: 400 },
  },
  {
    value: "fast3g",
    label: "Fast 3G (1.6 MB/s)",
    config: {
      enabled: true,
      download_rate: 1_600 * 1024,
      upload_rate: 768 * 1024,
      latency_ms: 150,
    },
  },
  {
    value: "lte",
    label: "4G/LTE (4 MB/s)",
    config: {
      enabled: true,
      download_rate: 4 * 1024 * 1024,
      upload_rate: 3 * 1024 * 1024,
      latency_ms: 50,
    },
  },
  {
    value: "wifi",
    label: "WiFi (30 MB/s)",
    config: {
      enabled: true,
      download_rate: 30 * 1024 * 1024,
      upload_rate: 15 * 1024 * 1024,
      latency_ms: 2,
    },
  },
  { value: "custom", label: "Custom", config: null },
] as const;
