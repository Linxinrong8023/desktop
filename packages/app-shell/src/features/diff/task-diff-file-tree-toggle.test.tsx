import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { parseDiff } from "react-diff-view";
import { AppI18nProvider } from "../../i18n/i18n";
import { TaskDiffFileTree } from "./task-diff-file-tree";

const PATCH = [
  "diff --git a/src/a.ts b/src/a.ts",
  "new file mode 100644",
  "index 0000000..1111111",
  "--- /dev/null",
  "+++ b/src/a.ts",
  "@@ -0,0 +1,1 @@",
  "+export const a = 1;",
  "",
].join("\n");

function renderTree(
  stagedByPath: ReadonlySet<string>,
  onToggleStage: (p: string) => void,
) {
  return render(
    <AppI18nProvider>
      <TaskDiffFileTree
        files={parseDiff(PATCH)}
        selectedPath="src/a.ts"
        stagedByPath={stagedByPath}
        onToggleStage={onToggleStage}
        onSelect={() => undefined}
      />
    </AppI18nProvider>,
  );
}

describe("task diff file tree staging", () => {
  it("shows a stage button for an unstaged file and stages it on click", async () => {
    const user = userEvent.setup();
    const onToggleStage = vi.fn();
    renderTree(new Set(), onToggleStage);

    const stageButton = screen.getByRole("button", { name: "暂存 a.ts" });
    expect(stageButton).toBeInTheDocument();
    await user.click(stageButton);

    expect(onToggleStage).toHaveBeenCalledWith("src/a.ts");
  });

  it("shows an unstage button for a staged file", () => {
    renderTree(new Set(["src/a.ts"]), vi.fn());

    expect(
      screen.getByRole("button", { name: "取消暂存 a.ts" }),
    ).toBeInTheDocument();
  });
});
