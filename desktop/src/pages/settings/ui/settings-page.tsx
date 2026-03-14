import { useState, useEffect, useCallback, useRef } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Button, Badge } from "@/shared/ui";
import {
  GeneralSettings,
  CertificateSettings,
  CliSettings,
  ThrottleSection,
  ConnectionStrategySection,
  UpstreamProxySection,
  ProxyAuthSection,
  ShortcutSection,
  ClientCertificateSection,
  RequestClientCertSection,
  SslProxyingSection,
  ProtoFilesSection,
} from "./components";
import { SettingsFormProvider, useSettingsForm, type SettingsFormValues } from "./settings-form";
import { saveAllSettings } from "./save-settings";

type SettingsCategory = "general" | "certificate" | "network" | "security" | "tools";
const CATEGORIES: SettingsCategory[] = ["general", "certificate", "network", "security", "tools"];

// =============================================================================
// Page
// =============================================================================
export function SettingsPage() {
  return (
    <SettingsFormProvider>
      <SettingsPageInner />
    </SettingsFormProvider>
  );
}

function SettingsPageInner() {
  const { t } = useLingui();
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>("general");
  const form = useSettingsForm();
  const { isDirty, isSubmitting, dirtyFields } = form.formState;
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "error">("idle");
  const [saveErrorMessage, setSaveErrorMessage] = useState<string | null>(null);
  const dirtyFieldsRef = useRef(dirtyFields);
  dirtyFieldsRef.current = dirtyFields;

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<Record<SettingsCategory, HTMLDivElement | null>>({
    general: null,
    certificate: null,
    network: null,
    security: null,
    tools: null,
  });

  // Track which section is visible via IntersectionObserver
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const cat = entry.target.getAttribute("data-category") as SettingsCategory | null;
            if (cat) setActiveCategory(cat);
          }
        }
      },
      { root: container, rootMargin: "-20% 0px -70% 0px", threshold: 0 },
    );

    for (const cat of CATEGORIES) {
      const el = sectionRefs.current[cat];
      if (el) observer.observe(el);
    }

    return () => observer.disconnect();
  }, []);

  const handleSave = useCallback(
    async (data: SettingsFormValues) => {
      setSaveStatus("idle");
      setSaveErrorMessage(null);
      try {
        await saveAllSettings(data, dirtyFieldsRef.current);
        form.reset(data);
        setSaveStatus("saved");
        setTimeout(() => setSaveStatus("idle"), 2000);
      } catch (e) {
        console.error("Settings save failed:", e);
        setSaveStatus("error");
        setSaveErrorMessage(e instanceof Error ? e.message : null);
        setTimeout(() => {
          setSaveStatus("idle");
          setSaveErrorMessage(null);
        }, 3000);
      }
    },
    [form],
  );

  const handleScrollTo = useCallback((cat: SettingsCategory) => {
    const el = sectionRefs.current[cat];
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, []);

  const categoryLabels: Record<SettingsCategory, string> = {
    general: t`General`,
    certificate: t`Certificate`,
    network: t`Network`,
    security: t`Security`,
    tools: t`Tools`,
  };

  return (
    <form onSubmit={form.handleSubmit(handleSave)} className="flex-1 flex h-full overflow-hidden">
      {/* Sidebar */}
      <nav className="w-48 flex-shrink-0 border-r p-4 space-y-1">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            onClick={() => handleScrollTo(cat)}
            className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
              activeCategory === cat
                ? "bg-accent text-accent-foreground font-medium"
                : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
            }`}
          >
            {categoryLabels[cat]}
          </button>
        ))}
      </nav>

      {/* Content */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Sticky header */}
        <div className="flex items-center justify-between px-6 py-4 border-b flex-shrink-0">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Settings</Trans>
            </h1>
            <p className="text-sm text-muted-foreground">
              <Trans>Proxy configuration and preferences</Trans>
            </p>
          </div>
          <div className="flex items-center gap-3">
            {saveStatus === "saved" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Saved</Trans>
              </Badge>
            )}
            {saveStatus === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                {saveErrorMessage ? saveErrorMessage : <Trans>Save failed</Trans>}
              </Badge>
            )}
            {isDirty && (
              <Button type="submit" disabled={isSubmitting} size="sm">
                {isSubmitting ? t`Saving...` : t`Save Changes`}
              </Button>
            )}
          </div>
        </div>

        {/* Scrollable content — all sections visible */}
        <div ref={scrollContainerRef} className="flex-1 overflow-auto p-6 space-y-10">
          <div
            ref={(el) => {
              sectionRefs.current.general = el;
            }}
            data-category="general"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.general}</h2>
            <GeneralSettings />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.certificate = el;
            }}
            data-category="certificate"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.certificate}</h2>
            <CertificateSettings />
            <ClientCertificateSection />
            <RequestClientCertSection />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.network = el;
            }}
            data-category="network"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.network}</h2>
            <ThrottleSection />
            <ConnectionStrategySection />
            <SslProxyingSection />
            <UpstreamProxySection />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.security = el;
            }}
            data-category="security"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.security}</h2>
            <ProxyAuthSection />
          </div>

          <div
            ref={(el) => {
              sectionRefs.current.tools = el;
            }}
            data-category="tools"
            className="space-y-6"
          >
            <h2 className="text-xl font-semibold text-foreground">{categoryLabels.tools}</h2>
            <CliSettings />
            <ShortcutSection />
            <ProtoFilesSection />
          </div>
        </div>
      </div>
    </form>
  );
}
