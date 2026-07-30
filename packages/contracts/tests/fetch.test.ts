import assert from "node:assert/strict";
import test from "node:test";

import { createFetchTransport, decodeErrorEnvelope, resolveUrl } from "../src/fetch.js";
import {
  ContractTransportError,
  LocalTransportError,
  RemoteContractError,
  UnknownRemoteError,
  decodeRemoteError,
  type ContractTransportRequest,
} from "../src/transport.js";

const requestId = "123e4567-e89b-42d3-a456-426614174000";

test("validates known, unknown, and malformed remote errors", () => {
  assert.ok(
    decodeRemoteError({ code: "project_not_found", params: {}, requestId }, 404)
      instanceof RemoteContractError,
  );
  assert.ok(
    decodeRemoteError({ code: "future_error", params: {}, requestId }, 500)
      instanceof UnknownRemoteError,
  );
  assert.ok(
    decodeRemoteError({ code: "project_not_found", params: {} }, 404)
      instanceof LocalTransportError,
  );
});

test("resolves paths relative to the current origin when baseUrl is empty", () => {
  assert.equal(resolveUrl("", "/api/projects"), "/api/projects");
});

test("resolves paths against an absolute server base", () => {
  assert.equal(
    resolveUrl("http://localhost:32578", "/api/projects"),
    "http://localhost:32578/api/projects",
  );
});

test("decodes the shared HTTP error envelope", () => {
  assert.deepEqual(
    decodeErrorEnvelope({
      error: {
        code: "project_not_found",
        message: "project not found: project-1",
      },
    }),
    {
      code: "project_not_found",
      message: "project not found: project-1",
    },
  );
});

test("normalizes structured server errors from fetch responses", async () => {
  const requests: Array<{
    url: string;
    init: RequestInit | undefined;
  }> = [];
  const transport = createFetchTransport({
    baseUrl: "http://localhost:32578",
    fetch: async (input, init) => {
      requests.push({
        url: String(input),
        init,
      });

      return new Response(
        JSON.stringify({
          error: {
            code: "project_not_found",
            message: "project not found: project-1",
          },
        }),
        {
          status: 404,
          headers: {
            "content-type": "application/json",
          },
        },
      );
    },
  });
  const request: ContractTransportRequest = {
    operationName: "getProject",
    request: { projectId: "project-1" },
    method: "GET",
    path: "/api/projects/project-1",
    body: undefined,
    headers: {},
  };

  await assert.rejects(
    transport.send(request),
    (error: unknown) => {
      assert.ok(error instanceof ContractTransportError);
      const transportError = error as ContractTransportError;

      assert.equal(transportError.code, "project_not_found");
      assert.equal(transportError.status, 404);
      assert.deepEqual(transportError.responseBody, {
        error: {
          code: "project_not_found",
          message: "project not found: project-1",
        },
      });

      return true;
    },
  );
  assert.deepEqual(requests, [
    {
      url: "http://localhost:32578/api/projects/project-1",
      init: {
        method: "GET",
        headers: {},
        body: undefined,
      },
    },
  ]);
});

test("starts NDJSON streams lazily, decodes split frames, and enforces single consumption", async () => {
  let fetchCalls = 0;
  const encoder = new TextEncoder();
  const transport = createFetchTransport({
    fetch: async () => {
      fetchCalls += 1;
      return new Response(new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(encoder.encode('{"type":"data","data":{"value":'));
          controller.enqueue(encoder.encode('1}}\n{"type":"end"}\n'));
          controller.close();
        },
      }), { status: 200 });
    },
  });
  const request: ContractTransportRequest = {
    operationName: "loadSession",
    request: { sessionId: "session-1" },
    method: "POST",
    path: "/api/sessions/session-1/load",
    body: undefined,
    headers: {},
  };
  const stream = transport.stream<{ value: number }>(request);

  assert.equal(fetchCalls, 0);
  const received: Array<{ value: number }> = [];
  for await (const event of stream) received.push(event);
  assert.deepEqual(received, [{ value: 1 }]);
  assert.equal(fetchCalls, 1);
  assert.throws(
    () => stream[Symbol.asyncIterator](),
    (error: unknown) => error instanceof ContractTransportError && error.code === "stream_already_consumed",
  );
});

test("surfaces a typed stream error frame and aborts the underlying fetch lifecycle", async () => {
  let observedSignal: AbortSignal | undefined;
  const transport = createFetchTransport({
    fetch: async (_input, init) => {
      observedSignal = init?.signal as AbortSignal;
      return new Response(
        '{"type":"error","error":{"code":"session_busy","message":"busy"}}\n',
        { status: 200 },
      );
    },
  });
  const stream = transport.stream({
    operationName: "promptSession",
    request: { sessionId: "session-1", text: "hello" },
    method: "POST",
    path: "/api/sessions/session-1/prompt",
    body: { text: "hello" },
    headers: { "content-type": "application/json" },
  });

  await assert.rejects(
    async () => {
      for await (const _event of stream) {
        assert.fail("error-only stream must not yield data");
      }
    },
    (error: unknown) => error instanceof ContractTransportError && error.code === "session_busy",
  );
  assert.equal(observedSignal?.aborted, true);
});
