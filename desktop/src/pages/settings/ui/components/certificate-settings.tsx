import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import {
  checkCaInstalled,
  installCaCert,
  uninstallCaCert,
  getCaCertPath,
  getCertDownloadInfo,
  importCustomCa,
  importCustomCaPkcs12,
  removeCustomCa,
  getCustomCaStatus,
  type CertDownloadInfo,
  type CertificateInfo,
} from "@/shared/api/proxy";
import { useProxyStore } from "@/shared/stores/proxy-store";
import { Button, Badge, Input } from "@/shared/ui";

const CERT_DOWNLOAD_URL = "http://cheolsu.proxy/ssl";
const CERT_DOWNLOAD_PATHS = {
  pem: "/ssl/pem",
  der: "/ssl/der",
  universal: "/ssl/ca.crt",
} as const;

export function CertificateSettings() {
  const { t } = useLingui();
  const proxyPort = useProxyStore((s) => s.port);
  const isProxyConnected = useProxyStore((s) => s.isConnected);

  const [caInstalled, setCaInstalled] = useState(false);
  const [caInstalling, setCaInstalling] = useState(false);
  const [caMessage, setCaMessage] = useState("");
  const [caCertPath, setCaCertPath] = useState("");
  const [certDownloadInfo, setCertDownloadInfo] = useState<CertDownloadInfo | null>(null);
  const [certDownloadLoading, setCertDownloadLoading] = useState(false);
  const [certUrlCopied, setCertUrlCopied] = useState(false);
  const [showIosGuide, setShowIosGuide] = useState(false);
  const [showAndroidGuide, setShowAndroidGuide] = useState(false);

  // CA 인증서 상태 확인
  useEffect(() => {
    checkCaInstalled()
      .then(setCaInstalled)
      .catch(() => setCaInstalled(false));
    getCaCertPath()
      .then(setCaCertPath)
      .catch(() => setCaCertPath(""));
  }, []);

  // 프록시 실행 중일 때 인증서 다운로드 정보 로드
  const loadCertDownloadInfo = useCallback(async () => {
    setCertDownloadLoading(true);
    try {
      const info = await getCertDownloadInfo(proxyPort);
      setCertDownloadInfo(info);
    } catch (e) {
      console.error("인증서 다운로드 정보 로드 실패:", e);
      setCertDownloadInfo(null);
    } finally {
      setCertDownloadLoading(false);
    }
  }, [proxyPort]);

  useEffect(() => {
    if (isProxyConnected) {
      loadCertDownloadInfo();
    } else {
      setCertDownloadInfo(null);
    }
  }, [isProxyConnected, loadCertDownloadInfo]);

  const handleInstallCa = useCallback(async () => {
    setCaInstalling(true);
    setCaMessage("");
    try {
      const msg = await installCaCert();
      setCaMessage(msg);
      setCaInstalled(true);
    } catch (e) {
      setCaMessage(String(e));
    } finally {
      setCaInstalling(false);
    }
  }, []);

  const handleUninstallCa = useCallback(async () => {
    setCaInstalling(true);
    setCaMessage("");
    try {
      const msg = await uninstallCaCert();
      setCaMessage(msg);
      setCaInstalled(false);
    } catch (e) {
      setCaMessage(String(e));
    } finally {
      setCaInstalling(false);
    }
  }, []);

  return (
    <>
      {/* Custom CA Certificate Section */}
      <CustomCaSection />

      {/* CA Certificate Section */}
      <div className="border rounded-lg p-5 space-y-4">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>CA Certificate</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>
              Install the CA certificate to trust HTTPS traffic intercepted by the proxy
            </Trans>
          </p>
        </div>
        {caCertPath && (
          <p className="text-xs text-muted-foreground font-mono break-all">{caCertPath}</p>
        )}
        <div className="flex items-center gap-3">
          <Button onClick={handleInstallCa} disabled={caInstalling}>
            {caInstalling ? t`Installing...` : caInstalled ? t`Reinstall` : t`Install`}
          </Button>
          {caInstalled && (
            <Button variant="outline" onClick={handleUninstallCa} disabled={caInstalling}>
              {t`Uninstall`}
            </Button>
          )}
          {caInstalled && (
            <Badge variant="outline" className="text-green-600 border-green-600">
              <Trans>Trusted</Trans>
            </Badge>
          )}
          {!caInstalled && caCertPath && (
            <Badge variant="outline" className="text-yellow-600 border-yellow-600">
              <Trans>Not Trusted</Trans>
            </Badge>
          )}
          {!caCertPath && (
            <Badge variant="outline" className="text-muted-foreground">
              <Trans>Start proxy first</Trans>
            </Badge>
          )}
        </div>
        {caMessage && <p className="text-xs text-muted-foreground">{caMessage}</p>}
      </div>

      {/* Remote Device Certificate Section */}
      <div className="border rounded-lg p-5 space-y-4">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Remote Device Certificate</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>
              Install the CA certificate on external devices (mobile, tablet) to intercept HTTPS
              traffic
            </Trans>
          </p>
        </div>

        {!isProxyConnected ? (
          <div className="text-sm text-muted-foreground">
            <Badge variant="outline" className="text-yellow-600 border-yellow-600">
              <Trans>Start proxy first</Trans>
            </Badge>
          </div>
        ) : certDownloadLoading ? (
          <div className="text-sm text-muted-foreground">
            <Trans>Loading...</Trans>
          </div>
        ) : certDownloadInfo ? (
          <div className="space-y-4">
            {/* 설치 안내 */}
            <div className="bg-muted/50 rounded-lg p-4 text-sm space-y-2">
              <p className="font-medium">
                <Trans>Setup Instructions:</Trans>
              </p>
              <ol className="list-decimal list-inside space-y-1 text-muted-foreground">
                <li>
                  <Trans>
                    Set Wi-Fi proxy on your device to{" "}
                    <code className="bg-muted px-1 py-0.5 rounded text-xs font-mono">
                      {certDownloadInfo.local_ips[0] || "127.0.0.1"}:{certDownloadInfo.port}
                    </code>
                  </Trans>
                </li>
                <li>
                  <Trans>
                    Open{" "}
                    <code className="bg-muted px-1 py-0.5 rounded text-xs font-mono">
                      {CERT_DOWNLOAD_URL}
                    </code>{" "}
                    in your device browser
                  </Trans>
                </li>
                <li>
                  <Trans>Install and trust the downloaded certificate</Trans>
                </li>
              </ol>
            </div>

            {/* 모바일 인증서 설치 카드 */}
            <div className="border rounded-lg p-4 space-y-3 bg-muted/30">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold">
                  <Trans>Mobile Certificate Install</Trans>
                </h3>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(CERT_DOWNLOAD_URL);
                      setCertUrlCopied(true);
                      setTimeout(() => setCertUrlCopied(false), 2000);
                    } catch {
                      // Fallback for HTTP or unfocused contexts
                      const textarea = document.createElement("textarea");
                      textarea.value = CERT_DOWNLOAD_URL;
                      textarea.style.position = "fixed";
                      textarea.style.opacity = "0";
                      document.body.appendChild(textarea);
                      textarea.select();
                      try {
                        document.execCommand("copy");
                        setCertUrlCopied(true);
                        setTimeout(() => setCertUrlCopied(false), 2000);
                      } catch {
                        console.error("클립보드 복사 실패");
                      }
                      document.body.removeChild(textarea);
                    }
                  }}
                >
                  {certUrlCopied ? t`Copied!` : t`Copy URL`}
                </Button>
              </div>

              <div className="font-mono text-sm bg-muted px-3 py-2 rounded text-center select-all">
                {CERT_DOWNLOAD_URL}
              </div>

              <p className="text-xs text-muted-foreground">
                <Trans>
                  Open this URL in your mobile browser after setting the Wi-Fi proxy. The page
                  automatically detects your device and provides the correct certificate format.
                </Trans>
              </p>

              <div className="text-xs text-muted-foreground space-y-1">
                <p>
                  <code className="bg-muted px-1 py-0.5 rounded">{CERT_DOWNLOAD_PATHS.pem}</code> —
                  PEM {t`format`} (iOS)
                </p>
                <p>
                  <code className="bg-muted px-1 py-0.5 rounded">{CERT_DOWNLOAD_PATHS.der}</code> —
                  DER {t`format`} (Android)
                </p>
                <p>
                  <code className="bg-muted px-1 py-0.5 rounded">
                    {CERT_DOWNLOAD_PATHS.universal}
                  </code>{" "}
                  — {t`Universal format`}
                </p>
              </div>

              {/* iOS 가이드 */}
              <div className="border rounded-md">
                <button
                  type="button"
                  aria-expanded={showIosGuide}
                  aria-controls="ios-guide-content"
                  className="w-full text-left px-3 py-2 text-sm font-medium flex items-center justify-between hover:bg-muted/50 transition-colors"
                  onClick={() => setShowIosGuide(!showIosGuide)}
                >
                  <span>iOS {t`Install Guide`}</span>
                  <span className="text-muted-foreground">{showIosGuide ? "▲" : "▼"}</span>
                </button>
                {showIosGuide && (
                  <div
                    id="ios-guide-content"
                    className="px-3 pb-3 text-xs text-muted-foreground space-y-1"
                  >
                    <ol className="list-decimal list-inside space-y-1">
                      <li>
                        <Trans>
                          Open <strong>{CERT_DOWNLOAD_URL}</strong> in Safari
                        </Trans>
                      </li>
                      <li>
                        <Trans>
                          Tap "Allow" when prompted to download the configuration profile
                        </Trans>
                      </li>
                      <li>
                        <Trans>
                          Go to <strong>Settings → General → VPN & Device Management</strong>
                        </Trans>
                      </li>
                      <li>
                        <Trans>Select the downloaded profile and tap "Install"</Trans>
                      </li>
                      <li>
                        <Trans>
                          Go to{" "}
                          <strong>Settings → General → About → Certificate Trust Settings</strong>
                        </Trans>
                      </li>
                      <li>
                        <Trans>Enable full trust for the Cheolsu Proxy CA certificate</Trans>
                      </li>
                    </ol>
                  </div>
                )}
              </div>

              {/* Android 가이드 */}
              <div className="border rounded-md">
                <button
                  type="button"
                  aria-expanded={showAndroidGuide}
                  aria-controls="android-guide-content"
                  className="w-full text-left px-3 py-2 text-sm font-medium flex items-center justify-between hover:bg-muted/50 transition-colors"
                  onClick={() => setShowAndroidGuide(!showAndroidGuide)}
                >
                  <span>Android {t`Install Guide`}</span>
                  <span className="text-muted-foreground">{showAndroidGuide ? "▲" : "▼"}</span>
                </button>
                {showAndroidGuide && (
                  <div
                    id="android-guide-content"
                    className="px-3 pb-3 text-xs text-muted-foreground space-y-1"
                  >
                    <ol className="list-decimal list-inside space-y-1">
                      <li>
                        <Trans>
                          Open <strong>{CERT_DOWNLOAD_URL}</strong> in Chrome
                        </Trans>
                      </li>
                      <li>
                        <Trans>The DER certificate file will download automatically</Trans>
                      </li>
                      <li>
                        <Trans>
                          Go to <strong>Settings → Security → Encryption & credentials</strong>
                        </Trans>
                      </li>
                      <li>
                        <Trans>Tap "Install a certificate" → "CA certificate"</Trans>
                      </li>
                      <li>
                        <Trans>Select the downloaded certificate file and confirm</Trans>
                      </li>
                    </ol>
                  </div>
                )}
              </div>
            </div>

            {/* QR 코드 + URL 정보 */}
            <div className="flex gap-6 items-start">
              {/* QR 코드 */}
              <div className="flex-shrink-0">
                <div className="bg-white p-2 rounded-lg border">
                  <img
                    src={`data:image/png;base64,${certDownloadInfo.qr_code_base64}`}
                    alt={`Certificate download QR code for ${CERT_DOWNLOAD_URL}`}
                    className="w-32 h-32"
                    style={{ imageRendering: "pixelated" }}
                  />
                </div>
                <p className="text-xs text-muted-foreground mt-1 text-center">
                  <Trans>Scan for proxy info</Trans>
                </p>
              </div>

              {/* URL 정보 */}
              <div className="flex-1 space-y-3">
                <div>
                  <label className="text-xs font-medium text-muted-foreground block mb-1">
                    <Trans>Proxy Address</Trans>
                  </label>
                  {certDownloadInfo.local_ips.map((ip) => (
                    <div key={ip} className="font-mono text-sm bg-muted px-3 py-1.5 rounded mb-1">
                      {ip}:{certDownloadInfo.port}
                    </div>
                  ))}
                </div>
                <div>
                  <label className="text-xs font-medium text-muted-foreground block mb-1">
                    <Trans>Certificate Download URL</Trans>
                  </label>
                  <div className="font-mono text-sm bg-muted px-3 py-1.5 rounded">
                    {CERT_DOWNLOAD_URL}
                  </div>
                </div>
                <Button variant="outline" size="sm" onClick={loadCertDownloadInfo}>
                  {t`Refresh`}
                </Button>
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}

function CustomCaSection() {
  const { t } = useLingui();
  const isProxyConnected = useProxyStore((s) => s.isConnected);

  const [mode, setMode] = useState<"pem" | "pkcs12">("pem");
  const [certPath, setCertPath] = useState("");
  const [keyPath, setKeyPath] = useState("");
  const [p12Path, setP12Path] = useState("");
  const [p12Password, setP12Password] = useState("");
  const [importing, setImporting] = useState(false);
  const [status, setStatus] = useState<"idle" | "imported" | "removed" | "error">("idle");
  const [errorMessage, setErrorMessage] = useState("");
  const [customCaInfo, setCustomCaInfo] = useState<CertificateInfo | null>(null);
  const [loading, setLoading] = useState(true);

  // 초기 로드: 현재 커스텀 CA 상태 확인
  useEffect(() => {
    getCustomCaStatus()
      .then((info) => {
        setCustomCaInfo(info);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  const handleSelectCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer", "der"] }],
    });
    if (selected) {
      setCertPath(selected as string);
      setStatus("idle");
    }
  }, []);

  const handleSelectKey = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Key", extensions: ["pem", "key"] }],
    });
    if (selected) {
      setKeyPath(selected as string);
      setStatus("idle");
    }
  }, []);

  const handleSelectP12 = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "PKCS12", extensions: ["p12", "pfx"] }],
    });
    if (selected) {
      setP12Path(selected as string);
      setStatus("idle");
    }
  }, []);

  const handleImport = useCallback(async () => {
    setImporting(true);
    setStatus("idle");
    setErrorMessage("");
    try {
      let info: CertificateInfo;
      if (mode === "pem") {
        if (!certPath || !keyPath) {
          setErrorMessage(t`Please select both certificate and key files`);
          setStatus("error");
          setImporting(false);
          return;
        }
        info = await importCustomCa(certPath, keyPath);
      } else {
        if (!p12Path) {
          setErrorMessage(t`Please select a PKCS12 file`);
          setStatus("error");
          setImporting(false);
          return;
        }
        info = await importCustomCaPkcs12(p12Path, p12Password);
      }
      setCustomCaInfo(info);
      setStatus("imported");
    } catch (e) {
      setErrorMessage(String(e));
      setStatus("error");
    } finally {
      setImporting(false);
    }
  }, [mode, certPath, keyPath, p12Path, p12Password, t]);

  const handleRemove = useCallback(async () => {
    setImporting(true);
    setStatus("idle");
    setErrorMessage("");
    try {
      await removeCustomCa();
      setCustomCaInfo(null);
      setCertPath("");
      setKeyPath("");
      setP12Path("");
      setP12Password("");
      setStatus("removed");
    } catch (e) {
      setErrorMessage(String(e));
      setStatus("error");
    } finally {
      setImporting(false);
    }
  }, []);

  if (loading) return null;

  return (
    <div className="border rounded-lg p-5 space-y-4">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>Custom CA Certificate</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          <Trans>Use your own CA certificate instead of the auto-generated one</Trans>
        </p>
      </div>

      {/* 현재 커스텀 CA 정보 */}
      {customCaInfo && (
        <div className="bg-muted/50 rounded-lg p-4 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">
              <Trans>Active Custom CA</Trans>
            </h3>
            <Badge variant="outline" className="text-green-600 border-green-600">
              <Trans>Active</Trans>
            </Badge>
          </div>
          <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
            {customCaInfo.subject_cn && (
              <>
                <span className="text-muted-foreground">CN</span>
                <span className="font-mono">{customCaInfo.subject_cn}</span>
              </>
            )}
            {customCaInfo.organization && (
              <>
                <span className="text-muted-foreground">
                  <Trans>Organization</Trans>
                </span>
                <span className="font-mono">{customCaInfo.organization}</span>
              </>
            )}
            <>
              <span className="text-muted-foreground">
                <Trans>Valid Until</Trans>
              </span>
              <span className="font-mono">{customCaInfo.not_after}</span>
            </>
            <>
              <span className="text-muted-foreground">SHA-256</span>
              <span className="font-mono text-xs break-all">{customCaInfo.fingerprint_sha256}</span>
            </>
          </div>
          <div className="flex items-center gap-2 pt-1">
            <Button variant="destructive" size="sm" onClick={handleRemove} disabled={importing}>
              <Trans>Remove Custom CA</Trans>
            </Button>
            {isProxyConnected && (
              <p className="text-xs text-yellow-600">
                <Trans>Restart proxy to apply changes</Trans>
              </p>
            )}
          </div>
        </div>
      )}

      {/* 임포트 폼 */}
      {!customCaInfo && (
        <div className="space-y-4">
          {/* 모드 선택 */}
          <div className="flex gap-2">
            <Button
              variant={mode === "pem" ? "default" : "outline"}
              size="sm"
              onClick={() => setMode("pem")}
            >
              PEM / DER
            </Button>
            <Button
              variant={mode === "pkcs12" ? "default" : "outline"}
              size="sm"
              onClick={() => setMode("pkcs12")}
            >
              PKCS12 (.p12/.pfx)
            </Button>
          </div>

          {mode === "pem" ? (
            <div className="space-y-3">
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>CA Certificate File</Trans>
                </label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    placeholder={t`Select CA certificate (.pem, .crt, .cer, .der)`}
                    value={certPath}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectCert}>
                    {t`Browse`}
                  </Button>
                </div>
              </div>
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>CA Key File</Trans>
                </label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    placeholder={t`Select CA key file (.pem, .key)`}
                    value={keyPath}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectKey}>
                    {t`Browse`}
                  </Button>
                </div>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>PKCS12 File</Trans>
                </label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    placeholder={t`Select PKCS12 file (.p12, .pfx)`}
                    value={p12Path}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectP12}>
                    {t`Browse`}
                  </Button>
                </div>
              </div>
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Password</Trans>
                </label>
                <Input
                  type="password"
                  placeholder={t`PKCS12 password (leave empty if none)`}
                  value={p12Password}
                  onChange={(e) => setP12Password(e.target.value)}
                />
              </div>
            </div>
          )}

          <div className="flex items-center gap-3">
            <Button onClick={handleImport} disabled={importing}>
              {importing ? t`Importing...` : t`Import`}
            </Button>
            {status === "imported" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Imported</Trans>
              </Badge>
            )}
            {status === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                <Trans>Failed</Trans>
              </Badge>
            )}
          </div>

          {status === "error" && errorMessage && (
            <p className="text-xs text-red-600">{errorMessage}</p>
          )}

          <p className="text-xs text-muted-foreground">
            <Trans>
              The certificate must be a CA certificate (BasicConstraints CA=true). After importing,
              restart the proxy to use the new CA. You also need to install this CA on your system
              and devices.
            </Trans>
          </p>
        </div>
      )}

      {status === "removed" && (
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Removed</Trans>
          </Badge>
          {isProxyConnected && (
            <p className="text-xs text-yellow-600">
              <Trans>Restart proxy to use auto-generated CA</Trans>
            </p>
          )}
        </div>
      )}
    </div>
  );
}
