import type { Meta } from "./common.js";

export interface EnvironmentVariable {
  name: string;
  value: string;
}

export interface HttpHeader {
  name: string;
  value: string;
}

export type McpTransport = "http" | "sse";

export interface StdioMcpServer {
  name: string;
  command: string;
  args: string[];
  env: EnvironmentVariable[];
  _meta?: Meta;
}

export interface HttpMcpServer {
  type: "http";
  name: string;
  url: string;
  headers?: HttpHeader[];
  _meta?: Meta;
}

export interface SseMcpServer {
  type: "sse";
  name: string;
  url: string;
  headers?: HttpHeader[];
  _meta?: Meta;
}

export type McpServer = StdioMcpServer | HttpMcpServer | SseMcpServer;
