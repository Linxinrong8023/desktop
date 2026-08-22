import { describe, expect, it } from "vitest";
import { countChanges, parseTaskDiffPatch } from "./task-diff-data";

const PATCH = [
  "diff --git a/src/main.ts b/src/main.ts",
  "index 3bd1f0e..17c13d8 100644",
  "--- a/src/main.ts",
  "+++ b/src/main.ts",
  "@@ -1,2 +1,2 @@",
  " const stable = true;",
  "-const value = 1;",
  "+const value = 2;",
  "",
].join("\n");

describe("task diff view mapping", () => {
  it("maps an empty backend patch to an empty file list", () => {
    expect(parseTaskDiffPatch(" \r\n")).toEqual([]);
  });

  it("counts additions and deletions from parsed backend patches", () => {
    expect(countChanges(parseTaskDiffPatch(PATCH))).toEqual({
      additions: 1,
      deletions: 1,
    });
  });
});
