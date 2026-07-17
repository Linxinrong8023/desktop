import assert from "node:assert/strict";
import test from "node:test";

import type { acp } from "../src/index.js";

test("exports ACP message payload types from the contracts package", () => {
  const request: acp.AuthenticateRequest = {
    methodId: "login",
  };
  const response: acp.AuthenticateResponse = {};
  const server: acp.McpServer = {
    type: "http",
    name: "remote",
    url: "https://mcp.example.test",
  };
  const sessions: acp.ListSessionsResponse = {
    sessions: [
      {
        sessionId: "session-1",
        cwd: "/workspace",
      },
    ],
  };
  const notification: acp.SessionUpdateNotification = {
    sessionId: "session-1",
    update: {
      sessionUpdate: "session_info_update",
      title: { kind: "set", value: "Implement ACP" },
      updatedAt: { kind: "absent" },
    },
  };

  assert.equal(request.methodId, "login");
  assert.deepEqual(response, {});
  assert.equal(server.type, "http");
  assert.equal(sessions.sessions.length, 1);
  assert.equal(notification.update.sessionUpdate, "session_info_update");
});
