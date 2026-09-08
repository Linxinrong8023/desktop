import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AppI18nProvider } from "../../i18n/i18n";
import { TaskGitActions } from "./task-git-actions";

/** Renders the Git action popover with deterministic callbacks for interaction tests. */
function renderGitActions(message = "", stagedCount = 2) {
  const callbacks = {
    onOpenChange: vi.fn(),
    onMessageChange: vi.fn(),
    onStageAll: vi.fn(),
    onUnstageAll: vi.fn(),
    onCommit: vi.fn(),
    onCommitAndPush: vi.fn(),
    onPush: vi.fn(),
  };

  render(
    <AppI18nProvider>
      <TaskGitActions
        open
        message={message}
        stagedCount={stagedCount}
        pending={false}
        {...callbacks}
      />
    </AppI18nProvider>,
  );

  return callbacks;
}

describe("task Git actions", () => {
  it("shows staging and commit actions and requires a message before committing", async () => {
    const user = userEvent.setup();
    const callbacks = renderGitActions("", 2);

    expect(
      screen.getByRole("textbox", { name: "提交说明" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "提交" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "提交并推送" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "暂存所有改动" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "取消暂存" })).toBeEnabled();
    expect(screen.getByText("将提交已暂存的 2 项更改")).toBeInTheDocument();

    await user.type(
      screen.getByRole("textbox", { name: "提交说明" }),
      "fix diff layout",
    );
    expect(callbacks.onMessageChange).toHaveBeenCalled();
  });

  it("disables commit and unstage-all when nothing is staged", async () => {
    renderGitActions("fix diff layout", 0);

    expect(screen.getByRole("button", { name: "提交" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "提交并推送" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "取消暂存" })).toBeDisabled();
    expect(screen.getByText("没有已暂存的更改")).toBeInTheDocument();
  });

  it("routes the stage, combined, and push actions to their callbacks", async () => {
    const user = userEvent.setup();
    const callbacks = renderGitActions("fix diff layout", 2);

    expect(screen.getByRole("button", { name: "提交" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "暂存所有改动" }));
    await user.click(screen.getByRole("button", { name: "取消暂存" }));
    await user.click(screen.getByRole("button", { name: "提交并推送" }));
    await user.click(screen.getByRole("button", { name: "推送" }));

    expect(callbacks.onStageAll).toHaveBeenCalledOnce();
    expect(callbacks.onUnstageAll).toHaveBeenCalledOnce();
    expect(callbacks.onCommitAndPush).toHaveBeenCalledOnce();
    expect(callbacks.onPush).toHaveBeenCalledOnce();
  });
});
