// frontend/src/hooks/useConfig.ts
import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "react-hot-toast";
import { ConfigData } from "../types/config.types";
import { configService } from "../services/configService";

export const useConfig = () => {
  const { t } = useTranslation();
  const [config, setConfig] = useState<ConfigData | null>(null);
  const [originalPassword, setOriginalPassword] = useState<string>("");
  const [originalAdminPassword, setOriginalAdminPassword] =
    useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  // Fetch config on hook initialization
  const fetchConfig = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const data = await configService.fetchConfig();
      setConfig(data);
      // Store both original passwords for comparison later
      setOriginalPassword(data.password || "");
      setOriginalAdminPassword(data.admin_password || "");
    } catch (err) {
      setError(
        t("common.error", {
          message: err instanceof Error ? err.message : String(err),
        }),
      );
      console.error("Config fetch error:", err);
    } finally {
      setLoading(false);
    }
  }, [t]);

  // Save config
  const saveConfig = useCallback(async () => {
    if (!config) return;

    setSaving(true);
    setError("");
    try {
      await configService.saveConfig(config);
      toast.success(t("config.success"));

      // Check if passwords were changed
      const adminPasswordChanged = configService.isAdminPasswordChanged(
        originalAdminPassword,
        config.admin_password,
      );
      const passwordChanged = configService.isPasswordChanged(
        originalPassword,
        config.password,
      );

      // Handle regular password change (API password)
      if (passwordChanged && !adminPasswordChanged) {
        toast.success(t("config.passwordChanged"), {
          duration: 2000,
          icon: "🔑",
        });
      }

      // Handle admin password change - requires logout
      if (adminPasswordChanged) {
        // Show toast notification
        toast.success(t("config.adminPasswordChanged"), {
          duration: 3000,
          icon: "🔐",
        });

        // Wait 3 seconds before logging out to allow user to see the toast
        setTimeout(() => {
          localStorage.removeItem("authToken");
          // Redirect with a query parameter to indicate password change
          window.location.href = "/?passwordChanged=true";
        }, 3000);
      }
    } catch (err) {
      setError(
        t("common.error", {
          message: err instanceof Error ? err.message : String(err),
        }),
      );
      console.error("Config save error:", err);
      toast.error(t("config.error"));
    } finally {
      setSaving(false);
    }
  }, [config, originalPassword, originalAdminPassword, t]);

  // Handle form changes
  const handleChange = useCallback(
    (
      e: React.ChangeEvent<
        HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
      >,
    ) => {
      if (!config) return;
      const updatedConfig = configService.handleConfigChange(config, e);
      setConfig(updatedConfig);
    },
    [config],
  );

  // Fetch config on mount
  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  return {
    config,
    loading,
    saving,
    error,
    fetchConfig,
    saveConfig,
    handleChange,
  };
};
