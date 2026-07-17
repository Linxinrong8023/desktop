import type { Cursor, Meta, SessionId } from "./common.js";
import type { McpServer } from "./mcp.js";

export type PatchField<T> =
  | { kind: "absent" }
  | { kind: "set"; value: T }
  | { kind: "clear" };

export interface SessionEnvironment {
  cwd: string;
  mcpServers: McpServer[];
  additionalDirectories?: string[];
}

export interface NewSessionRequest extends SessionEnvironment {
  _meta?: Meta;
}

export interface NewSessionResponse {
  sessionId: SessionId;
  _meta?: Meta;
}

export interface LoadSessionRequest extends SessionEnvironment {
  sessionId: SessionId;
  _meta?: Meta;
}

export interface LoadSessionResponse {}

export interface ResumeSessionRequest {
  sessionId: SessionId;
  cwd: string;
  mcpServers?: McpServer[];
  additionalDirectories?: string[];
  _meta?: Meta;
}

export interface ResumeSessionResponse {}

export interface CloseSessionRequest {
  sessionId: SessionId;
  _meta?: Meta;
}

export interface CloseSessionResponse {}

export interface DeleteSessionRequest {
  sessionId: SessionId;
  _meta?: Meta;
}

export interface DeleteSessionResponse {}

export interface CancelSessionNotification {
  sessionId: SessionId;
  _meta?: Meta;
}

export type SessionUpdateType = "session_info_update";

export interface SessionInfoUpdate {
  sessionUpdate: SessionUpdateType;
  title: PatchField<string>;
  updatedAt: PatchField<string>;
  _meta?: Meta;
}

export type SessionUpdate = SessionInfoUpdate;

export interface SessionUpdateNotification {
  sessionId: SessionId;
  update: SessionUpdate;
  _meta?: Meta;
}

export interface ListSessionsRequest {
  cwd?: string;
  cursor?: Cursor;
  _meta?: Meta;
}

export interface ListSessionsResponse {
  sessions: SessionInfo[];
  nextCursor?: Cursor;
  _meta?: Meta;
}

export interface SessionInfo {
  sessionId: SessionId;
  cwd: string;
  additionalDirectories?: string[];
  title?: string;
  updatedAt?: string;
  _meta?: Meta;
}
