import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";

export function ContextProfilesToggle() {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const change = async (enabled: boolean) => {
    setBusy(true);
    setError("");
    try {
      const result = await commands.changePostProcessProfilesSetting(enabled);
      if (result.status === "error") setError(result.error);
      await refreshSettings();
    } catch {
      setError(t("profiles.requestFailed"));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div>
      <ToggleSwitch
        checked={settings?.post_process_profiles ?? false}
        disabled={!settings?.post_process_enabled}
        isUpdating={busy}
        onChange={(enabled) => void change(enabled)}
        label={t("profiles.enable")}
        description={t("profiles.enableDescription")}
        descriptionMode="inline"
      />
      {busy && (
        <p role="status" className="text-xs p-2">
          {t("profiles.savingSetting")}
        </p>
      )}
      {error && (
        <p role="alert" className="text-xs p-2">
          {error}
        </p>
      )}
    </div>
  );
}
