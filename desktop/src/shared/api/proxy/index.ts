export {
  type ProxyStartResult,
  startProxyV2,
  stopProxyV2,
  getProxyV2Status,
  cleanOldProxyCache,
  type ProxyAuthConfig,
  updateProxyAuth,
  type ConnectionStrategy,
  updateConnectionStrategy,
  updateQuickSettings,
  type ThrottleConfig,
  updateThrottle,
} from "./control";

export {
  type ReplayRequestParams,
  type ReplayResponse,
  type SequenceReplayResult,
  replayRequest,
  replaySequence,
  type AdvancedRepeatParams,
  type AdvancedRepeatProgress,
  type AdvancedRepeatResult,
  advancedRepeat,
  type ServerReplayEntry,
  updateServerReplay,
  type WsInjectParams,
  wsInjectMessage,
} from "./replay";

export {
  type LoadSessionResult,
  saveSession,
  loadSession,
  importHarFile,
  autosaveSession,
  autoloadSession,
} from "./session";

export {
  getCaCertPath,
  checkCaInstalled,
  installCaCert,
  uninstallCaCert,
  type CertDownloadInfo,
  getCertDownloadInfo,
  type CertificateInfo,
  parseCertificateInfo,
  type DomainClientCertConfig,
  type ClientCertConfig,
  updateClientCertificate,
  type RequestClientCertConfig,
  updateRequestClientCert,
  importCustomCa,
  importCustomCaPkcs12,
  removeCustomCa,
  getCustomCaStatus,
} from "./certificate";

export {
  updateInterceptRules,
  updateBreakpointRules,
  resolveBreakpoint,
  type HostMapping,
  updateHostMappings,
  type ReverseProxyRule,
  updateReverseProxyRules,
  type SslProxyingMode,
  type SslProxyingEntry,
  updateSslProxyingList,
  updateDefaultPassthroughDomains,
  getDefaultPassthroughDomains,
  updateNeverPassthroughDomains,
  getNeverPassthroughDomains,
} from "./rules";

export {
  type DiffTransactionData,
  type DiffTransactionPair,
  type HeaderDiff,
  type DiffLine,
  type JsonDiffEntry,
  type BodyDiff,
  type TransactionPartDiff,
  type TrafficDiff,
  diffTransactions,
  diffTransactionPairs,
} from "./diff";

export { loadScript, unloadScript } from "./script";

export { getMcpServerPath, installCli, uninstallCli, checkCliInstalled } from "./tools";
