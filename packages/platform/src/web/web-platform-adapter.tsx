import {
  LocalTransportError,
  type ContractsClient,
} from "@ora/contracts";
import { uploadSkillFolder } from "@ora/contracts/fetch";
import { type ReactNode } from "react";
import { renderPlatformHost, type PlatformHostRenderer } from "../platform-host-renderer";
import {
  PathSelectionInProgressError,
  type PlatformAdapter,
  type PlatformLocale,
  type SelectPathOptions,
} from "../types";
import { WebPathPickerHost } from "./web-path-picker-host";

interface ActivePathSelection {
  requestId: number;
  options: SelectPathOptions;
  restoreFocusTo: HTMLElement | null;
  resolve: (path: string | null) => void;
}

type SelectSkillFolderFiles = () => Promise<readonly File[] | null>;
type UploadSkillFolder = typeof uploadSkillFolder;


export type WebPlatformSnapshot =
  | { kind: "idle" }
  | { kind: "selecting"; requestId: number; options: SelectPathOptions };

/** Coordinates Promise-based platform calls with the React-owned Web path picker dialog. */
export class WebPlatformAdapter implements PlatformAdapter, PlatformHostRenderer {
  readonly worktreeStorage = { kind: "unsupported" as const };
  // The browser owns its own chrome, so the shell paints no window controls.
  readonly windowControls = { kind: "none" as const };
  // The browser cannot launch native file managers, terminals, or editors.
  readonly locationActions = { kind: "unsupported" as const };
  private activeSelection: ActivePathSelection | null = null;
  private listeners = new Set<() => void>();
  readonly skillFolderImport = {
    kind: "supported" as const,
    importFolder: async () => {
      const files = await this.selectSkillFolderFiles();
      if (files === null) return null;
      const response = await this.uploadSelectedSkillFolder(files.map((file) => ({
        relativePath: skillRootRelativePath(file),
        contents: file,
      })));
      return response.skill;
    },
  };

  private nextRequestId = 1;
  private snapshot: WebPlatformSnapshot = { kind: "idle" };

  constructor(
    readonly client: ContractsClient,
    private readonly selectSkillFolderFiles: SelectSkillFolderFiles = selectBrowserSkillFolder,
    private readonly uploadSelectedSkillFolder: UploadSkillFolder = uploadSkillFolder,
  ) {}

  /** Opens one Web path picker and resolves after the host confirms or cancels it. */
  selectPath(options: SelectPathOptions): Promise<string | null> {
    if (this.activeSelection !== null) {
      return Promise.reject(new PathSelectionInProgressError());
    }

    return new Promise((resolve) => {
      const requestId = this.nextRequestId;
      this.nextRequestId += 1;
      this.activeSelection = {
        requestId,
        options,
        restoreFocusTo:
          typeof document !== "undefined" && document.activeElement instanceof HTMLElement
            ? document.activeElement
            : null,
        resolve,
      };
      this.snapshot = { kind: "selecting", requestId, options };
      this.emitChange();
    });
  }

  /** Returns the stable external-store snapshot consumed by PlatformHost. */
  getSnapshot = (): WebPlatformSnapshot => this.snapshot;

  /** Subscribes one React host to selection state changes. */
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  /** Completes only the currently visible request so stale async UI cannot resolve a newer picker. */
  completeSelection(requestId: number, path: string | null): void {
    if (this.activeSelection?.requestId !== requestId) {
      return;
    }

    const { resolve, restoreFocusTo } = this.activeSelection;
    this.activeSelection = null;
    this.snapshot = { kind: "idle" };
    this.emitChange();
    resolve(path);
    queueMicrotask(() => restoreFocusTo?.focus());
  }

  /** Supplies the Web-only dialog host without exposing rendering on the public adapter interface. */
  [renderPlatformHost](locale: PlatformLocale): ReactNode {
    return <WebPathPickerHost adapter={this} locale={locale} />;
  }

  /** Notifies React after replacing the immutable external-store snapshot. */
  private emitChange(): void {
    for (const listener of this.listeners) {
      listener();
    }
  }
}

/** Opens the browser's native directory chooser and resolves its selected files or cancellation. */
function selectBrowserSkillFolder(): Promise<readonly File[] | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    input.webkitdirectory = true;
    input.hidden = true;
    document.body.append(input);

    let settled = false;
    let focusTimer: number | null = null;
    let removeFocusFallback = () => {};
    const complete = (files: readonly File[] | null) => {
      if (settled) return;
      settled = true;
      removeFocusFallback();
      input.remove();
      resolve(files);
    };

    input.addEventListener("change", () => {
      const files = input.files === null ? [] : Array.from(input.files);
      complete(files);
    }, { once: true });
    input.addEventListener("cancel", () => complete(null), { once: true });

    const onWindowFocus = () => {
      // Legacy browsers may omit the input cancel event. Wait briefly so a pending change event
      // wins when the user selected a directory, then settle an otherwise abandoned picker.
      focusTimer = window.setTimeout(() => complete(null), 300);
    };
    window.addEventListener("focus", onWindowFocus, { once: true });
    removeFocusFallback = () => {
      window.removeEventListener("focus", onWindowFocus);
      if (focusTimer !== null) window.clearTimeout(focusTimer);
    };

    input.click();
  });
}

/**
 * Removes the browser-added selected-directory segment so the backend receives paths relative
 * to the skill root, with SKILL.md at exactly the root.
 */
function skillRootRelativePath(file: File): string {
  const segments = file.webkitRelativePath.split("/").filter(Boolean);
  if (segments.length < 2) {
    throw new LocalTransportError(
      "malformed_response",
      "Browser directory selection did not provide a root-relative path",
      file.webkitRelativePath,
    );
  }
  return segments.slice(1).join("/");
}

/** Creates the Web platform adapter around the same contracts client injected into AppShell. */
export function createWebPlatformAdapter(client: ContractsClient): WebPlatformAdapter {
  return new WebPlatformAdapter(client);
}
