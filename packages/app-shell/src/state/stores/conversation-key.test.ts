import { describe, it, expect } from "vitest";
import { conversationKeyFor } from "./conversation-key";

describe("conversationKeyFor", () => {
  it("prefers the session id, then a task key, then a sentinel", () => {
    expect(conversationKeyFor({ sessionId: "s1", taskId: "t1" })).toBe("s1");
    expect(conversationKeyFor({ sessionId: null, taskId: "t1" })).toBe(
      "task:t1",
    );
    expect(conversationKeyFor({ sessionId: null, taskId: null })).toBe(
      "__none__",
    );
  });

  it("separates sibling sessions that share a task", () => {
    expect(conversationKeyFor({ sessionId: "s1", taskId: "t1" })).not.toBe(
      conversationKeyFor({ sessionId: "s2", taskId: "t1" }),
    );
  });
});
