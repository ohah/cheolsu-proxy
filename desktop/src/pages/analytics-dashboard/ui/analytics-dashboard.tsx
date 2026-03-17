import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { BarChart3, RefreshCw, Activity, AlertTriangle, Clock, Gauge } from "lucide-react";

import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  Badge,
} from "@/shared/ui";
import { useTransactionData } from "@/shared/hooks/use-transaction-selectors";

import {
  computeSummary,
  analyzePerformance,
  analyzeErrors,
  analyzeEndpoints,
  detectDuplicateRequests,
  detectNPlusOne,
  analyzeMisc,
  type SummaryMetrics,
  type PerformanceReport,
  type ErrorReport,
  type EndpointStat,
  type EndpointSortKey,
  type DuplicateRequestGroup,
  type NPlusOnePattern,
  type MiscReport,
} from "../lib";

import { PerformanceTab } from "./performance-tab";
import { ErrorTab } from "./error-tab";
import { EndpointTab } from "./endpoint-tab";
import { DuplicateTab } from "./duplicate-tab";
import { NPlusOneTab } from "./nplus-one-tab";
import { MiscTab } from "./misc-tab";

interface AnalysisState {
  summary: SummaryMetrics | null;
  performance: PerformanceReport | null;
  errors: ErrorReport | null;
  endpoints: EndpointStat[] | null;
  duplicates: DuplicateRequestGroup[] | null;
  nPlusOne: NPlusOnePattern[] | null;
  misc: MiscReport | null;
}

export const AnalyticsDashboard = () => {
  const { t } = useLingui();
  const { transactions } = useTransactionData();
  const [analysis, setAnalysis] = useState<AnalysisState>({
    summary: null,
    performance: null,
    errors: null,
    endpoints: null,
    duplicates: null,
    nPlusOne: null,
    misc: null,
  });
  const [endpointSort, setEndpointSort] = useState<EndpointSortKey>("count");
  const [analyzing, setAnalyzing] = useState(false);

  const runAnalysis = useCallback(() => {
    setAnalyzing(true);
    // Use requestAnimationFrame to avoid blocking UI
    requestAnimationFrame(() => {
      const summary = computeSummary(transactions);
      const performance = analyzePerformance(transactions);
      const errors = analyzeErrors(transactions);
      const endpoints = analyzeEndpoints(transactions, endpointSort);
      const duplicates = detectDuplicateRequests(transactions);
      const nPlusOne = detectNPlusOne(transactions);
      const misc = analyzeMisc(transactions);

      setAnalysis({
        summary,
        performance,
        errors,
        endpoints,
        duplicates,
        nPlusOne,
        misc,
      });
      setAnalyzing(false);
    });
  }, [transactions, endpointSort]);

  const handleSortChange = useCallback(
    (sort: EndpointSortKey) => {
      setEndpointSort(sort);
      if (analysis.endpoints) {
        const endpoints = analyzeEndpoints(transactions, sort);
        setAnalysis((prev) => ({ ...prev, endpoints }));
      }
    },
    [transactions, analysis.endpoints],
  );

  const hasData = analysis.summary !== null;

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b bg-background">
        <div className="flex items-center gap-2">
          <BarChart3 className="w-5 h-5 text-muted-foreground" />
          <h1 className="text-lg font-semibold">
            <Trans>Traffic Analysis</Trans>
          </h1>
          {hasData && (
            <Badge variant="secondary" className="ml-2">
              {analysis.summary!.totalRequests.toLocaleString()} <Trans>requests</Trans>
            </Badge>
          )}
        </div>
        <Button size="sm" onClick={runAnalysis} disabled={analyzing || transactions.length === 0}>
          <RefreshCw className={`w-4 h-4 mr-1 ${analyzing ? "animate-spin" : ""}`} />
          <Trans>Run Analysis</Trans>
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {!hasData && (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-3">
            <BarChart3 className="w-12 h-12 opacity-40" />
            <p className="text-sm">
              {transactions.length === 0 ? (
                <Trans>No traffic data available. Start the proxy to capture requests.</Trans>
              ) : (
                <Trans>
                  Click "Run Analysis" to analyze {transactions.length} captured requests.
                </Trans>
              )}
            </p>
          </div>
        )}

        {hasData && analysis.summary && (
          <>
            {/* Summary Cards */}
            <div className="grid grid-cols-4 gap-3">
              <SummaryCard
                icon={<Activity className="w-4 h-4 text-blue-500" />}
                label={t`Total Requests`}
                value={analysis.summary.totalRequests.toLocaleString()}
              />
              <SummaryCard
                icon={<AlertTriangle className="w-4 h-4 text-red-500" />}
                label={t`Error Rate`}
                value={`${analysis.summary.errorRate.toFixed(1)}%`}
                alert={analysis.summary.errorRate > 5}
              />
              <SummaryCard
                icon={<Clock className="w-4 h-4 text-amber-500" />}
                label={t`Avg Response Time`}
                value={`${analysis.summary.avgResponseTime.toFixed(0)} ms`}
              />
              <SummaryCard
                icon={<Gauge className="w-4 h-4 text-purple-500" />}
                label={t`P95 Response Time`}
                value={`${analysis.summary.p95ResponseTime.toFixed(0)} ms`}
              />
            </div>

            {/* Tabs */}
            <Tabs defaultValue="performance">
              <TabsList>
                <TabsTrigger value="performance">
                  <Trans>Performance</Trans>
                </TabsTrigger>
                <TabsTrigger value="errors">
                  <Trans>Errors</Trans>
                </TabsTrigger>
                <TabsTrigger value="endpoints">
                  <Trans>Endpoints</Trans>
                </TabsTrigger>
                <TabsTrigger value="duplicates">
                  <Trans>Duplicate Requests</Trans>
                </TabsTrigger>
                <TabsTrigger value="nplus1">
                  <Trans>N+1</Trans>
                </TabsTrigger>
                <TabsTrigger value="misc">
                  <Trans>Misc</Trans>
                </TabsTrigger>
              </TabsList>

              <TabsContent value="performance">
                {analysis.performance && <PerformanceTab report={analysis.performance} />}
              </TabsContent>

              <TabsContent value="errors">
                {analysis.errors && <ErrorTab report={analysis.errors} />}
              </TabsContent>

              <TabsContent value="endpoints">
                {analysis.endpoints && (
                  <EndpointTab
                    endpoints={analysis.endpoints}
                    sortBy={endpointSort}
                    onSortChange={handleSortChange}
                  />
                )}
              </TabsContent>

              <TabsContent value="duplicates">
                {analysis.duplicates && <DuplicateTab groups={analysis.duplicates} />}
              </TabsContent>

              <TabsContent value="nplus1">
                {analysis.nPlusOne && <NPlusOneTab patterns={analysis.nPlusOne} />}
              </TabsContent>

              <TabsContent value="misc">
                {analysis.misc && <MiscTab report={analysis.misc} />}
              </TabsContent>
            </Tabs>
          </>
        )}
      </div>
    </div>
  );
};

function SummaryCard({
  icon,
  label,
  value,
  alert,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  alert?: boolean;
}) {
  return (
    <Card className={`py-3 ${alert ? "border-red-300 dark:border-red-800" : ""}`}>
      <CardHeader className="pb-1 pt-0 px-4">
        <CardTitle className="text-xs font-medium text-muted-foreground flex items-center gap-1.5">
          {icon}
          {label}
        </CardTitle>
      </CardHeader>
      <CardContent className="px-4 pt-0">
        <div className={`text-xl font-bold ${alert ? "text-red-600 dark:text-red-400" : ""}`}>
          {value}
        </div>
      </CardContent>
    </Card>
  );
}
