import { expect, it } from "vitest";
import { createHttpClient, HttpError } from "./client";

it("uses relative proxy requests and preserves the request identity", async () => {
  const input = {
    requestId: "original",
    repository: "https://example.com/repo.git",
    branch: "main",
  };
  const receipt = {
    requestId: "original",
    operationId: "operation",
    executionId: "execution",
  };
  const controller = new AbortController();
  const client = createHttpClient(async (url, init) => {
    expect(url).toBe("/api/clones");
    expect(init).toEqual({
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
      signal: controller.signal,
    });
    return Response.json(receipt, { status: 202 });
  });
  expect(await client.submit(input, controller.signal)).toEqual(receipt);
});

it("reports HTTP unavailability separately from clone results", async () => {
  const client = createHttpClient(
    async () => new Response("offline", { status: 503 }),
  );
  await expect(client.list(new AbortController().signal)).rejects.toEqual(
    new HttpError("unavailable"),
  );
});
