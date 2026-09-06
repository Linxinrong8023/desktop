/** Any value that can cross Ora's plugin JSON-RPC boundary. */
export type JsonValue = null | boolean | number | string | JsonValue[] | {
  [key: string]: JsonValue;
};

/** JSON-RPC identifiers accepted in either protocol direction. */
export type RequestId = number | string;

/** One JSON-RPC request that expects a correlated response. */
export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: RequestId;
  method: string;
  params?: JsonValue;
}

/** One JSON-RPC notification with no response channel. */
export interface JsonRpcNotification {
  jsonrpc: "2.0";
  method: string;
  params?: JsonValue;
}
