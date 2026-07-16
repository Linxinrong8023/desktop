import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Project, Session, SessionStatus, Task, TaskStatus } from "@ora/contracts";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Input,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@ora/ui";
import {
  IconChevronDown,
  IconChevronRight,
  IconDots,
  IconFolder,
  IconGitBranch,
  IconLayoutSidebarLeftCollapse,
  IconPlus,
  IconSearch,
  IconSparkles,
  IconTrash,
  IconX,
} from "@tabler/icons-react";
import type { CurrentUser } from "../../lib/types";
import type { WorkspaceData } from "../../hooks/use-workspace";
import { UserProfile } from "../sidebar/user-profile";
import { EntityDialog, type EntityField } from "./entity-dialog";

type DialogState =
  | { kind: "project"; entity?: Project }
  | { kind: "task"; projectId: string; entity?: Task }
  | { kind: "session"; taskId: string; entity?: Session };

type DeleteTarget =
  | { kind: "project"; id: string; name: string }
  | { kind: "task"; id: string; name: string }
  | { kind: "session"; id: string; name: string };

interface WorkspaceSidebarProps {
  user: CurrentUser;
  workspace: WorkspaceData;
  onCollapse: () => void;
  onSignOut: () => void;
}

/** Renders projects, worktree tasks, and agent sessions as a dense three-level navigation tree. */
export function WorkspaceSidebar({ user, workspace, onCollapse, onSignOut }: WorkspaceSidebarProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget | null>(null);
  const [expandedProjects, setExpandedProjects] = useState(() => new Set<string>());
  const [expandedTasks, setExpandedTasks] = useState(() => new Set<string>());
  const needle = query.trim().toLowerCase();

  const visibleProjects = useMemo(() => workspace.projects.filter((project) => {
    if (!needle) return true;
    const projectTasks = workspace.tasks.filter((task) => task.projectId === project.id);
    return project.name.toLowerCase().includes(needle)
      || projectTasks.some((task) => task.title.toLowerCase().includes(needle)
        || workspace.sessions.some((session) => session.taskId === task.id && session.agentId.toLowerCase().includes(needle)));
  }), [needle, workspace.projects, workspace.sessions, workspace.tasks]);

  // Mutations select their new child. Expand its ancestors once without preventing a later manual collapse.
  useEffect(() => {
    const projectId = workspace.selection.projectId;
    const taskId = workspace.selection.taskId;
    if (taskId) setExpandedTasks((current) => current.has(taskId) ? current : new Set(current).add(taskId));
    if (projectId) setExpandedProjects((current) => current.has(projectId) ? current : new Set(current).add(projectId));
  }, [workspace.selection.projectId, workspace.selection.taskId]);

  const toggleSet = (setter: React.Dispatch<React.SetStateAction<Set<string>>>, id: string) => {
    setter((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const openProject = (projectId: string) => {
    toggleSet(setExpandedProjects, projectId);
    workspace.selectProject(projectId);
  };

  const openTask = (taskId: string) => {
    toggleSet(setExpandedTasks, taskId);
    workspace.selectTask(taskId);
  };

  return (
    <>
      <aside className="flex h-dvh w-[320px] shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
        <header className="flex h-12 items-center gap-2 border-b border-sidebar-border px-3">
          <div className="flex size-7 items-center justify-center rounded-md bg-foreground text-background">
            <IconSparkles className="size-4" />
          </div>
          <span className="text-sm font-semibold">Ora Agent</span>
          <div className="flex-1" />
          <Tooltip>
            <TooltipTrigger render={<Button variant="ghost" size="icon-sm" onClick={onCollapse} aria-label={t("sidebar.collapse")} />}>
              <IconLayoutSidebarLeftCollapse />
            </TooltipTrigger>
            <TooltipContent>{t("sidebar.collapse")}</TooltipContent>
          </Tooltip>
        </header>

        <div className="flex items-center gap-2 px-3 py-3">
          <div className="relative min-w-0 flex-1">
            <IconSearch className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("sidebar.search")}
              className="h-8 bg-background px-7 text-xs"
            />
            {query && (
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                className="absolute right-1 top-1/2 -translate-y-1/2"
                aria-label={t("sidebar.clearSearch")}
                onClick={() => setQuery("")}
              >
                <IconX />
              </Button>
            )}
          </div>
          <Tooltip>
            <TooltipTrigger render={<Button size="icon" onClick={() => setDialog({ kind: "project" })} aria-label={t("sidebar.newProject")} />}>
              <IconPlus />
            </TooltipTrigger>
            <TooltipContent>{t("sidebar.newProject")}</TooltipContent>
          </Tooltip>
        </div>

        <div className="flex items-center px-3 pb-1 text-[11px] font-semibold uppercase text-muted-foreground">
          <span>{t("sidebar.workspace")}</span><span className="ml-auto">{t("sidebar.projectCount", { count: workspace.projects.length })}</span>
        </div>

        <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
          {workspace.loading && <p className="px-2 py-6 text-center text-xs text-muted-foreground">{t("sidebar.loading")}</p>}
          {!workspace.loading && visibleProjects.length === 0 && (
            <p className="px-2 py-6 text-center text-xs text-muted-foreground">{t("sidebar.empty")}</p>
          )}
          {visibleProjects.map((project) => {
            const projectTasks = workspace.tasks.filter((task) => task.projectId === project.id);
            const projectOpen = expandedProjects.has(project.id) || Boolean(needle);
            return (
              <div key={project.id} className="mt-1">
                <TreeRow
                  depth={0}
                  active={workspace.selection.projectId === project.id && workspace.selection.taskId === null}
                  icon={<IconFolder className="size-4 text-amber-600" />}
                  label={project.name}
                  meta={`${projectTasks.length}`}
                  expanded={projectOpen}
                  onClick={() => openProject(project.id)}
                  menu={(
                    <EntityMenu
                      onAdd={() => setDialog({ kind: "task", projectId: project.id })}
                      addLabel={t("sidebar.newWorktree")}
                      onEdit={() => setDialog({ kind: "project", entity: project })}
                      onDelete={() => setDeleteTarget({ kind: "project", id: project.id, name: project.name })}
                    />
                  )}
                />
                {projectOpen && projectTasks.map((task) => {
                  const taskSessions = workspace.sessions.filter((session) => session.taskId === task.id);
                  const taskOpen = expandedTasks.has(task.id) || Boolean(needle);
                  return (
                    <div key={task.id}>
                      <TreeRow
                        depth={1}
                        active={workspace.selection.taskId === task.id && workspace.selection.sessionId === null}
                        icon={<IconGitBranch className="size-3.5 text-sky-600" />}
                        label={task.title}
                        meta={t(`common.${task.status}`)}
                        expanded={taskOpen}
                        onClick={() => openTask(task.id)}
                        menu={(
                          <EntityMenu
                            onAdd={() => setDialog({ kind: "session", taskId: task.id })}
                            addLabel={t("sidebar.newSession")}
                            onEdit={() => setDialog({ kind: "task", projectId: project.id, entity: task })}
                            onDelete={() => setDeleteTarget({ kind: "task", id: task.id, name: task.title })}
                          />
                        )}
                      />
                      {taskOpen && taskSessions.map((session) => (
                        <TreeRow
                          key={session.id}
                          depth={2}
                          active={workspace.selection.sessionId === session.id}
                          icon={<span className={`size-2 rounded-full ${session.status === "running" ? "bg-emerald-500" : "bg-zinc-400"}`} />}
                          label={session.agentId}
                          meta={t(`common.${session.status}`)}
                          onClick={() => workspace.selectSession(session.id)}
                          menu={(
                            <EntityMenu
                              onEdit={() => setDialog({ kind: "session", taskId: task.id, entity: session })}
                              onDelete={() => setDeleteTarget({ kind: "session", id: session.id, name: session.agentId })}
                            />
                          )}
                        />
                      ))}
                    </div>
                  );
                })}
              </div>
            );
          })}
        </nav>

        {workspace.error && <p className="border-t border-destructive/20 bg-destructive/10 px-3 py-2 text-xs text-destructive">{workspace.error}</p>}
        <div className="border-t border-sidebar-border p-2">
          <UserProfile user={user} onSignOut={onSignOut} />
        </div>
      </aside>
      {dialog && (
        <WorkspaceDialog dialog={dialog} workspace={workspace} onOpenChange={(open) => !open && setDialog(null)} />
      )}
      <DeleteEntityDialog target={deleteTarget} workspace={workspace} onOpenChange={(open) => !open && setDeleteTarget(null)} />
    </>
  );
}

interface TreeRowProps {
  depth: 0 | 1 | 2;
  active: boolean;
  icon: React.ReactNode;
  label: string;
  meta: string;
  expanded?: boolean;
  onClick: () => void;
  menu: React.ReactNode;
}

/** Keeps every tree level aligned while preserving a stable row width for actions. */
function TreeRow({ depth, active, icon, label, meta, expanded, onClick, menu }: TreeRowProps) {
  return (
    <div className={`group/tree flex h-8 items-center rounded-md ${active ? "bg-sidebar-accent text-sidebar-accent-foreground" : "hover:bg-sidebar-accent/60"}`}>
      <button
        type="button"
        onClick={onClick}
        aria-expanded={expanded}
        className="flex h-full min-w-0 flex-1 items-center gap-1.5 rounded-md text-left text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
        style={{ paddingLeft: `${6 + depth * 16}px` }}
      >
        <span className="relative flex size-4 shrink-0 items-center justify-center">
          <span className={`flex items-center justify-center transition-opacity duration-100 ${expanded === undefined ? "" : "group-hover/tree:opacity-0"}`}>{icon}</span>
          {expanded !== undefined && (expanded
            ? <IconChevronDown className="absolute size-3.5 opacity-0 transition-opacity duration-100 group-hover/tree:opacity-100" />
            : <IconChevronRight className="absolute size-3.5 opacity-0 transition-opacity duration-100 group-hover/tree:opacity-100" />)}
        </span>
        <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
        <span className="truncate text-[10px] text-muted-foreground">{meta}</span>
      </button>
      <div className="mr-1 opacity-0 transition-opacity duration-100 group-hover/tree:opacity-100 group-focus-within/tree:opacity-100">{menu}</div>
    </div>
  );
}

/** Confirms destructive tree mutations and prevents duplicate requests while cascading deletes run. */
function DeleteEntityDialog({ target, workspace, onOpenChange }: { target: DeleteTarget | null; workspace: WorkspaceData; onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation();
  const [deleting, setDeleting] = useState(false);

  const confirmDelete = async () => {
    if (!target || deleting) return;
    setDeleting(true);
    try {
      if (target.kind === "project") await workspace.deleteProject(target.id);
      if (target.kind === "task") await workspace.deleteTask(target.id);
      if (target.kind === "session") await workspace.deleteSession(target.id);
      onOpenChange(false);
    } catch {
      // The workspace error banner retains the transport error while the dialog stays open for retry.
    } finally {
      setDeleting(false);
    }
  };

  return (
    <AlertDialog open={target !== null} onOpenChange={(open) => !deleting && onOpenChange(open)}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("delete.title", { name: target?.name ?? "" })}</AlertDialogTitle>
          <AlertDialogDescription>{target ? t(`delete.${target.kind}Description`) : ""}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>{t("common.cancel")}</AlertDialogCancel>
          <AlertDialogAction variant="destructive" disabled={deleting} onClick={() => void confirmDelete()}>
            <IconTrash />{deleting ? t("delete.deleting") : t("common.delete")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/** Provides contextual CRUD commands without making every tree row visually noisy. */
function EntityMenu({ onAdd, addLabel, onEdit, onDelete }: { onAdd?: () => void; addLabel?: string; onEdit: () => void; onDelete: () => void }) {
  const { t } = useTranslation();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" aria-label={t("sidebar.openActions")} onClick={(event) => event.stopPropagation()} />}>
        <IconDots />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        {onAdd && <DropdownMenuItem onClick={onAdd}><IconPlus />{addLabel}</DropdownMenuItem>}
        {onAdd && <DropdownMenuSeparator />}
        <DropdownMenuItem onClick={onEdit}>{t("common.edit")}</DropdownMenuItem>
        <DropdownMenuItem variant="destructive" onClick={onDelete}>{t("common.delete")}</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Adapts the generic entity form to the selected workspace entity and mutation. */
function WorkspaceDialog({ dialog, workspace, onOpenChange }: { dialog: DialogState; workspace: WorkspaceData; onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation();
  let title: string;
  let description: string;
  let fields: EntityField[];
  let submitLabel: string;
  let submit: (values: Record<string, string>) => Promise<void>;

  if (dialog.kind === "project") {
    title = dialog.entity ? t("dialog.editProject") : t("dialog.addProject");
    description = t("dialog.projectDescription");
    submitLabel = dialog.entity ? t("dialog.saveProject") : t("dialog.addProject");
    fields = [
      { name: "name", label: t("dialog.projectName"), value: dialog.entity?.name ?? "", placeholder: t("dialog.projectNamePlaceholder") },
      { name: "rootPath", label: t("dialog.repositoryPath"), value: dialog.entity?.rootPath ?? "", placeholder: "C:\\workspace\\project" },
    ];
    submit = (values) => dialog.entity
      ? workspace.updateProject(dialog.entity, values.name!, values.rootPath!)
      : workspace.createProject(values.name!, values.rootPath!);
  } else if (dialog.kind === "task") {
    title = dialog.entity ? t("dialog.editWorktree") : t("dialog.createWorktree");
    description = t("dialog.worktreeDescription");
    submitLabel = dialog.entity ? t("dialog.saveTask") : t("dialog.createTask");
    fields = [
      { name: "title", label: t("dialog.taskTitle"), value: dialog.entity?.title ?? "", placeholder: t("dialog.taskPlaceholder") },
      { name: "status", label: t("dialog.status"), value: dialog.entity?.status ?? "todo", options: [
        { label: t("common.todo"), value: "todo" }, { label: t("common.doing"), value: "doing" }, { label: t("common.done"), value: "done" },
      ] },
    ];
    submit = (values) => dialog.entity
      ? workspace.updateTask(dialog.entity, values.title!, values.status as TaskStatus)
      : workspace.createTask(dialog.projectId, values.title!, values.status as TaskStatus);
  } else {
    title = dialog.entity ? t("dialog.editSession") : t("dialog.startSession");
    description = t("dialog.sessionDescription");
    submitLabel = dialog.entity ? t("dialog.saveSession") : t("dialog.startSession");
    fields = [
      { name: "agentId", label: t("dialog.agent"), value: dialog.entity?.agentId ?? "codex", placeholder: "codex" },
      { name: "status", label: t("dialog.status"), value: dialog.entity?.status ?? "running", options: [
        { label: t("common.running"), value: "running" }, { label: t("common.stopped"), value: "stopped" },
      ] },
    ];
    submit = (values) => dialog.entity
      ? workspace.updateSession(dialog.entity, values.agentId!, values.status as SessionStatus)
      : workspace.createSession(dialog.taskId, values.agentId!, values.status as SessionStatus);
  }

  return <EntityDialog open title={title} description={description} submitLabel={submitLabel} fields={fields} onOpenChange={onOpenChange} onSubmit={submit} />;
}
