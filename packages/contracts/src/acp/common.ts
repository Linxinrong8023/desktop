export type ProtocolVersion = number;
export type SessionId = string;
export type Cursor = string;
export type AuthMethodId = string;
export type MessageId = string;
export type Meta = Record<string, unknown>;

export interface ImplementationInfo {
  name: string;
  title?: string;
  version: string;
  _meta?: Meta;
}

export interface EmptyObject {}
