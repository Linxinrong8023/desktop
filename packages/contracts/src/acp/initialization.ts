import type { AuthMethod } from "./authentication.js";
import type { EmptyObject, ImplementationInfo, Meta, ProtocolVersion } from "./common.js";

export interface InitializeRequest {
  protocolVersion: ProtocolVersion;
  clientCapabilities: ClientCapabilities;
  clientInfo?: ImplementationInfo;
  _meta?: Meta;
}

export interface InitializeResponse {
  protocolVersion: ProtocolVersion;
  agentCapabilities: AgentCapabilities;
  agentInfo?: ImplementationInfo;
  authMethods: AuthMethod[];
  _meta?: Meta;
}

export interface ClientCapabilities {
  fs?: FileSystemCapabilities;
  terminal?: boolean;
}

export interface FileSystemCapabilities {
  readTextFile?: boolean;
  writeTextFile?: boolean;
}

export interface AgentCapabilities {
  loadSession?: boolean;
  promptCapabilities?: PromptCapabilities;
  mcpCapabilities?: McpCapabilities;
  auth?: AuthenticationCapabilities;
  sessionCapabilities?: SessionCapabilities;
}

export interface PromptCapabilities {
  image?: boolean;
  audio?: boolean;
  embeddedContext?: boolean;
}

export interface McpCapabilities {
  http?: boolean;
  sse?: boolean;
}

export interface AuthenticationCapabilities {
  logout?: EmptyObject;
}

export interface SessionCapabilities {
  list?: boolean;
  delete?: boolean;
  resume?: boolean;
  close?: boolean;
  additionalDirectories?: boolean;
}
