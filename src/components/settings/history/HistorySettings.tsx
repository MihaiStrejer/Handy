import React, { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";
import { listen } from "@tauri-apps/api/event";
import {
  Check,
  Copy,
  FolderOpen,
  PanelRightOpen,
  RotateCcw,
  Star,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  commands,
  events,
  type HistoryEntry,
  type HistoryUpdatePayload,
} from "@/bindings";
import { useOsType } from "@/hooks/useOsType";
import { useSettings } from "@/hooks/useSettings";
import { formatDateTime } from "@/utils/dateFormat";
import { AudioPlayer, AudioPlayerGroup } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";
import { copyToClipboard } from "./clipboard";
import {
  HistoryInspector,
  errorNames,
  textButtonClass,
} from "./HistoryInspector";
import { formatUsd, historyApi } from "./historyApi";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
    aria-label={title}
  >
    {children}
  </button>
);

const PAGE_SIZE = 30;

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="secondary"
    size="sm"
    className="flex items-center gap-2"
    title={label}
  >
    <FolderOpen className="w-4 h-4" />
    <span>{label}</span>
  </Button>
);

export const HistorySettings: React.FC = () => {
  const { t } = useTranslation();
  const osType = useOsType();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const entriesRef = useRef<HistoryEntry[]>([]);
  const loadingRef = useRef(false);
  const { settings, refreshSettings } = useSettings();
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [archiveRevision, setArchiveRevision] = useState(0);
  const [archiveBusy, setArchiveBusy] = useState(false);
  const [warningEntries, setWarningEntries] = useState<Set<number>>(new Set());
  const triggerRefs = useRef(new Map<number, HTMLButtonElement>());
  const archiveEnabled = settings?.save_history_request_contents ?? true;
  const selectedEntry =
    entries.find((entry) => entry.id === selectedId) ?? null;

  useEffect(() => {
    const unlisten = listen<number>("history-details-warning", (event) => {
      setWarningEntries((previous) => new Set(previous).add(event.payload));
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const closeInspector = () => {
    const id = selectedId;
    setSelectedId(null);
    if (id !== null)
      requestAnimationFrame(() => triggerRefs.current.get(id)?.focus());
  };

  const setArchive = async (enabled: boolean) => {
    setArchiveBusy(true);
    try {
      await historyApi.setArchiveEnabled(enabled);
      await refreshSettings();
    } catch {
      toast.error(t("settings.history.inspector.archiveUpdateError"));
    } finally {
      setArchiveBusy(false);
    }
  };

  const clearArchive = async () => {
    setArchiveBusy(true);
    try {
      await historyApi.clearRequests();
      setEntries((previous) =>
        previous.map((entry) => ({ ...entry, post_process_prompt: null })),
      );
      setArchiveRevision((value) => value + 1);
      toast.success(t("settings.history.inspector.archiveCleared"));
    } catch {
      toast.error(t("settings.history.inspector.archiveClearError"));
    } finally {
      setArchiveBusy(false);
    }
  };

  // Keep ref in sync for use in IntersectionObserver callback
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

  const loadPage = useCallback(async (cursor?: number) => {
    const isFirstPage = cursor === undefined;
    if (!isFirstPage && loadingRef.current) return;
    loadingRef.current = true;

    if (isFirstPage) setLoading(true);

    try {
      const result = await commands.getHistoryEntries(
        cursor ?? null,
        PAGE_SIZE,
      );
      if (result.status === "ok") {
        const { entries: newEntries, has_more } = result.data;
        setEntries((prev) =>
          isFirstPage ? newEntries : [...prev, ...newEntries],
        );
        setHasMore(has_more);
      }
    } catch (error) {
      console.error("Failed to load history entries:", error);
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadPage();
  }, [loadPage]);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage]);

  // Listen for new entries added from the transcription pipeline
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      const payload: HistoryUpdatePayload = event.payload;
      if (payload.action === "added") {
        setEntries((prev) => [payload.entry, ...prev]);
      } else if (payload.action === "updated") {
        setEntries((prev) =>
          prev.map((e) => (e.id === payload.entry.id ? payload.entry : e)),
        );
      }
      // "deleted" and "toggled" are handled by optimistic updates only,
      // so we intentionally ignore them here to avoid double-mutation.
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleSaved = async (id: number) => {
    // Optimistic update
    setEntries((prev) =>
      prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
    );
    try {
      const result = await commands.toggleHistoryEntrySaved(id);
      if (result.status !== "ok") {
        // Revert on failure
        setEntries((prev) =>
          prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
        );
      }
    } catch (error) {
      console.error("Failed to toggle saved status:", error);
      // Revert on failure
      setEntries((prev) =>
        prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
      );
    }
  };

  const getAudioUrl = useCallback(
    async (fileName: string) => {
      try {
        const result = await commands.getAudioFilePath(fileName);
        if (result.status === "ok") {
          if (osType === "linux") {
            const fileData = await readFile(result.data);
            const blob = new Blob([fileData], { type: "audio/wav" });
            return URL.createObjectURL(blob);
          }
          return convertFileSrc(result.data, "asset");
        }
        return null;
      } catch (error) {
        console.error("Failed to get audio file path:", error);
        return null;
      }
    },
    [osType],
  );

  const deleteAudioEntry = async (id: number) => {
    // Optimistically remove
    setEntries((prev) => prev.filter((e) => e.id !== id));
    if (selectedId === id) setSelectedId(null);
    setWarningEntries((previous) => {
      const next = new Set(previous);
      next.delete(id);
      return next;
    });
    try {
      const result = await commands.deleteHistoryEntry(id);
      if (result.status !== "ok") {
        // Reload on failure
        loadPage();
      }
    } catch (error) {
      console.error("Failed to delete entry:", error);
      loadPage();
    }
  };

  const retryHistoryEntry = async (id: number) => {
    const result = await commands.retryHistoryEntryTranscription(id);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
  };

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  let content: React.ReactNode;

  if (loading) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.loading")}
      </div>
    );
  } else if (entries.length === 0) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.empty")}
      </div>
    );
  } else {
    content = (
      <>
        <AudioPlayerGroup>
          <div className="divide-y divide-mid-gray/20">
            {entries.map((entry) => (
              <HistoryEntryComponent
                key={entry.id}
                entry={entry}
                onToggleSaved={() => toggleSaved(entry.id)}
                onCopyText={() => copyToClipboard(entry.transcription_text)}
                getAudioUrl={getAudioUrl}
                deleteAudio={deleteAudioEntry}
                retryTranscription={retryHistoryEntry}
                selected={selectedId === entry.id}
                onInspect={(button) => {
                  triggerRefs.current.set(entry.id, button);
                  setSelectedId(entry.id);
                }}
                registerInspect={(button) => {
                  if (button) triggerRefs.current.set(entry.id, button);
                  else triggerRefs.current.delete(entry.id);
                }}
                metadataWarning={warningEntries.has(entry.id)}
              />
            ))}
          </div>
        </AudioPlayerGroup>
        {/* Sentinel for infinite scroll */}
        <div ref={sentinelRef} className="h-1" />
      </>
    );
  }

  return (
    <div
      className={`w-full mx-auto space-y-6 ${selectedEntry ? "max-w-6xl" : "max-w-3xl"}`}
    >
      <div className="space-y-2">
        <div className="px-4 flex items-center justify-between">
          <div>
            <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
              {t("settings.history.title")}
            </h2>
          </div>
          <OpenRecordingsButton
            onClick={openRecordingsFolder}
            label={t("settings.history.openFolder")}
          />
        </div>
        <div className="mx-4 flex flex-wrap items-center justify-between gap-2 text-xs text-text/70">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={archiveEnabled}
              disabled={archiveBusy}
              onChange={(event) => void setArchive(event.target.checked)}
            />
            {t("settings.history.inspector.saveRequests")}
          </label>
          <button
            type="button"
            disabled={archiveBusy}
            onClick={() => void clearArchive()}
            className={textButtonClass}
          >
            {t("settings.history.inspector.clearRequests")}
          </button>
          <p className="w-full text-text/50">
            {t("settings.history.inspector.saveRequestsDescription")}
          </p>
        </div>
        <div className="flex min-w-0 overflow-hidden rounded-lg border border-mid-gray/20 bg-background">
          <div
            className={`min-w-0 ${selectedEntry ? "hidden lg:block lg:w-[44%] lg:flex-none lg:overflow-y-auto" : "w-full"}`}
          >
            {content}
          </div>
          {selectedEntry && (
            <HistoryInspector
              key={selectedEntry.id}
              entry={selectedEntry}
              onClose={closeInspector}
              metadataWarning={warningEntries.has(selectedEntry.id)}
              refreshKey={`${JSON.stringify(selectedEntry.processing_summary)}:${archiveRevision}`}
              archiveRevision={archiveRevision}
            />
          )}
        </div>
      </div>
    </div>
  );
};

interface HistoryEntryProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => Promise<boolean>;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number) => Promise<void>;
  selected: boolean;
  onInspect: (button: HTMLButtonElement) => void;
  registerInspect: (button: HTMLButtonElement | null) => void;
  metadataWarning: boolean;
}

const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
  selected,
  onInspect,
  registerInspect,
  metadataWarning,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);

  const hasTranscription = entry.transcription_text.trim().length > 0;

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleCopyText = async () => {
    if (!hasTranscription) {
      return;
    }

    const copied = await onCopyText();
    if (!copied) {
      toast.error(t("settings.history.copyError"));
      return;
    }

    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async () => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const formattedDate = formatDateTime(String(entry.timestamp), i18n.language);
  const summary = entry.processing_summary;
  const hasProcessingDetails = Boolean(
    summary || entry.post_processed_text || entry.post_process_prompt,
  );
  const status =
    summary?.status === "running"
      ? t("settings.history.inspector.status.running")
      : summary?.status === "succeeded"
        ? t("settings.history.inspector.status.succeeded")
        : summary?.status === "cancelled"
          ? t("settings.history.inspector.status.cancelled")
          : summary?.status === "interrupted"
            ? t("settings.history.inspector.status.interrupted")
            : summary?.status === "failed"
              ? t("settings.history.inspector.status.failed")
              : null;

  return (
    <div className="px-4 py-2 pb-5 flex flex-col gap-3">
      <div className="flex flex-wrap justify-between items-center gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <p className="text-sm font-medium" title={entry.title}>
            {formattedDate}
          </p>
          {status && <span className="text-xs text-text/60">{status}</span>}
        </div>
        <div className="flex items-center">
          <IconButton
            onClick={handleCopyText}
            disabled={!hasTranscription || retrying}
            title={t("settings.history.copyToClipboard")}
          >
            {showCopied ? (
              <Check width={16} height={16} />
            ) : (
              <Copy width={16} height={16} />
            )}
          </IconButton>
          {hasProcessingDetails && (
            <button
              ref={registerInspect}
              type="button"
              onClick={(event) => onInspect(event.currentTarget)}
              aria-label={t("settings.history.inspector.open")}
              title={t("settings.history.inspector.open")}
              aria-expanded={selected}
              aria-controls={`history-inspector-${entry.id}`}
              className="rounded-md p-1.5 text-text/50 hover:text-logo-primary focus-visible:outline focus-visible:outline-2 focus-visible:outline-logo-primary"
            >
              <PanelRightOpen size={16} />
            </button>
          )}
          <IconButton
            onClick={onToggleSaved}
            disabled={retrying}
            active={entry.saved}
            title={
              entry.saved
                ? t("settings.history.unsave")
                : t("settings.history.save")
            }
          >
            <Star
              width={16}
              height={16}
              fill={entry.saved ? "currentColor" : "none"}
            />
          </IconButton>
          <IconButton
            onClick={handleRetranscribe}
            disabled={retrying}
            title={t("settings.history.retranscribe")}
          >
            <RotateCcw
              width={16}
              height={16}
              style={
                retrying
                  ? { animation: "spin 1s linear infinite reverse" }
                  : undefined
              }
            />
          </IconButton>
          <IconButton
            onClick={handleDeleteEntry}
            disabled={retrying}
            title={t("settings.history.delete")}
          >
            <Trash2 width={16} height={16} />
          </IconButton>
        </div>
      </div>

      <p
        className={`italic text-sm pb-2 ${
          retrying
            ? ""
            : hasTranscription
              ? "text-text/90 select-text cursor-text whitespace-pre-wrap break-words"
              : "text-text/40"
        }`}
        style={
          retrying
            ? { animation: "transcribe-pulse 3s ease-in-out infinite" }
            : undefined
        }
      >
        {retrying && (
          <style>{`
            @keyframes transcribe-pulse {
              0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
              50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
            }
          `}</style>
        )}
        {retrying
          ? t("settings.history.transcribing")
          : hasTranscription
            ? entry.transcription_text
            : t("settings.history.transcriptionFailed")}
      </p>

      <AudioPlayer onLoadRequest={handleLoadAudio} className="w-full" />

      {summary && (
        <div className="rounded-md bg-mid-gray/5 px-2 py-2 text-xs text-text/65">
          <div className="flex flex-wrap gap-x-2 gap-y-1">
            {summary.provider_name && (
              <span>
                {summary.provider_name}
                {summary.requested_model ? ` / ${summary.requested_model}` : ""}
              </span>
            )}
            {summary.profile_name && <span>{summary.profile_name}</span>}
            {summary.elapsed_ms !== null && (
              <span>
                {t("settings.history.inspector.durationSeconds", {
                  seconds: (summary.elapsed_ms / 1000).toFixed(1),
                })}
              </span>
            )}
            {summary.known_total_tokens !== null && (
              <span>
                {summary.usage_complete
                  ? ""
                  : `${t("settings.history.inspector.knownOnly")} `}
                {summary.known_total_tokens.toLocaleString()}{" "}
                {t("settings.history.inspector.tokens").toLowerCase()}
              </span>
            )}
            {summary.total_cost_usd && (
              <span>
                {summary.cost_complete
                  ? t("settings.history.inspector.estimated")
                  : t("settings.history.inspector.knownEstimated")}{" "}
                {formatUsd(summary.total_cost_usd)}
              </span>
            )}
          </div>
          {summary.status === "failed" && (
            <p className="mt-1 text-red-600 dark:text-red-400" role="status">
              {summary.error_code === "http_error" && summary.http_status
                ? t("settings.history.inspector.httpStatus", {
                    status: summary.http_status,
                  })
                : t(
                    `settings.history.inspector.error.${errorNames[summary.error_code ?? ""] ?? "generic"}.reason`,
                  )}
            </p>
          )}
          {(summary.details_incomplete || metadataWarning) && (
            <p role="alert" className="mt-1 text-red-600 dark:text-red-400">
              {t("settings.history.inspector.detailsSaveFailed")}
            </p>
          )}
        </div>
      )}
      {!summary && metadataWarning && (
        <p role="alert" className="text-xs text-red-600 dark:text-red-400">
          {t("settings.history.inspector.detailsSaveFailed")}
        </p>
      )}
    </div>
  );
};
