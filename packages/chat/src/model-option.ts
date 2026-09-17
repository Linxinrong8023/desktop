import type * as acp from "@agentclientprotocol/sdk";

/**
 * Finds the agent's model selector among its configuration options.
 *
 * `category` is a UX hint the protocol says clients must tolerate missing, so a
 * lone select option is treated as the model picker when nothing is categorised.
 * An agent that exposes no selectable model yields `null`.
 */
export function findModelOption(
  configOptions: acp.SessionConfigOption[],
): acp.SessionConfigOption | null {
  const selects = configOptions.filter((option) => option.type === "select");
  return (
    selects.find((option) => option.category === "model") ??
    (selects.length === 1 ? selects[0]! : null)
  );
}

/**
 * Finds the agent's thought (reasoning effort) selector among its configuration options.
 *
 * Unlike the model picker there is no fallback for an uncategorised option: the
 * protocol names `thought_level` explicitly, and guessing would risk presenting
 * some unrelated selector as the effort control. An agent that reports none — or
 * has not reported its options yet — yields `null`, and the picker stays hidden.
 */
export function findThoughtLevelOption(
  configOptions: acp.SessionConfigOption[],
): acp.SessionConfigOption | null {
  return (
    configOptions.find(
      (option) =>
        option.type === "select" && option.category === "thought_level",
    ) ?? null
  );
}

/**
 * The thought levels that form an ordered scale, in the agent's order.
 *
 * Some agents list a `default` entry alongside the real levels. It is not a
 * level of its own but a pointer to whichever one the agent applies when not
 * told, and its position in the list says nothing about how much effort that
 * is — so it is left off the scale rather than shown as its lowest rung. A
 * session whose current value is `default` therefore maps to no rung.
 */
export function thoughtLevelScale(
  option: acp.SessionConfigOption,
): acp.SessionConfigSelectOption[] {
  return selectableValues(option).filter((value) => value.value !== "default");
}

/** Flattens grouped and ungrouped select values into one ordered list. */
export function selectableValues(
  option: acp.SessionConfigOption,
): acp.SessionConfigSelectOption[] {
  if (option.type !== "select") return [];
  return option.options.flatMap((entry) =>
    "group" in entry ? entry.options : [entry],
  );
}

/** Returns the human-readable name of the option value currently in effect. */
export function currentValueName(
  option: acp.SessionConfigOption,
): string | null {
  if (option.type !== "select") return null;
  const current = selectableValues(option).find(
    (value) => value.value === option.currentValue,
  );
  return current?.name ?? option.currentValue;
}

/** Describes the model in effect, or `null` when the agent offers no model selector. */
export function currentModel(
  configOptions: acp.SessionConfigOption[],
): { value: string; name: string } | null {
  const option = findModelOption(configOptions);
  if (option === null || option.type !== "select") return null;
  return {
    value: option.currentValue,
    name: currentValueName(option) ?? option.currentValue,
  };
}
