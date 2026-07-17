import type { AuthMethodId, Meta } from "./common.js";

export type AuthMethodType = "agent";

export interface AuthMethod {
  id: AuthMethodId;
  type?: AuthMethodType;
  name: string;
  description?: string;
  _meta?: Meta;
}

export interface AuthenticateRequest {
  methodId: AuthMethodId;
}

export interface AuthenticateResponse {}

export interface LogoutRequest {}

export interface LogoutResponse {}
