import type { ComponentType } from "react";
import { IconPlug } from "@tabler/icons-react";
import type { PluginLogo as PluginLogoAsset } from "@ora/contracts";
import { useDarkTheme } from "../../state/hooks/use-dark-theme";

/**
 * Renders a plugin's own brand mark from the host-local asset URL the backend hands out.
 *
 * The mark is drawn through an `<img>`: an SVG referenced as an image never runs scripts or
 * loads external resources, so it stays inert even if a future package slips something past
 * validation, and a bitmap icon needs no separate path. The bytes come from the host's own
 * protocol, so nothing here reaches the network.
 *
 * A plugin that ships a theme pair gets the half drawn for the current background, chosen host-
 * side into two URLs; the host never recolours, inverts or composites an icon, so a plugin with
 * a single icon is drawn identically under both themes. Packages without a logo fall back to a
 * generic mark so every row keeps the same shape; `fallback` lets a surface pick one that reads
 * correctly for what it is listing, such as an agent rather than a plugin in general.
 */
export function PluginLogoMark({
  logo,
  className,
  fallback: Fallback = IconPlug,
}: {
  logo: PluginLogoAsset | null | undefined;
  className?: string;
  fallback?: ComponentType<{ className?: string }>;
}) {
  const dark = useDarkTheme();
  if (logo === null || logo === undefined) {
    return <Fallback className={className} />;
  }
  const source =
    logo.variant === "universal" ? logo.url : dark ? logo.dark : logo.light;
  return <img src={source} alt="" aria-hidden="true" className={className} />;
}

/** The settings list's fixed-size plugin mark, centred in the row's leading column. */
export function PluginLogo({ logo }: { logo: PluginLogoAsset | null }) {
  return (
    <span className="flex size-10 shrink-0 items-center justify-center text-muted-foreground">
      <PluginLogoMark logo={logo} className="size-6 object-contain" />
    </span>
  );
}
