import { useCallback, useEffect, useRef, useState } from "react";
import { Copy, RotateCcw, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import type { HistoryEntry } from "@/bindings";
import { copyToClipboard } from "./clipboard";
import {
  formatUsd,
  historyApi,
  parseUsage,
  summarizeCalls,
  type ProcessingRun,
  type ProviderCall,
  type RequestContents,
  type RunDetail,
} from "./historyApi";

type Tab = "overview" | "requests" | "calls";
const tabs: Tab[] = ["overview", "requests", "calls"];

export const errorNames: Record<string, string> = {
  rate_limit: "rateLimit",
  authentication: "authentication",
  quota: "quota",
  timeout: "timeout",
  invalid_response: "invalidResponse",
  missing_model: "missingModel",
  blocked_input_changed: "inputChanged",
  blocked_input_recheck: "inputRecheck",
  http_error: "httpError",
  provider_unavailable: "providerUnavailable",
  transport_error: "transportError",
};

export const textButtonClass =
  "inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs text-logo-primary hover:bg-logo-primary/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-logo-primary";

function Metric({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs text-text/55">{label}</dt>
      <dd className="break-words text-sm text-text">{value}</dd>
    </div>
  );
}

function duration(ms: number | null, notReported: string): string {
  return ms === null ? notReported : `${(ms / 1000).toFixed(1)} s`;
}

function usageValue(
  value: number | string | undefined,
  notReported: string,
): string {
  return typeof value === "number" ? value.toLocaleString() : notReported;
}

export function HistoryInspector({
  entry,
  onClose,
  refreshKey,
  archiveRevision,
  metadataWarning,
}: {
  entry: HistoryEntry;
  onClose: () => void;
  refreshKey: string;
  archiveRevision: number;
  metadataWarning: boolean;
}) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("overview");
  const [runs, setRuns] = useState<ProcessingRun[]>([]);
  const [runId, setRunId] = useState<number | null>(null);
  const [detail, setDetail] = useState<RunDetail | null>(null);
  const [detailArchiveRevision, setDetailArchiveRevision] =
    useState(archiveRevision);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [selectedCallId, setSelectedCallId] = useState<number | null>(null);
  const [request, setRequest] = useState<RequestContents | null>(null);
  const [requestLoading, setRequestLoading] = useState(false);
  const [requestError, setRequestError] = useState(false);
  const closeRef = useRef<HTMLButtonElement>(null);
  const loadGeneration = useRef(0);
  const requestGeneration = useRef(0);
  const archiveRevisionRef = useRef(archiveRevision);
  archiveRevisionRef.current = archiveRevision;
  const tabRefs = useRef<Record<Tab, HTMLButtonElement | null>>({
    overview: null,
    requests: null,
    calls: null,
  });

  const load = useCallback(
    async (preferredRun?: number | null, background = false) => {
      const generation = ++loadGeneration.current;
      const requestedArchiveRevision = archiveRevisionRef.current;
      if (!background) setLoading(true);
      try {
        const available = await historyApi.runs(entry.id);
        const selected =
          available.find((run) => run.id === preferredRun) ?? available[0];
        const nextDetail = selected ? await historyApi.run(selected.id) : null;
        if (generation !== loadGeneration.current) return;
        setRuns(available);
        setRunId(selected?.id ?? null);
        setDetail(nextDetail);
        setDetailArchiveRevision(requestedArchiveRevision);
        setLoadError(false);
      } catch {
        if (generation === loadGeneration.current) setLoadError(true);
      } finally {
        if (generation === loadGeneration.current) setLoading(false);
      }
    },
    [entry.id],
  );

  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    if (!loading || archiveRevision !== detailArchiveRevision)
      void load(runId, true);
  }, [refreshKey]);
  useEffect(() => {
    if (detail?.run.status !== "running") return;
    const timer = window.setInterval(() => {
      void load(runId, true);
    }, 1500);
    return () => window.clearInterval(timer);
  }, [detail?.run.status, load, runId]);
  useEffect(() => {
    closeRef.current?.focus();
  }, [entry.id]);
  useEffect(
    () => () => {
      loadGeneration.current++;
      requestGeneration.current++;
    },
    [],
  );

  const selectCall = useCallback(async (callId: number) => {
    const generation = ++requestGeneration.current;
    setSelectedCallId(callId);
    setRequest(null);
    setRequestLoading(true);
    setRequestError(false);
    try {
      const next = await historyApi.request(callId);
      if (generation === requestGeneration.current)
        setRequest(
          next ?? {
            call_id: callId,
            archive_status: "not_recorded",
            request_json: null,
            byte_count: 0,
            prompt_template: null,
          },
        );
    } catch {
      if (generation === requestGeneration.current) setRequestError(true);
    } finally {
      if (generation === requestGeneration.current) setRequestLoading(false);
    }
  }, []);

  useEffect(() => {
    if (tab !== "requests") return;
    const call =
      detail?.calls.find((item) => item.id === selectedCallId) ??
      detail?.calls[0];
    if (call && (!request || request.call_id !== call.id))
      void selectCall(call.id);
  }, [tab, detail?.calls, selectedCallId, request, selectCall]);
  useEffect(() => {
    requestGeneration.current++;
    setRequest(null);
    setRequestLoading(false);
    if (tab === "requests" && selectedCallId !== null)
      void selectCall(selectedCallId);
  }, [refreshKey]);

  const copy = async (value: string) => {
    if (await copyToClipboard(value))
      toast.success(t("settings.history.inspector.copied"));
    else toast.error(t("settings.history.copyError"));
  };

  const onTabKeyDown = (
    event: React.KeyboardEvent<HTMLButtonElement>,
    current: Tab,
  ) => {
    const index = tabs.indexOf(current);
    const next =
      event.key === "ArrowRight"
        ? tabs[(index + 1) % tabs.length]
        : event.key === "ArrowLeft"
          ? tabs[(index + tabs.length - 1) % tabs.length]
          : event.key === "Home"
            ? tabs[0]
            : event.key === "End"
              ? tabs[tabs.length - 1]
              : null;
    if (next) {
      event.preventDefault();
      setTab(next);
      tabRefs.current[next]?.focus();
    }
  };

  const run = detail?.run;
  const calls = detail?.calls ?? [];
  const resolvedCall =
    calls.find((call) => call.id === selectedCallId) ?? calls[0];
  const visibleRequest = request?.call_id === resolvedCall?.id ? request : null;
  const visibleTemplate =
    detailArchiveRevision === archiveRevision ? run?.prompt_template : null;
  const failureCode =
    run?.error_code === "processing_failed"
      ? ([...calls].reverse().find((call) => call.error_code)?.error_code ??
        run.error_code)
      : run?.error_code;
  const errorName = failureCode ? (errorNames[failureCode] ?? "generic") : null;
  const lastCall = calls[calls.length - 1];
  const notReported = t("settings.history.inspector.notReported");
  const totals = summarizeCalls(calls);
  const totalsComplete = !run?.details_incomplete && !metadataWarning;
  const aggregate = (field: string) => {
    const values = calls
      .map((call) => parseUsage(call)?.[field])
      .filter((value): value is number => typeof value === "number");
    return values.length
      ? `${values.reduce((sum, value) => sum + value, 0).toLocaleString()}${values.length < calls.length ? ` ${t("settings.history.inspector.knownOnly")}` : ""}`
      : notReported;
  };

  return (
    <section
      id={`history-inspector-${entry.id}`}
      aria-label={t("settings.history.inspector.title")}
      className="min-w-0 flex-1 border-l border-mid-gray/20 bg-background"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
      }}
    >
      <div className="sticky top-0 z-10 flex items-center justify-between border-b border-mid-gray/20 bg-background px-4 py-2">
        <h3 className="text-sm font-semibold">
          {t("settings.history.inspector.title")}
        </h3>
        <button
          ref={closeRef}
          type="button"
          onClick={onClose}
          className={textButtonClass}
          aria-label={t("settings.history.inspector.close")}
        >
          <X size={15} />
          {t("settings.history.inspector.close")}
        </button>
      </div>
      <div className="max-h-[calc(100vh-11rem)] min-h-0 overflow-y-auto px-4 pb-5">
        {loading && (
          <p className="py-4 text-sm text-text/60">
            {t("settings.history.inspector.loading")}
          </p>
        )}
        {loadError && (
          <div className="py-4 text-sm" role="alert">
            {t("settings.history.inspector.loadError")}{" "}
            <button
              type="button"
              className={textButtonClass}
              onClick={() => void load(runId)}
            >
              <RotateCcw size={13} />
              {t("settings.history.inspector.reload")}
            </button>
          </div>
        )}
        {!loading && (!loadError || detail) && (
          <>
            {runs.length > 1 && (
              <label className="mt-3 flex items-center gap-2 text-xs text-text/60">
                {t("settings.history.inspector.attempt")}
                <select
                  aria-label={t("settings.history.inspector.attempt")}
                  value={runId ?? ""}
                  onChange={(event) => {
                    requestGeneration.current++;
                    setRequest(null);
                    setSelectedCallId(null);
                    void load(Number(event.target.value));
                  }}
                  className="min-w-0 flex-1 rounded border border-mid-gray/30 bg-background px-2 py-1 text-text"
                >
                  {runs.map((item, index) => (
                    <option key={item.id} value={item.id}>
                      {t("settings.history.inspector.attemptNumber", {
                        number: runs.length - index,
                      })}{" "}
                      ·{" "}
                      {t(`settings.history.inspector.status.${item.status}`, {
                        defaultValue: item.status,
                      })}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {run && (
              <div className="mt-3 text-xs text-text/60">
                {t(`settings.history.inspector.status.${run.status}`, {
                  defaultValue: run.status,
                })}
              </div>
            )}
            {(run?.details_incomplete || metadataWarning) && (
              <p
                role="alert"
                className="mt-2 text-xs text-red-600 dark:text-red-400"
              >
                {t("settings.history.inspector.detailsSaveFailed")}
              </p>
            )}
            <div
              role="tablist"
              aria-label={t("settings.history.inspector.tabsLabel")}
              className="mt-3 flex border-b border-mid-gray/20"
            >
              {tabs.map((item) => (
                <button
                  key={item}
                  ref={(node) => {
                    tabRefs.current[item] = node;
                  }}
                  role="tab"
                  type="button"
                  id={`history-tab-${entry.id}-${item}`}
                  aria-controls={`history-panel-${entry.id}-${item}`}
                  aria-selected={tab === item}
                  tabIndex={tab === item ? 0 : -1}
                  onClick={() => setTab(item)}
                  onKeyDown={(event) => onTabKeyDown(event, item)}
                  className={`px-3 py-2 text-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-logo-primary ${tab === item ? "border-b-2 border-logo-primary text-logo-primary" : "text-text/60 hover:text-text"}`}
                >
                  {t(`settings.history.inspector.tab.${item}`)}
                </button>
              ))}
            </div>
            <div
              role="tabpanel"
              id={`history-panel-${entry.id}-${tab}`}
              aria-labelledby={`history-tab-${entry.id}-${tab}`}
              className="min-w-0 pt-4"
            >
              {tab === "overview" && (
                <>
                  {errorName && (
                    <div
                      className="mb-4 rounded-md border border-red-500/30 bg-red-500/5 p-3"
                      role="status"
                    >
                      <h4 className="text-sm font-medium">
                        {t(
                          `settings.history.inspector.error.${errorName}.title`,
                        )}
                      </h4>
                      <p className="mt-1 text-xs text-text/80">
                        {failureCode === "http_error" && lastCall?.http_status
                          ? t("settings.history.inspector.httpStatus", {
                              status: lastCall.http_status,
                            })
                          : t(
                              `settings.history.inspector.error.${errorName}.reason`,
                            )}
                      </p>
                      <p className="mt-1 text-xs text-text/60">
                        {t(
                          `settings.history.inspector.error.${errorName}.next`,
                        )}
                      </p>
                    </div>
                  )}
                  {run?.error_code === "missing_model" && !calls.length && (
                    <p className="mb-3 text-xs text-text/60">
                      {t("settings.history.inspector.noProviderRequest")}
                    </p>
                  )}
                  <dl className="grid grid-cols-2 gap-x-3 gap-y-3 border-b border-mid-gray/20 pb-4 sm:grid-cols-3">
                    <Metric
                      label={t("settings.history.inspector.provider")}
                      value={
                        run?.provider_name ??
                        lastCall?.provider_name ??
                        notReported
                      }
                    />
                    <Metric
                      label={t("settings.history.inspector.model")}
                      value={
                        run?.requested_model ??
                        lastCall?.requested_model ??
                        notReported
                      }
                    />
                    <Metric
                      label={t("settings.history.inspector.profile")}
                      value={run?.profile_name ?? notReported}
                    />
                    <Metric
                      label={t("settings.history.inspector.duration")}
                      value={duration(run?.elapsed_ms ?? null, notReported)}
                    />
                    <Metric
                      label={t("settings.history.inspector.tokens")}
                      value={
                        totals.knownTokens === null
                          ? notReported
                          : `${totals.knownTokens.toLocaleString()}${totals.usageComplete && totalsComplete ? "" : ` ${t("settings.history.inspector.knownOnly")}`}`
                      }
                    />
                    <Metric
                      label={t("settings.history.inspector.cost")}
                      value={
                        totals.knownCostUsd
                          ? `${t(totals.costComplete && totalsComplete ? "settings.history.inspector.estimated" : "settings.history.inspector.knownEstimated")} ${formatUsd(totals.knownCostUsd)}`
                          : t("settings.history.inspector.unavailable")
                      }
                    />
                    {!totals.knownCostUsd && !!calls.length && (
                      <p className="col-span-full text-xs text-text/55">
                        {t("settings.history.inspector.costUnavailableReason")}
                      </p>
                    )}
                    <Metric
                      label={t("settings.history.inspector.inputTokens")}
                      value={aggregate("input_tokens")}
                    />
                    <Metric
                      label={t("settings.history.inspector.cachedTokens")}
                      value={aggregate("cached_input_tokens")}
                    />
                    <Metric
                      label={t("settings.history.inspector.outputTokens")}
                      value={aggregate("output_tokens")}
                    />
                    {lastCall?.reported_model &&
                      lastCall.reported_model !== lastCall.requested_model && (
                        <Metric
                          label={t("settings.history.inspector.returnedModel")}
                          value={lastCall.reported_model}
                        />
                      )}
                    {run?.profile_revision !== null &&
                      run?.profile_revision !== undefined && (
                        <Metric
                          label={t(
                            "settings.history.inspector.profileRevision",
                          )}
                          value={run.profile_revision}
                        />
                      )}
                    {run?.prompt_source && (
                      <Metric
                        label={t("settings.history.inspector.promptSource")}
                        value={t(
                          `settings.history.inspector.promptSourceValue.${run.prompt_source}`,
                          { defaultValue: run.prompt_source },
                        )}
                      />
                    )}
                    <Metric
                      label={t("settings.history.inspector.output")}
                      value={
                        run?.output_outcome
                          ? t(
                              `settings.history.inspector.outputStatus.${run.output_outcome}`,
                              { defaultValue: run.output_outcome },
                            )
                          : notReported
                      }
                    />
                    <Metric
                      label={t("settings.history.inspector.stopToOutput")}
                      value={duration(
                        run?.stop_to_output_ms ?? null,
                        notReported,
                      )}
                    />
                    {run?.validated_operation && (
                      <Metric
                        label={t("settings.history.inspector.validatedAction")}
                        value={`${t(`settings.history.inspector.action.${run.validated_operation}`, { defaultValue: run.validated_operation })}${run.validated_effect_kind ? ` · ${t(`settings.history.inspector.effect.${run.validated_effect_kind}`, { defaultValue: run.validated_effect_kind })}` : ""}`}
                      />
                    )}
                  </dl>
                  {!run && (
                    <p className="my-4 text-sm text-text/60">
                      {t("settings.history.inspector.legacy")}
                    </p>
                  )}
                  <TextSection
                    label={t("settings.history.inspector.original")}
                    text={run?.original_text ?? entry.transcription_text}
                    onCopy={copy}
                    copyLabel={t("settings.history.inspector.copyOriginal")}
                  />
                  {(run ? run.processed_text : entry.post_processed_text) && (
                    <TextSection
                      label={t("settings.history.inspector.processed")}
                      text={
                        (run
                          ? run.processed_text
                          : entry.post_processed_text) ?? ""
                      }
                      onCopy={copy}
                      copyLabel={t("settings.history.inspector.copyProcessed")}
                    />
                  )}
                  {(run ? visibleTemplate : entry.post_process_prompt) && (
                    <TextSection
                      label={t("settings.history.inspector.template")}
                      text={
                        (run ? visibleTemplate : entry.post_process_prompt) ??
                        ""
                      }
                      onCopy={copy}
                      copyLabel={t("settings.history.inspector.copyTemplate")}
                    />
                  )}
                </>
              )}
              {tab === "requests" && (
                <>
                  {!calls.length && (
                    <p className="text-sm text-text/60">
                      {!run
                        ? t("settings.history.inspector.legacy")
                        : run.error_code === "missing_model"
                          ? t("settings.history.inspector.noProviderRequest")
                          : t("settings.history.inspector.noCalls")}
                    </p>
                  )}
                  {!!calls.length && (
                    <>
                      <CallPicker
                        calls={calls}
                        selected={selectedCallId ?? calls[0].id}
                        onSelect={(id) => void selectCall(id)}
                      />
                      {requestLoading && (
                        <p className="py-3 text-sm text-text/60">
                          {t("settings.history.inspector.loading")}
                        </p>
                      )}
                      {requestError && (
                        <p role="alert" className="py-3 text-sm">
                          {t("settings.history.inspector.loadError")}{" "}
                          <button
                            type="button"
                            className={textButtonClass}
                            onClick={() =>
                              void selectCall(selectedCallId ?? calls[0].id)
                            }
                          >
                            {t("settings.history.inspector.reload")}
                          </button>
                        </p>
                      )}
                      {visibleRequest && !requestLoading && (
                        <>
                          {visibleRequest.archive_status === "retained" &&
                          visibleRequest.request_json ? (
                            <TextSection
                              label={t(
                                "settings.history.inspector.exactRequest",
                              )}
                              text={visibleRequest.request_json}
                              onCopy={copy}
                              copyLabel={t(
                                "settings.history.inspector.copyRequest",
                              )}
                              json
                            />
                          ) : (
                            <p className="py-3 text-sm text-text/60">
                              {t(
                                `settings.history.inspector.archive.${visibleRequest.archive_status}`,
                                {
                                  defaultValue: t(
                                    "settings.history.inspector.archive.not_recorded",
                                  ),
                                },
                              )}
                            </p>
                          )}
                        </>
                      )}
                    </>
                  )}
                </>
              )}
              {tab === "calls" && (
                <>
                  {!calls.length && (
                    <p className="text-sm text-text/60">
                      {run?.compatibility_cache_hit
                        ? t("settings.history.inspector.cacheHit")
                        : !run
                          ? t("settings.history.inspector.legacy")
                          : run.error_code === "missing_model"
                            ? t("settings.history.inspector.noProviderRequest")
                            : t("settings.history.inspector.noCalls")}
                    </p>
                  )}
                  {run?.compatibility_cache_hit && !!calls.length && (
                    <p className="mb-3 text-xs text-text/60">
                      {t("settings.history.inspector.cacheHit")}
                    </p>
                  )}
                  <ol className="space-y-3">
                    {calls.map((call) => (
                      <CallCard key={call.id} call={call} calls={calls} />
                    ))}
                  </ol>
                </>
              )}
            </div>
          </>
        )}
      </div>
    </section>
  );
}

function TextSection({
  label,
  text,
  copyLabel,
  onCopy,
  json = false,
}: {
  label: string;
  text: string;
  copyLabel: string;
  onCopy: (text: string) => void;
  json?: boolean;
}) {
  let display = text;
  if (json) {
    try {
      display = JSON.stringify(JSON.parse(text), null, 2);
    } catch {
      display = text;
    }
  }
  return (
    <section className="min-w-0 border-b border-mid-gray/15 py-3">
      <div className="flex items-center justify-between gap-2">
        <h4 className="text-xs font-semibold uppercase tracking-wide text-text/60">
          {label}
        </h4>
        <button
          type="button"
          className={textButtonClass}
          onClick={() => onCopy(text)}
        >
          <Copy size={13} />
          {copyLabel}
        </button>
      </div>
      {json ? (
        <pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap break-all rounded bg-mid-gray/10 p-2 text-xs select-text">
          {display}
        </pre>
      ) : (
        <p className="mt-2 max-h-80 overflow-y-auto whitespace-pre-wrap break-words text-sm select-text">
          {text}
        </p>
      )}
    </section>
  );
}

function CallPicker({
  calls,
  selected,
  onSelect,
}: {
  calls: ProviderCall[];
  selected: number;
  onSelect: (id: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <div
      className="flex flex-wrap gap-1"
      aria-label={t("settings.history.inspector.callsLabel")}
    >
      {calls.map((call) => (
        <button
          key={call.id}
          type="button"
          aria-pressed={selected === call.id}
          onClick={() => onSelect(call.id)}
          className={`rounded-md border px-2 py-1 text-xs focus-visible:outline focus-visible:outline-2 focus-visible:outline-logo-primary ${selected === call.id ? "border-logo-primary text-logo-primary" : "border-mid-gray/25 text-text/70"}`}
        >
          {t("settings.history.inspector.callNumber", { number: call.ordinal })}{" "}
          ·{" "}
          {t(`settings.history.inspector.purpose.${call.purpose}`, {
            defaultValue: call.purpose,
          })}
        </button>
      ))}
    </div>
  );
}

function CallCard({
  call,
  calls,
}: {
  call: ProviderCall;
  calls: ProviderCall[];
}) {
  const { t } = useTranslation();
  const usage = parseUsage(call);
  let rate: Record<string, string> | null = null;
  if (call.rate_json) {
    try {
      const parsed: unknown = JSON.parse(call.rate_json);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed))
        rate = parsed as Record<string, string>;
    } catch {
      rate = null;
    }
  }
  const priorOrdinal = calls.find((item) => item.id === call.retry_of)?.ordinal;
  const notReported = t("settings.history.inspector.notReported");
  return (
    <li className="min-w-0 rounded-md border border-mid-gray/25 p-3">
      <h4 className="text-sm font-medium">
        {t("settings.history.inspector.callNumber", { number: call.ordinal })} ·{" "}
        {t(`settings.history.inspector.purpose.${call.purpose}`, {
          defaultValue: call.purpose,
        })}
      </h4>
      <dl className="mt-2 grid grid-cols-2 gap-2 text-xs sm:grid-cols-3">
        <Metric
          label={t("settings.history.inspector.statusLabel")}
          value={
            call.http_status ??
            t(`settings.history.inspector.callOutcome.${call.outcome}`, {
              defaultValue: call.outcome,
            })
          }
        />
        <Metric
          label={t("settings.history.inspector.duration")}
          value={duration(call.elapsed_ms, notReported)}
        />
        <Metric
          label={t("settings.history.inspector.model")}
          value={call.requested_model}
        />
        {call.reported_model &&
          call.reported_model !== call.requested_model && (
            <Metric
              label={t("settings.history.inspector.returnedModel")}
              value={call.reported_model}
            />
          )}
        <Metric
          label={t("settings.history.inspector.inputTokens")}
          value={usageValue(usage?.input_tokens, notReported)}
        />
        <Metric
          label={t("settings.history.inspector.cachedTokens")}
          value={usageValue(usage?.cached_input_tokens, notReported)}
        />
        <Metric
          label={t("settings.history.inspector.cacheWriteTokens")}
          value={usageValue(usage?.cache_write_tokens, notReported)}
        />
        <Metric
          label={t("settings.history.inspector.outputTokens")}
          value={usageValue(usage?.output_tokens, notReported)}
        />
        <Metric
          label={t("settings.history.inspector.totalTokens")}
          value={`${usageValue(usage?.total_tokens, notReported)}${usage?.total_source === "derived" ? ` (${t("settings.history.inspector.derived")})` : ""}`}
        />
        <Metric
          label={t("settings.history.inspector.cost")}
          value={
            call.cost_usd
              ? `${t("settings.history.inspector.estimated")} ${formatUsd(call.cost_usd)}`
              : t("settings.history.inspector.unavailable")
          }
        />
        {priorOrdinal && (
          <Metric
            label={t("settings.history.inspector.retryOf")}
            value={t("settings.history.inspector.callNumber", {
              number: priorOrdinal,
            })}
          />
        )}
        {call.provider_request_id && (
          <Metric
            label={t("settings.history.inspector.providerRequestId")}
            value={call.provider_request_id}
          />
        )}
        {rate?.source && (
          <Metric
            label={t("settings.history.inspector.rateSource")}
            value={rate.source}
          />
        )}
        {rate?.verified_at && (
          <Metric
            label={t("settings.history.inspector.rateVerified")}
            value={rate.verified_at}
          />
        )}
      </dl>
      {!call.cost_usd && (
        <p className="mt-2 text-xs text-text/55">
          {t("settings.history.inspector.costUnavailableReason")}
        </p>
      )}
      {call.error_code && (
        <p className="mt-2 text-xs text-text/70">
          {t(
            `settings.history.inspector.error.${errorNames[call.error_code] ?? "generic"}.title`,
          )}
        </p>
      )}
    </li>
  );
}
