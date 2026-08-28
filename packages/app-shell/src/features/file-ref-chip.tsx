import {
  composerFileChipTitle,
  composerFileLabel,
  composerFileLineRangeLabel,
  type ComposerFileAttrs,
} from "@ora/editor/composer";
import { IconFileDiff } from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import { useTaskChangesNavigation } from "./diff/task-changes-navigation-context";
import { navigateToFileRef } from "./file-ref-chip-navigation";
import { WorkspaceFileIcon } from "./files/workspace-file-visuals";
import "./file-ref-chip.css";

/**
 * Icon + basename + optional `L12-34` range: the inside of a workspace file
 * reference chip. Split from the wrappers so the composer's TipTap node view
 * and read-only chat history render byte-identical chip innards instead of two
 * drifting copies.
 */
export function FileRefChipContent({ attrs }: { attrs: ComposerFileAttrs }) {
  const kind = attrs.kind === "directory" ? "directory" : "file";
  const rangeLabel = composerFileLineRangeLabel(attrs);
  const diff = attrs.origin === "diff";
  return (
    <>
      {diff ? (
        <span className="composer-file-ref-icon" aria-hidden="true">
          <IconFileDiff
            className="composer-file-ref-icon composer-file-ref-diff-icon"
            stroke={2.25}
          />
        </span>
      ) : (
        <WorkspaceFileIcon
          path={attrs.path}
          kind={kind}
          className="composer-file-ref-icon"
        />
      )}
      <span className="composer-file-ref-label">
        <span className="composer-file-ref-name">
          {composerFileLabel(attrs)}
        </span>
        {rangeLabel !== null && (
          <span className="composer-file-ref-range">{rangeLabel}</span>
        )}
      </span>
    </>
  );
}

/**
 * Read-only chip for surfaces that only hold the sent prompt text. The composer
 * wraps the same content in a `NodeViewWrapper` instead, because an editable
 * chip also carries drag and node-selection behaviour.
 *
 * Clickable whenever a `TaskChangesNavigationContext` is in scope (task/project
 * conversations), so a user's own reference jumps to the same Files/Changes
 * location the equivalent assistant link would. Rendering the inert `<span>`
 * without that context keeps this chip usable from any surface that renders
 * sent history outside a review layout, rather than throwing on a missing
 * navigator.
 */
export function FileRefChip({ attrs }: { attrs: ComposerFileAttrs }) {
  const { t } = useTranslation();
  const navigation = useTaskChangesNavigation();
  const kind = attrs.kind === "directory" ? "directory" : "file";
  const dataAttrs = {
    "data-composer-file": attrs.path,
    "data-kind": kind,
    ...(attrs.startLine === undefined
      ? {}
      : { "data-start-line": String(attrs.startLine) }),
    ...(attrs.endLine === undefined
      ? {}
      : { "data-end-line": String(attrs.endLine) }),
  };
  const title = composerFileChipTitle(attrs);

  if (navigation === null) {
    return (
      <span className="composer-file-ref" {...dataAttrs} title={title}>
        <FileRefChipContent attrs={attrs} />
      </span>
    );
  }

  return (
    <button
      type="button"
      className="composer-file-ref"
      data-navigable="true"
      {...dataAttrs}
      title={title}
      aria-label={t(
        kind === "directory"
          ? "chat.fileLink.openFolderAria"
          : "chat.fileLink.aria",
        { path: attrs.path },
      )}
      onClick={() => navigateToFileRef(attrs, navigation)}
    >
      <FileRefChipContent attrs={attrs} />
    </button>
  );
}
