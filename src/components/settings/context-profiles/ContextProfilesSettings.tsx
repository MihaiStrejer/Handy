import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import {
  commands,
  type Profile,
  type ProfileCatalog,
  type MemoryRecord,
} from "@/bindings";
import {
  BUILTIN_PROFILE_ICONS,
  ProfileIcon,
} from "@/components/icons/ProfileIcon";
import { Button } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import "./ContextProfilesSettings.css";

const tabs = ["context", "dictionary", "memory"] as const;
const variables = [
  "dictionary",
  "short_term_memory",
  "input_context",
  "transcript",
];
type Tab = (typeof tabs)[number];
let pendingUnmountSave: Promise<boolean> | null = null;

const normalizeDictionary = (profile: Profile): Profile => ({
  ...profile,
  dictionary: profile.dictionary.map((item) => ({
    ...item,
    misheard_forms: item.misheard_forms
      .map((form) => form.trim())
      .filter(Boolean),
  })),
});

export function ContextProfilesSettings() {
  const { t } = useTranslation();
  const [catalog, setCatalog] = useState<ProfileCatalog | null>(null);
  const [draft, setDraft] = useState<Profile | null>(null);
  const [tab, setTab] = useState<Tab>("context");
  const [error, setError] = useState("");
  const [iconError, setIconError] = useState("");
  const [busy, setBusy] = useState(false);
  const [editingPrompt, setEditingPrompt] = useState(false);
  const [promptDraft, setPromptDraft] = useState("");
  const [promptReset, setPromptReset] = useState(false);
  const [promptError, setPromptError] = useState("");
  const [promptBusy, setPromptBusy] = useState(false);
  const [memory, setMemory] = useState<MemoryRecord[]>([]);
  const promptRef = useRef<HTMLTextAreaElement>(null);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const catalogRef = useRef<ProfileCatalog | null>(null);
  const draftRef = useRef<Profile | null>(null);
  const editVersionRef = useRef(0);
  const savedVersionRef = useRef(0);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const savePromiseRef = useRef<Promise<boolean> | null>(null);
  const saveLatestRef = useRef<() => Promise<boolean>>(async () => true);
  const lastSaveErrorRef = useRef("");
  const mountedRef = useRef(true);
  const refreshMemoryRef = useRef<() => Promise<void>>(async () => {});
  const p = (key: string) => t(`profiles.${key}`);

  const choose = (profile: Profile) => {
    const next = structuredClone(profile);
    draftRef.current = next;
    editVersionRef.current = 0;
    savedVersionRef.current = 0;
    setDraft(next);
    setTab("context");
    setEditingPrompt(false);
    setError("");
    setIconError("");
    setMemory([]);
  };
  const load = async () => {
    try {
      const result = await commands.getContextProfiles();
      if (result.status === "error") {
        setError(result.error);
        return;
      }
      catalogRef.current = result.data;
      setCatalog(result.data);
      choose(
        result.data.profiles.find(
          (profile) => profile.id === draftRef.current?.id,
        ) ?? result.data.profiles[0],
      );
    } catch {
      setError(p("requestFailed"));
    }
  };
  useEffect(() => {
    mountedRef.current = true;
    void (async () => {
      if (pendingUnmountSave) await pendingUnmountSave;
      if (mountedRef.current) await load();
    })();
    return () => {
      mountedRef.current = false;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      const pending = saveLatestRef.current();
      pendingUnmountSave = pending;
      void pending.finally(() => {
        if (pendingUnmountSave === pending) pendingUnmountSave = null;
      });
    };
  }, []);
  useEffect(() => {
    let current = true;
    let request = 0;
    const profileId = draft?.id;
    setMemory([]);
    const refresh = async () => {
      if (!profileId) return;
      const version = ++request;
      try {
        const result = await commands.getProfileMemory(profileId);
        if (!current || version !== request) return;
        if (result.status === "ok") setMemory(result.data);
        else setError(result.error);
      } catch {
        if (current && version === request) setError(p("requestFailed"));
      }
    };
    refreshMemoryRef.current = refresh;
    // Subscribe before the initial read so a correction cannot fall in a gap.
    const subscription = listen<string>(
      "profile-memory-updated",
      ({ payload }) => {
        if (current && payload === profileId) void refresh();
      },
    );
    void subscription
      .then(() => {
        if (current) void refresh();
      })
      .catch(() => {
        if (current) {
          setError(p("requestFailed"));
          void refresh();
        }
      });
    return () => {
      current = false;
      refreshMemoryRef.current = async () => {};
      void subscription.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [draft?.id, tab]);

  const saveLatest = async (): Promise<boolean> => {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = null;
    if (savePromiseRef.current) return savePromiseRef.current;
    if (savedVersionRef.current === editVersionRef.current) return true;
    const work = async (): Promise<boolean> => {
      while (savedVersionRef.current !== editVersionRef.current) {
        const profile = draftRef.current;
        const currentCatalog = catalogRef.current;
        if (!profile || !currentCatalog) return false;
        const version = editVersionRef.current;
        try {
          const result = await commands.saveContextProfile(
            normalizeDictionary(profile),
            currentCatalog.revision,
          );
          if (result.status === "error") {
            if (editVersionRef.current !== version) continue;
            lastSaveErrorRef.current = result.error;
            if (mountedRef.current) {
              setError(result.error);
            }
            return false;
          }
          catalogRef.current = result.data;
          if (mountedRef.current) setCatalog(result.data);
          const saved = result.data.profiles.find((item) =>
            profile.id
              ? item.id === profile.id
              : item.name === profile.name.trim(),
          );
          if (!saved) return false;
          savedVersionRef.current = version;
          if (draftRef.current) {
            draftRef.current = {
              ...draftRef.current,
              id: saved.id,
              revision: saved.revision,
              dictionary_revision: saved.dictionary_revision,
            };
            if (mountedRef.current) setDraft(draftRef.current);
          }
          if (mountedRef.current) setError("");
          lastSaveErrorRef.current = "";
        } catch {
          if (editVersionRef.current !== version) continue;
          lastSaveErrorRef.current = p("requestFailed");
          if (mountedRef.current) {
            setError(p("requestFailed"));
          }
          return false;
        }
      }
      return true;
    };
    const pending = work();
    savePromiseRef.current = pending;
    try {
      return await pending;
    } finally {
      if (savePromiseRef.current === pending) savePromiseRef.current = null;
    }
  };
  saveLatestRef.current = saveLatest;

  const updateDraft = (update: (current: Profile) => Profile) => {
    const current = draftRef.current;
    if (!current) return;
    const next = update(current);
    draftRef.current = next;
    editVersionRef.current += 1;
    setDraft(next);
    setError("");
    lastSaveErrorRef.current = "";
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => void saveLatestRef.current(), 400);
  };

  const selectProfile = async (profile: Profile) => {
    if (draftRef.current?.id === profile.id && profile.id) return;
    if (!(await saveLatest())) return;
    choose(
      catalogRef.current?.profiles.find((item) => item.id === profile.id) ??
        profile,
    );
  };

  const removeProfile = async () => {
    if (!draftRef.current?.id || !window.confirm(p("deleteConfirm"))) return;
    setBusy(true);
    try {
      if (!(await saveLatest())) return;
      const current = draftRef.current;
      const currentCatalog = catalogRef.current;
      if (!current || !currentCatalog) return;
      const result = await commands.deleteContextProfile(
        current.id,
        currentCatalog.revision,
      );
      if (result.status === "error") {
        setError(result.error);
        return;
      }
      catalogRef.current = result.data;
      setCatalog(result.data);
      choose(result.data.profiles[0]);
    } catch {
      setError(p("requestFailed"));
    } finally {
      setBusy(false);
    }
  };
  const importIcon = async () => {
    setIconError("");
    setBusy(true);
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "PNG / ICO", extensions: ["png", "ico"] }],
      });
      if (typeof path !== "string") return;
      const result = await commands.importProfileIcon(path);
      if (result.status === "error") {
        setIconError(result.error);
        return;
      }
      updateDraft((current) => ({
        ...current,
        icon: { custom: result.data },
      }));
    } catch {
      setIconError(p("importIconFailed"));
    } finally {
      setBusy(false);
    }
  };
  const closePrompt = () => {
    setEditingPrompt(false);
    setPromptDraft("");
    setPromptReset(false);
    setPromptError("");
  };
  const savePrompt = async () => {
    setPromptError("");
    setPromptBusy(true);
    try {
      if (!(await saveLatest())) {
        setPromptError(lastSaveErrorRef.current || p("requestFailed"));
        return;
      }
      const current = draftRef.current;
      const currentCatalog = catalogRef.current;
      if (!current || !currentCatalog) return;
      const nextPrompt = promptReset ? null : promptDraft;
      if (current.prompt === nextPrompt) {
        closePrompt();
        return;
      }
      const result = await commands.saveContextProfile(
        { ...current, prompt: nextPrompt },
        currentCatalog.revision,
      );
      if (result.status === "error") {
        setPromptError(result.error);
        return;
      }
      const saved = result.data.profiles.find((item) => item.id === current.id);
      if (!saved) {
        setPromptError(p("requestFailed"));
        return;
      }
      catalogRef.current = result.data;
      draftRef.current = structuredClone(saved);
      setCatalog(result.data);
      setDraft(draftRef.current);
      closePrompt();
    } catch {
      setPromptError(p("requestFailed"));
    } finally {
      setPromptBusy(false);
    }
  };
  const removeMemory = async (id: string | null) => {
    if (!draft) return;
    setBusy(true);
    setError("");
    try {
      const result = await commands.removeProfileMemory(draft.id, id);
      if (result.status === "error") {
        setError(result.error);
        return;
      }
      await refreshMemoryRef.current();
    } catch {
      setError(p("requestFailed"));
    } finally {
      setBusy(false);
    }
  };

  if (!catalog || !draft)
    return <div role="status">{error || p("loading")}</div>;
  const general = catalog.profiles.find((profile) => profile.id === "general")!;
  const isGeneral = draft.id === "general";
  const inherited = !isGeneral && draft.prompt === null;
  const effectivePrompt = draft.prompt ?? general.prompt ?? "";

  return (
    <section className="context-profiles" aria-label={p("title")}>
      <nav className="profile-selector" aria-label={p("selector")}>
        <h2>{p("title")}</h2>
        <div className="profile-list">
          {catalog.profiles.map((profile) => {
            const subtitle =
              profile.id === "general"
                ? p("fallback")
                : profile.rules.map((rule) => rule.application).join(", ");
            return (
              <button
                className="profile-selector-item"
                key={profile.id}
                type="button"
                aria-current={draft.id === profile.id ? "true" : undefined}
                disabled={busy}
                onClick={() => void selectProfile(profile)}
                title={profile.name}
              >
                <ProfileIcon
                  icon={profile.icon}
                  className="profile-selector-icon"
                  size={17}
                />
                <span className="profile-selector-copy">
                  <span className="profile-selector-name">{profile.name}</span>
                  <small title={subtitle}>{subtitle}</small>
                </span>
              </button>
            );
          })}
        </div>
        <Button
          className="profile-add"
          size="sm"
          variant="secondary"
          disabled={busy}
          onClick={() =>
            void selectProfile({
              id: "",
              name: "",
              revision: 0,
              dictionary_revision: 0,
              icon: "terminal",
              rules: [{ application: "WindowsTerminal.exe", workspace: null }],
              prompt: null,
              dictionary: [],
            })
          }
        >
          {p("add")}
        </Button>
      </nav>
      <div className="profile-detail">
        <div role="tablist" aria-label={p("tabs")} className="profile-tabs">
          {tabs.map((name, index) => (
            <button
              key={name}
              type="button"
              role="tab"
              id={`profile-tab-${name}`}
              aria-selected={tab === name}
              aria-controls="profile-panel"
              tabIndex={tab === name ? 0 : -1}
              ref={(el) => {
                tabRefs.current[index] = el;
              }}
              onClick={() => setTab(name)}
              onKeyDown={(event) => {
                if (event.key !== "ArrowLeft" && event.key !== "ArrowRight")
                  return;
                event.preventDefault();
                const next =
                  (index +
                    (event.key === "ArrowRight" ? 1 : -1) +
                    tabs.length) %
                  tabs.length;
                setTab(tabs[next]);
                tabRefs.current[next]?.focus();
              }}
            >
              {p(`tab.${name}`)}
            </button>
          ))}
        </div>
        {error && (
          <div role="alert" className="profile-error">
            {error}
          </div>
        )}
        <div
          id="profile-panel"
          role="tabpanel"
          aria-labelledby={`profile-tab-${tab}`}
        >
          {tab === "context" && (
            <>
              <fieldset disabled={busy} className="profile-fields">
                <legend>{p("routing")}</legend>
                <label>
                  {p("name")}
                  <input
                    className="profile-field"
                    value={draft.name}
                    maxLength={80}
                    onChange={(event) =>
                      updateDraft((current) => ({
                        ...current,
                        name: event.target.value,
                      }))
                    }
                  />
                </label>
                <div className="profile-icon-field">
                  <span>{p("icon")}</span>
                  <div
                    className="profile-icon-choices"
                    role="radiogroup"
                    aria-label={p("icon")}
                  >
                    {BUILTIN_PROFILE_ICONS.map((icon) => (
                      <label
                        className="profile-icon-choice"
                        key={icon}
                        title={p(`icons.${icon}`)}
                      >
                        <input
                          type="radio"
                          name="profile-icon"
                          aria-label={p(`icons.${icon}`)}
                          value={icon}
                          checked={draft.icon === icon}
                          onChange={() => {
                            setIconError("");
                            updateDraft((current) => ({ ...current, icon }));
                          }}
                        />
                        <ProfileIcon icon={icon} size={20} />
                      </label>
                    ))}
                    {typeof draft.icon === "object" && (
                      <label
                        className="profile-icon-choice"
                        title={p("icons.custom")}
                      >
                        <input
                          type="radio"
                          name="profile-icon"
                          aria-label={p("icons.custom")}
                          value="custom"
                          checked
                          readOnly
                        />
                        <ProfileIcon icon={draft.icon} size={20} />
                      </label>
                    )}
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    className="profile-icon-import"
                    onClick={() => void importIcon()}
                  >
                    {p("importIcon")}
                  </Button>
                  {iconError && (
                    <p className="profile-icon-error" role="alert">
                      {iconError}
                    </p>
                  )}
                </div>
                {isGeneral ? (
                  <p className="profile-hint">{p("generalRouting")}</p>
                ) : (
                  <>
                    {draft.rules.map((rule, index) => (
                      <div className="profile-rule" key={index}>
                        <label>
                          {p("application")}
                          <input
                            className="profile-field"
                            value={rule.application}
                            onChange={(event) =>
                              updateDraft((current) => ({
                                ...current,
                                rules: current.rules.map((item, i) =>
                                  i === index
                                    ? {
                                        ...item,
                                        application: event.target.value,
                                      }
                                    : item,
                                ),
                              }))
                            }
                          />
                        </label>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() =>
                            updateDraft((current) => ({
                              ...current,
                              rules: current.rules.filter(
                                (_, i) => i !== index,
                              ),
                            }))
                          }
                        >
                          {p("remove")}
                        </Button>
                      </div>
                    ))}
                    <p className="profile-hint">{p("applicationHint")}</p>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() =>
                        updateDraft((current) => ({
                          ...current,
                          rules: [
                            ...current.rules,
                            { application: "", workspace: null },
                          ],
                        }))
                      }
                    >
                      {p("addApplication")}
                    </Button>
                  </>
                )}
              </fieldset>
              <section
                className="profile-prompt"
                aria-label={p("systemPrompt")}
              >
                <h3>{p("systemPrompt")}</h3>
                <p className="profile-hint">
                  {p(
                    isGeneral
                      ? "defaultPrompt"
                      : inherited
                        ? "inheritedPrompt"
                        : "customPrompt",
                  )}
                </p>
                <pre className="profile-prompt-preview">{effectivePrompt}</pre>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => {
                    setPromptDraft(effectivePrompt);
                    setPromptReset(false);
                    setPromptError("");
                    setEditingPrompt(true);
                  }}
                >
                  {p(inherited ? "customize" : "editPrompt")}
                </Button>
              </section>
              <div className="profile-actions profile-save-actions">
                {draft.id && !isGeneral && (
                  <Button
                    size="sm"
                    variant="danger-ghost"
                    disabled={busy}
                    onClick={() => void removeProfile()}
                  >
                    {p("delete")}
                  </Button>
                )}
              </div>
              <Dialog
                open={editingPrompt}
                title={p("systemPrompt")}
                description={p(
                  isGeneral
                    ? "defaultPrompt"
                    : inherited
                      ? "inheritedPrompt"
                      : "customPrompt",
                )}
                onOpenChange={(open) => {
                  if (!open) closePrompt();
                }}
                closeLabel={p("cancel")}
                dismissible={!promptBusy}
                initialFocusRef={promptRef}
                contentFades={false}
                footer={
                  <>
                    {!isGeneral && (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={promptBusy}
                        onClick={() => {
                          setPromptReset(true);
                          setPromptDraft(general.prompt ?? "");
                        }}
                      >
                        {p("resetPrompt")}
                      </Button>
                    )}
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={promptBusy}
                      onClick={closePrompt}
                    >
                      {p("cancel")}
                    </Button>
                    <Button
                      size="sm"
                      disabled={promptBusy}
                      onClick={() => void savePrompt()}
                    >
                      {p("savePromptAndClose")}
                    </Button>
                  </>
                }
              >
                <label
                  className="profile-sr-only"
                  htmlFor="profile-prompt-text"
                >
                  {p("systemPrompt")}
                </label>
                <textarea
                  id="profile-prompt-text"
                  ref={promptRef}
                  className="profile-field profile-prompt-editor"
                  rows={10}
                  maxLength={32000}
                  value={promptDraft}
                  disabled={promptBusy}
                  onChange={(event) => {
                    setPromptReset(false);
                    setPromptDraft(event.target.value);
                  }}
                />
                <div className="profile-variables" aria-label={p("variables")}>
                  {variables.map((variable) => (
                    <Button
                      key={variable}
                      size="sm"
                      variant="secondary"
                      disabled={promptBusy}
                      onClick={() => {
                        const input = promptRef.current;
                        const start =
                          input?.selectionStart ?? promptDraft.length;
                        const end = input?.selectionEnd ?? start;
                        const token = `{{${variable}}}`;
                        setPromptReset(false);
                        setPromptDraft(
                          promptDraft.slice(0, start) +
                            token +
                            promptDraft.slice(end),
                        );
                        requestAnimationFrame(() => {
                          input?.focus();
                          input?.setSelectionRange(
                            start + token.length,
                            start + token.length,
                          );
                        });
                      }}
                    >{`{{${variable}}}`}</Button>
                  ))}
                </div>
                <p className="profile-hint">{p("variablesHint")}</p>
                {promptError && (
                  <p role="alert" className="profile-error">
                    {promptError}
                  </p>
                )}
              </Dialog>
            </>
          )}
          {tab === "dictionary" && (
            <fieldset disabled={busy} className="profile-fields">
              <legend>{p("tab.dictionary")}</legend>
              <p className="profile-hint">{p("dictionaryHint")}</p>
              {draft.dictionary.length === 0 && (
                <p className="profile-hint">{p("dictionaryEmpty")}</p>
              )}
              {draft.dictionary.map((keyword, index) => (
                <div className="profile-keyword" key={keyword.id}>
                  <div className="profile-keyword-canonical">
                    <label>
                      {p("canonical")}
                      <input
                        className="profile-field"
                        value={keyword.canonical}
                        maxLength={120}
                        onChange={(event) =>
                          updateDraft((current) => ({
                            ...current,
                            dictionary: current.dictionary.map((item, i) =>
                              i === index
                                ? { ...item, canonical: event.target.value }
                                : item,
                            ),
                          }))
                        }
                      />
                    </label>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() =>
                        updateDraft((current) => ({
                          ...current,
                          dictionary: current.dictionary.filter(
                            (_, i) => i !== index,
                          ),
                        }))
                      }
                    >
                      {p("removeKeyword")}
                    </Button>
                  </div>
                  <label>
                    {p("misheard")}
                    <textarea
                      className="profile-field"
                      rows={3}
                      value={keyword.misheard_forms.join("\n")}
                      onChange={(event) =>
                        updateDraft((current) => ({
                          ...current,
                          dictionary: current.dictionary.map((item, i) =>
                            i === index
                              ? {
                                  ...item,
                                  misheard_forms:
                                    event.target.value.split("\n"),
                                }
                              : item,
                          ),
                        }))
                      }
                    />
                  </label>
                </div>
              ))}
              <div className="profile-actions">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() =>
                    updateDraft((current) => ({
                      ...current,
                      dictionary: [
                        ...current.dictionary,
                        {
                          id: crypto.randomUUID(),
                          canonical: "",
                          misheard_forms: [],
                        },
                      ],
                    }))
                  }
                >
                  {p("addKeyword")}
                </Button>
              </div>
            </fieldset>
          )}
          {tab === "memory" && (
            <section className="profile-fields" aria-label={p("tab.memory")}>
              <p className="profile-hint">{p("memoryHint")}</p>
              {memory.length === 0 ? (
                <p>{p("memoryEmpty")}</p>
              ) : (
                <>
                  <ul className="profile-memory">
                    {memory.map((item) => (
                      <li key={item.id}>
                        <p>{item.text}</p>
                        <Button
                          size="sm"
                          variant="ghost"
                          disabled={busy}
                          onClick={() => void removeMemory(item.id)}
                        >
                          {p("remove")}
                        </Button>
                      </li>
                    ))}
                  </ul>
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={busy}
                    onClick={() => void removeMemory(null)}
                  >
                    {p("clearMemory")}
                  </Button>
                </>
              )}
            </section>
          )}
        </div>
      </div>
    </section>
  );
}
