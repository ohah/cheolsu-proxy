import { AlertTriangle, CheckCircle, XCircle } from "lucide-react";
import { Trans } from "@lingui/react/macro";

import type { ContractValidationResult, ContractViolation } from "@/entities/proxy";

interface TransactionValidationProps {
  validations?: ContractValidationResult[];
}

function ViolationIcon({ severity }: { severity: string }) {
  if (severity === "Error") {
    return <XCircle className="h-4 w-4 text-destructive shrink-0" />;
  }
  return <AlertTriangle className="h-4 w-4 text-yellow-500 shrink-0" />;
}

function ViolationItem({ violation }: { violation: ContractViolation }) {
  return (
    <div className="flex items-start gap-2 py-2 px-3 border-b last:border-b-0">
      <ViolationIcon severity={violation.severity} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono">{violation.path}</code>
          <span className="text-xs text-muted-foreground">{violation.violation_type}</span>
        </div>
        <p className="text-sm mt-1">{violation.message}</p>
        {(violation.expected || violation.actual) && (
          <div className="mt-1 text-xs text-muted-foreground space-y-0.5">
            {violation.expected && (
              <div>
                <Trans>Expected</Trans>:{" "}
                <code className="bg-muted px-1 rounded">{violation.expected}</code>
              </div>
            )}
            {violation.actual && (
              <div>
                <Trans>Actual</Trans>:{" "}
                <code className="bg-muted px-1 rounded">{violation.actual}</code>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export function TransactionValidation({ validations }: TransactionValidationProps) {
  if (!validations || validations.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <CheckCircle className="h-8 w-8 mb-2 text-green-500" />
        <p className="text-sm">
          <Trans>No contract validation results</Trans>
        </p>
        <p className="text-xs mt-1">
          <Trans>Load an OpenAPI spec to enable contract testing</Trans>
        </p>
      </div>
    );
  }

  const allViolations = validations.flatMap((r) => r.violations);
  const errorCount = allViolations.filter((v) => v.severity === "Error").length;
  const warningCount = allViolations.filter((v) => v.severity === "Warning").length;

  if (allViolations.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <CheckCircle className="h-8 w-8 mb-2 text-green-500" />
        <p className="text-sm">
          <Trans>All contract validations passed</Trans>
        </p>
        {validations.map((r) => (
          <p key={r.spec_id} className="text-xs mt-1">
            {r.matched_path && (
              <code className="bg-muted px-1 rounded">
                {r.matched_operation} {r.matched_path}
              </code>
            )}
          </p>
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4 text-sm">
        {errorCount > 0 && (
          <span className="flex items-center gap-1 text-destructive">
            <XCircle className="h-4 w-4" />
            {errorCount} <Trans>errors</Trans>
          </span>
        )}
        {warningCount > 0 && (
          <span className="flex items-center gap-1 text-yellow-500">
            <AlertTriangle className="h-4 w-4" />
            {warningCount} <Trans>warnings</Trans>
          </span>
        )}
      </div>

      {validations.map((result) => (
        <div key={result.spec_id} className="border rounded-md">
          {result.matched_path && (
            <div className="px-3 py-2 bg-muted/50 border-b text-xs text-muted-foreground">
              <Trans>Matched</Trans>:{" "}
              <code>
                {result.matched_operation} {result.matched_path}
              </code>
            </div>
          )}
          {result.violations.map((violation, i) => (
            <ViolationItem key={i} violation={violation} />
          ))}
        </div>
      ))}
    </div>
  );
}
