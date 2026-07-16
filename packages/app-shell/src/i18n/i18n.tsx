import type { ReactNode } from "react";
import { createInstance } from "i18next";
import { I18nextProvider, initReactI18next } from "react-i18next";

export type Locale = "zh-CN" | "en-US";

const translations = {
  "zh-CN": {
    "common.cancel": "取消",
    "common.saving": "保存中…",
    "common.delete": "删除",
    "common.edit": "重命名 / 编辑",
    "common.settings": "设置",
    "common.running": "运行中",
    "common.stopped": "已停止",
    "common.todo": "待处理",
    "common.doing": "进行中",
    "common.done": "已完成",
    "sidebar.collapse": "收起侧边栏",
    "sidebar.expand": "展开侧边栏",
    "sidebar.search": "搜索工作区",
    "sidebar.clearSearch": "清除搜索",
    "sidebar.newProject": "新建项目",
    "sidebar.workspace": "工作区",
    "sidebar.projectCount": "{{count}} 个项目",
    "sidebar.loading": "正在加载工作区…",
    "sidebar.empty": "未找到项目。",
    "sidebar.newWorktree": "新建工作树",
    "sidebar.newSession": "新建会话",
    "sidebar.console": "控制台",
    "sidebar.openActions": "打开操作菜单",
    "account.label": "{{name}} 的账户",
    "account.logout": "退出登录",
    "account.language": "语言",
    "account.switchEnglish": "English",
    "account.switchChinese": "简体中文",
    "dialog.addProject": "添加项目",
    "dialog.editProject": "编辑项目",
    "dialog.projectDescription": "将代码仓库连接到 Ora 工作区。",
    "dialog.saveProject": "保存项目",
    "dialog.projectName": "项目名称",
    "dialog.projectNamePlaceholder": "Ora Desktop",
    "dialog.repositoryPath": "仓库路径",
    "dialog.editWorktree": "编辑工作树",
    "dialog.createWorktree": "创建工作树任务",
    "dialog.worktreeDescription": "任务对应独立工作树，让 Agent 可以专注处理一项工作。",
    "dialog.saveTask": "保存任务",
    "dialog.createTask": "创建任务",
    "dialog.taskTitle": "任务标题",
    "dialog.taskPlaceholder": "实现命令面板",
    "dialog.status": "状态",
    "dialog.required": "请填写所有必填字段。",
    "dialog.editSession": "编辑会话",
    "dialog.startSession": "启动 Agent 会话",
    "dialog.sessionDescription": "选择 Agent 标识和会话运行状态。",
    "dialog.saveSession": "保存会话",
    "dialog.agent": "Agent",
    "delete.title": "删除“{{name}}”？",
    "delete.projectDescription": "该项目下的所有工作树和会话也会被删除，此操作无法撤销。",
    "delete.taskDescription": "该工作树任务下的所有会话也会被删除，此操作无法撤销。",
    "delete.sessionDescription": "该 Agent 会话将被永久删除，此操作无法撤销。",
    "delete.deleting": "删除中…",
    "workspace.overview": "工作区概览",
    "workspace.defaultTitle": "你的 Agent 工作区",
    "workspace.taskHint": "从侧边栏选择已有会话，或从任务菜单启动一个新会话。",
    "workspace.projectHint": "选择一个工作树任务查看会话，或从项目菜单创建新任务。",
    "workspace.emptyHint": "添加项目后即可组织工作树与 Agent 会话。",
    "workspace.repository": "代码仓库",
    "workspace.agentSessions": "Agent 会话",
    "workspace.sessionCount": "{{count}} 个会话",
    "workspace.worktreeCount": "{{count}} 个工作树",
    "chat.new": "新建对话",
    "chat.heading": "今天想让我帮你做什么？",
    "chat.subheading": "可以直接提问，也可以从下面的建议开始。",
    "chat.placeholder": "给 Ora 发送消息…",
    "chat.sendHint": "Enter 发送 / Shift+Enter 换行",
    "chat.send": "发送消息",
    "chat.typing": "助手正在输入",
    "chat.copy": "复制",
    "chat.goodResponse": "回复有帮助",
    "chat.badResponse": "回复没有帮助",
    "chat.youSaid": "你说",
    "chat.assistantReplied": "助手回复",
    "chat.suggestion.runtime": "总结 Agent Runtime 重构进展",
    "chat.suggestion.layout": "设计 Web 客户端布局",
    "chat.suggestion.worktree": "解释工作树清理机制",
    "chat.suggestion.testing": "整理会话连接测试计划",
    "error.workspaceRequest": "工作区请求失败。",
    "error.workspaceLoad": "无法加载工作区数据。",
  },
  "en-US": {
    "common.cancel": "Cancel",
    "common.saving": "Saving...",
    "common.delete": "Delete",
    "common.edit": "Rename / edit",
    "common.settings": "Settings",
    "common.running": "Running",
    "common.stopped": "Stopped",
    "common.todo": "To do",
    "common.doing": "In progress",
    "common.done": "Done",
    "sidebar.collapse": "Collapse sidebar",
    "sidebar.expand": "Expand sidebar",
    "sidebar.search": "Search workspace",
    "sidebar.clearSearch": "Clear search",
    "sidebar.newProject": "New project",
    "sidebar.workspace": "Workspace",
    "sidebar.projectCount": "{{count}} projects",
    "sidebar.loading": "Loading workspace...",
    "sidebar.empty": "No projects found.",
    "sidebar.newWorktree": "New worktree",
    "sidebar.newSession": "New session",
    "sidebar.console": "Console",
    "sidebar.openActions": "Open actions",
    "account.label": "{{name}} account",
    "account.logout": "Log out",
    "account.language": "Language",
    "account.switchEnglish": "English",
    "account.switchChinese": "简体中文",
    "dialog.addProject": "Add project",
    "dialog.editProject": "Edit project",
    "dialog.projectDescription": "Connect a repository to the Ora workspace.",
    "dialog.saveProject": "Save project",
    "dialog.projectName": "Project name",
    "dialog.projectNamePlaceholder": "Ora Desktop",
    "dialog.repositoryPath": "Repository path",
    "dialog.editWorktree": "Edit worktree",
    "dialog.createWorktree": "Create worktree task",
    "dialog.worktreeDescription": "Tasks represent isolated worktrees where agents perform focused work.",
    "dialog.saveTask": "Save task",
    "dialog.createTask": "Create task",
    "dialog.taskTitle": "Task title",
    "dialog.taskPlaceholder": "Implement command palette",
    "dialog.status": "Status",
    "dialog.required": "Complete all required fields.",
    "dialog.editSession": "Edit session",
    "dialog.startSession": "Start agent session",
    "dialog.sessionDescription": "Choose the agent identity and the session lifecycle state.",
    "dialog.saveSession": "Save session",
    "dialog.agent": "Agent",
    "delete.title": "Delete “{{name}}”?",
    "delete.projectDescription": "All worktrees and sessions in this project will also be deleted. This cannot be undone.",
    "delete.taskDescription": "All sessions in this worktree task will also be deleted. This cannot be undone.",
    "delete.sessionDescription": "This agent session will be permanently deleted. This cannot be undone.",
    "delete.deleting": "Deleting...",
    "workspace.overview": "Workspace overview",
    "workspace.defaultTitle": "Your agent workspace",
    "workspace.taskHint": "Select an existing session in the sidebar or start a new one from the task menu.",
    "workspace.projectHint": "Choose a worktree task to review its sessions, or create focused work from the project menu.",
    "workspace.emptyHint": "Add a project to organize worktrees and agent sessions.",
    "workspace.repository": "Repository",
    "workspace.agentSessions": "Agent sessions",
    "workspace.sessionCount": "{{count}} sessions",
    "workspace.worktreeCount": "{{count}} worktrees",
    "chat.new": "New chat",
    "chat.heading": "How can I help you today?",
    "chat.subheading": "Ask anything, or start from one of these.",
    "chat.placeholder": "Message Ora...",
    "chat.sendHint": "Enter to send / Shift+Enter for newline",
    "chat.send": "Send message",
    "chat.typing": "Assistant is typing",
    "chat.copy": "Copy",
    "chat.goodResponse": "Good response",
    "chat.badResponse": "Bad response",
    "chat.youSaid": "You said",
    "chat.assistantReplied": "Assistant replied",
    "chat.suggestion.runtime": "Summarize the agent runtime refactor",
    "chat.suggestion.layout": "Draft a layout for the web client",
    "chat.suggestion.worktree": "Explain how worktree cleanup works",
    "chat.suggestion.testing": "Outline a test plan for session attach",
    "error.workspaceRequest": "The workspace request failed.",
    "error.workspaceLoad": "Unable to load workspace data.",
  },
} as const;

export type TranslationKey = keyof typeof translations["zh-CN"];
const LOCALE_STORAGE_KEY = "ora.locale";

/** Reads the persisted locale without making application startup depend on browser storage availability. */
function readInitialLocale(): Locale {
  if (typeof window === "undefined") return "zh-CN";
  try {
    return window.localStorage.getItem(LOCALE_STORAGE_KEY) === "en-US" ? "en-US" : "zh-CN";
  } catch {
    return "zh-CN";
  }
}

export const appI18n = createInstance();
const initialLocale = readInitialLocale();

void appI18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: translations["zh-CN"] },
    "en-US": { translation: translations["en-US"] },
  },
  lng: initialLocale,
  fallbackLng: "zh-CN",
  supportedLngs: ["zh-CN", "en-US"],
  keySeparator: false,
  interpolation: { escapeValue: false },
  initAsync: false,
});

if (typeof document !== "undefined") document.documentElement.lang = initialLocale;

appI18n.on("languageChanged", (language) => {
  const locale: Locale = language === "en-US" ? "en-US" : "zh-CN";
  if (typeof document !== "undefined") document.documentElement.lang = locale;
  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(LOCALE_STORAGE_KEY, locale);
    } catch {
      // Storage is an enhancement; language switching still works for the current runtime.
    }
  }
});

/** Binds the app-specific i18next instance without mutating a host application's global instance. */
export function AppI18nProvider({ children }: { children: ReactNode }) {
  return <I18nextProvider i18n={appI18n}>{children}</I18nextProvider>;
}
