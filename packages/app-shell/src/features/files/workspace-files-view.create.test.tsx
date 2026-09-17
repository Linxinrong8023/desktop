import type { ReactNode } from "react";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  QueryClient,
  QueryClientProvider,
  QueryObserver,
} from "@tanstack/react-query";
import type { WorkspaceEntry } from "@ora/contracts";
import { expect, it } from "vitest";
import { AppI18nProvider } from "../../i18n/i18n";
import "../../i18n/i18n-instance";
import { ContractsClientContext } from "../../contracts-client-context";
import {
  createTestClient,
  type OperationHandler,
} from "../../test/contracts-transport";
import { emptyFilesHandlers } from "../../test/memory/files";
import {
  directoryQueryKey,
  fileQueryKey,
  searchQueryKey,
  filesScopeApi,
  type FilesScope,
} from "../../state/data/files";
import { PlatformProvider } from "../../platform";
import { createStubPlatform } from "../../test/stub-platform";
import { WorkspaceFilesView } from "./workspace-files-view";

const scopes: FilesScope[] = [
  { kind: "task", taskId: "same-id" },
  { kind: "project", projectId: "same-id" },
  { kind: "task", taskId: "neighbor" },
  { kind: "project", projectId: "neighbor" },
];

it.each([
  { scope: scopes[0]!, kind: "file" as const },
  { scope: scopes[0]!, kind: "directory" as const },
  { scope: scopes[1]!, kind: "file" as const },
  { scope: scopes[1]!, kind: "directory" as const },
])(
  "refreshes $scope.kind caches after creating a $kind before watcher delivery",
  async ({ scope, kind }) => {
    const user = userEvent.setup();
    const original: WorkspaceEntry = {
      name: "README.md",
      path: "README.md",
      kind: "file",
      isSymbolicLink: false,
    };
    const created: WorkspaceEntry = {
      name: "notes",
      path: "notes",
      kind,
      isSymbolicLink: false,
    };
    let entries = [original];
    const requests: unknown[] = [];
    const createEntry: OperationHandler<"createWorkspaceEntry"> = (request) => {
      requests.push(request);
      entries = [original, created];
      return created;
    };
    // Hold both watcher streams open without delivering a batch.
    const watch: OperationHandler<"watchWorkspace"> = async function* (
      _,
      options,
    ) {
      const signal = options?.signal;
      if (!signal) throw new Error("Expected a cancellable watcher");
      await new Promise<void>((resolve) => {
        if (signal.aborted) resolve();
        else signal.addEventListener("abort", () => resolve(), { once: true });
      });
      yield* [];
    };
    const client = createTestClient({
      ...emptyFilesHandlers(),
      listWorkspaces: () => ({ workspaces: [] }),
      getTaskWorkspace: () => ({
        workspace: { rootPath: "/repo", branchName: "task/test" },
      }),
      listWorkspaceDirectory: ({ path }) => ({
        path: path ?? "",
        entries: path ? [] : entries,
      }),
      listProjectDirectory: ({ path }) => ({
        path: path ?? "",
        entries: path ? [] : entries,
      }),
      createWorkspaceEntry: createEntry,
      createProjectEntry: (request) => {
        requests.push(request);
        entries = [original, created];
        return created;
      },
      watchWorkspace: watch,
      watchProject: (_, options) => watch({ taskId: "unused" }, options),
      searchWorkspace: () => ({
        results: entries.map(({ path }) => ({ kind: "file", path })),
        truncated: false,
      }),
      searchProject: () => ({
        results: entries.map(({ path }) => ({ kind: "file", path })),
        truncated: false,
      }),
    });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false, staleTime: Infinity } },
    });
    const staleFile = {
      path: "notes",
      content: "deleted old content",
      version: "old",
      sizeBytes: 19,
    };
    for (const cachedScope of scopes) {
      queryClient.setQueryData(fileQueryKey(cachedScope, "notes"), staleFile);
      queryClient.setQueryData(directoryQueryKey(cachedScope, "notes"), {
        path: "notes",
        entries: [original],
      });
      for (const searchKind of ["files", "content"] as const) {
        queryClient.setQueryData(
          searchQueryKey(cachedScope, searchKind, "notes"),
          { results: [], truncated: false },
        );
      }
    }
    const searchKey = searchQueryKey(scope, "files", "notes");
    const observer = new QueryObserver(queryClient, {
      queryKey: searchKey,
      queryFn: () => filesScopeApi(client, scope).search("notes", "files"),
    });
    const unsubscribe = observer.subscribe(() => {});
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>
        <ContractsClientContext.Provider value={client}>
          <AppI18nProvider>
            <PlatformProvider adapter={createStubPlatform()}>
              {children}
            </PlatformProvider>
          </AppI18nProvider>
        </ContractsClientContext.Provider>
      </QueryClientProvider>
    );
    const view = render(
      <WorkspaceFilesView
        projectId="same-id"
        taskId={scope.kind === "task" ? scope.taskId : undefined}
        hideHeader
      />,
      { wrapper },
    );
    try {
      const row = await screen.findByRole("button", { name: /README.md/ });
      await user.pointer({ keys: "[MouseRight>]", target: row });
      const label =
        kind === "file" ? /^新建文件$|^New File$/ : /^新建文件夹$|^New Folder$/;
      await user.click(await screen.findByRole("menuitem", { name: label }));
      await user.type(
        await screen.findByRole("textbox", { name: label }),
        "notes{Enter}",
      );
      await screen.findByRole("button", { name: /notes/ });
      await waitFor(() => {
        expect(queryClient.getQueryData(searchKey)).toEqual({
          results: entries.map(({ path }) => ({ kind: "file", path })),
          truncated: false,
        });
        expect(
          queryClient.getQueryState(searchQueryKey(scope, "content", "notes"))
            ?.isInvalidated,
        ).toBe(true);
        if (kind === "file") {
          expect(
            queryClient.getQueryData(fileQueryKey(scope, "notes")),
          ).toEqual({
            path: "notes",
            content: "",
            version: "test",
            sizeBytes: 0,
          });
        } else {
          expect(
            queryClient.getQueryData(directoryQueryKey(scope, "notes")),
          ).toEqual({ path: "notes", entries: [] });
          expect(
            queryClient.getQueryState(fileQueryKey(scope, "notes"))
              ?.isInvalidated,
          ).toBe(true);
        }
      });
      expect(requests).toEqual([
        {
          ...(scope.kind === "task"
            ? { taskId: scope.taskId }
            : { projectId: scope.projectId }),
          path: "notes",
          kind,
        },
      ]);
      for (const other of scopes.filter((candidate) => candidate !== scope)) {
        const keys = [
          fileQueryKey(other, "notes"),
          directoryQueryKey(other, "notes"),
          searchQueryKey(other, "files", "notes"),
          searchQueryKey(other, "content", "notes"),
        ];
        expect(
          keys.map((key) => queryClient.getQueryState(key)?.isInvalidated),
        ).toEqual([false, false, false, false]);
        expect(queryClient.getQueryData(keys[0]!)).toEqual(staleFile);
      }
    } finally {
      await act(async () => view.unmount());
      unsubscribe();
      queryClient.clear();
    }
  },
);
