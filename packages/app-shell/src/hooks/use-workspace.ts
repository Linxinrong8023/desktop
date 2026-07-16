import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  ContractsClient,
  Project,
  Session,
  SessionStatus,
  Task,
  TaskStatus,
} from "@ora/contracts";
import { useTranslation } from "react-i18next";

export interface WorkspaceSelection {
  projectId: string | null;
  taskId: string | null;
  sessionId: string | null;
}

export interface WorkspaceData {
  projects: Project[];
  tasks: Task[];
  sessions: Session[];
  selection: WorkspaceSelection;
  loading: boolean;
  error: string | null;
  selectProject: (projectId: string) => void;
  selectTask: (taskId: string) => void;
  selectSession: (sessionId: string) => void;
  createProject: (name: string, rootPath: string) => Promise<void>;
  updateProject: (project: Project, name: string, rootPath: string) => Promise<void>;
  deleteProject: (projectId: string) => Promise<void>;
  createTask: (projectId: string, title: string, status: TaskStatus) => Promise<void>;
  updateTask: (task: Task, title: string, status: TaskStatus) => Promise<void>;
  deleteTask: (taskId: string) => Promise<void>;
  createSession: (taskId: string, agentId: string, status: SessionStatus) => Promise<void>;
  updateSession: (session: Session, agentId: string, status: SessionStatus) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
}

const EMPTY_SELECTION: WorkspaceSelection = { projectId: null, taskId: null, sessionId: null };

/** Owns the project tree and routes every mutation through the injected contracts client. */
export function useWorkspace(client: ContractsClient): WorkspaceData {
  const { t } = useTranslation();
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selection, setSelection] = useState<WorkspaceSelection>(EMPTY_SELECTION);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const runMutation = useCallback(async (mutation: () => Promise<void>) => {
    setError(null);
    try {
      await mutation();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : t("error.workspaceRequest"));
      throw cause;
    }
  }, [t]);

  useEffect(() => {
    let active = true;
    Promise.all([client.project.list({}), client.task.list({}), client.session.list({})])
      .then(([projectResponse, taskResponse, sessionResponse]) => {
        if (!active) return;
        setProjects(projectResponse.projects);
        setTasks(taskResponse.tasks);
        setSessions(sessionResponse.sessions);
        const firstProject = projectResponse.projects[0];
        if (firstProject) setSelection({ projectId: firstProject.id, taskId: null, sessionId: null });
      })
      .catch((cause: unknown) => {
        if (active) setError(cause instanceof Error ? cause.message : t("error.workspaceLoad"));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [client, t]);

  const selectProject = useCallback((projectId: string) => {
    setSelection({ projectId, taskId: null, sessionId: null });
  }, []);

  const selectTask = useCallback((taskId: string) => {
    const task = tasks.find((candidate) => candidate.id === taskId);
    if (task) setSelection({ projectId: task.projectId, taskId, sessionId: null });
  }, [tasks]);

  const selectSession = useCallback((sessionId: string) => {
    const session = sessions.find((candidate) => candidate.id === sessionId);
    const task = session ? tasks.find((candidate) => candidate.id === session.taskId) : undefined;
    if (session && task) setSelection({ projectId: task.projectId, taskId: task.id, sessionId });
  }, [sessions, tasks]);

  const createProject = useCallback(async (name: string, rootPath: string) => {
    await runMutation(async () => {
      const { project } = await client.project.create({ name, rootPath });
      setProjects((current) => [...current, project]);
      setSelection({ projectId: project.id, taskId: null, sessionId: null });
    });
  }, [client, runMutation]);

  const updateProject = useCallback(async (current: Project, name: string, rootPath: string) => {
    await runMutation(async () => {
      const { project } = await client.project.update({ projectId: current.id, name, rootPath });
      setProjects((items) => items.map((item) => item.id === project.id ? project : item));
    });
  }, [client, runMutation]);

  const deleteSession = useCallback(async (sessionId: string) => {
    await runMutation(async () => {
      await client.session.delete({ sessionId });
      setSessions((items) => items.filter((item) => item.id !== sessionId));
      setSelection((current) => current.sessionId === sessionId ? { ...current, sessionId: null } : current);
    });
  }, [client, runMutation]);

  const deleteTask = useCallback(async (taskId: string) => {
    await runMutation(async () => {
      const childSessions = sessions.filter((session) => session.taskId === taskId);
      await Promise.all(childSessions.map((session) => client.session.delete({ sessionId: session.id })));
      await client.task.delete({ taskId });
      setSessions((items) => items.filter((item) => item.taskId !== taskId));
      setTasks((items) => items.filter((item) => item.id !== taskId));
      setSelection((current) => current.taskId === taskId
        ? { projectId: current.projectId, taskId: null, sessionId: null }
        : current);
    });
  }, [client, runMutation, sessions]);

  const deleteProject = useCallback(async (projectId: string) => {
    await runMutation(async () => {
      const childTasks = tasks.filter((task) => task.projectId === projectId);
      const childTaskIds = new Set(childTasks.map((task) => task.id));
      const childSessions = sessions.filter((session) => childTaskIds.has(session.taskId));
      await Promise.all(childSessions.map((session) => client.session.delete({ sessionId: session.id })));
      await Promise.all(childTasks.map((task) => client.task.delete({ taskId: task.id })));
      await client.project.delete({ projectId });
      setSessions((items) => items.filter((item) => !childTaskIds.has(item.taskId)));
      setTasks((items) => items.filter((item) => item.projectId !== projectId));
      setProjects((items) => items.filter((item) => item.id !== projectId));
      const nextProject = projects.find((project) => project.id !== projectId);
      setSelection((current) => current.projectId === projectId
        ? { projectId: nextProject?.id ?? null, taskId: null, sessionId: null }
        : current);
    });
  }, [client, projects, runMutation, sessions, tasks]);

  const createTask = useCallback(async (projectId: string, title: string, status: TaskStatus) => {
    await runMutation(async () => {
      const { task } = await client.task.create({ projectId, title, status });
      setTasks((items) => [...items, task]);
      setSelection({ projectId, taskId: task.id, sessionId: null });
    });
  }, [client, runMutation]);

  const updateTask = useCallback(async (current: Task, title: string, status: TaskStatus) => {
    await runMutation(async () => {
      const { task } = await client.task.update({ taskId: current.id, projectId: current.projectId, title, status });
      setTasks((items) => items.map((item) => item.id === task.id ? task : item));
    });
  }, [client, runMutation]);

  const createSession = useCallback(async (taskId: string, agentId: string, status: SessionStatus) => {
    await runMutation(async () => {
      const { session } = await client.session.create({ taskId, agentId, agentSessionId: null, status });
      const task = tasks.find((candidate) => candidate.id === taskId);
      setSessions((items) => [...items, session]);
      if (task) setSelection({ projectId: task.projectId, taskId, sessionId: session.id });
    });
  }, [client, runMutation, tasks]);

  const updateSession = useCallback(async (current: Session, agentId: string, status: SessionStatus) => {
    await runMutation(async () => {
      const { session } = await client.session.update({
        sessionId: current.id,
        taskId: current.taskId,
        agentId,
        agentSessionId: current.agentSessionId,
        status,
      });
      setSessions((items) => items.map((item) => item.id === session.id ? session : item));
    });
  }, [client, runMutation]);

  return useMemo(() => ({
    projects, tasks, sessions, selection, loading, error,
    selectProject, selectTask, selectSession,
    createProject, updateProject, deleteProject,
    createTask, updateTask, deleteTask,
    createSession, updateSession, deleteSession,
  }), [
    projects, tasks, sessions, selection, loading, error,
    selectProject, selectTask, selectSession,
    createProject, updateProject, deleteProject,
    createTask, updateTask, deleteTask,
    createSession, updateSession, deleteSession,
  ]);
}
