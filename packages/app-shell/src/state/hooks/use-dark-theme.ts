import { useSyncExternalStore } from "react";
import {
  DARK_THEME_QUERY,
  isDarkTheme,
  useSettingsStore,
} from "../stores/settings-store";

/**
 * Reports whether the UI is currently rendering dark, following both the stored preference and,
 * when that preference is "system", the OS setting.
 *
 * Both inputs have to be watched: a preference change comes through the settings store, while a
 * user flipping their OS theme with the preference left on "system" only shows up as a media
 * query change, and a component that missed the second would keep showing the icon drawn for the
 * other background until something unrelated re-rendered it.
 */
export function useDarkTheme(): boolean {
  const theme = useSettingsStore((state) => state.settings.theme);
  const systemDark = useSyncExternalStore(subscribeToSystemTheme, () =>
    isDarkTheme("system"),
  );
  return theme === "system" ? systemDark : theme === "dark";
}

/** Subscribes one listener to OS theme changes for `useSyncExternalStore`. */
function subscribeToSystemTheme(onChange: () => void): () => void {
  const media = window.matchMedia(DARK_THEME_QUERY);
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}
