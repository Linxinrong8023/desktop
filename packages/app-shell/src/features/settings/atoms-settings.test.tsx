import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "@ora/ui";
import { PlatformProvider } from "@ora/platform";
import { createChatStore } from "@ora/chat";
import { AppI18nProvider } from "../../i18n/i18n";
import { createMockClient, createMockClientState } from "../../test/mock-client";
import { createHookWrapper, createTestQueryClient } from "../../test/hook-harness";
import { createStubPlatform } from "../../test/stub-platform";
import { RolesSettings, SkillsSettings } from "./atoms-settings";

function renderSettings(kind: "agent" | "skill", configure?: (client: ReturnType<typeof createMockClient>) => void) {
  const state = createMockClientState();
  if (kind === "agent") {
    state.agents = [{ id: "agent-1", name: "review-agent", description: "Reviews changes" }];
  } else {
    state.skills = [{ id: "skill-1", name: "review-skill", description: "Reviews changes" }];
  }
  const client = createMockClient(state);
  client.agent.get = async ({ agentId }) => ({
    agent: {
      ...state.agents.find((agent) => agent.id === agentId)!,
      content: "**Agent instructions**",
    },
  });
  client.skill.get = async ({ skillId }) => ({
    skill: {
      ...state.skills.find((skill) => skill.id === skillId)!,
      content: "## Skill instructions",
    },
  });
  configure?.(client);
  const Wrapper = createHookWrapper(client, createTestQueryClient(), createChatStore(client.session));

  return render(
    <Wrapper>
      <AppI18nProvider>
        <PlatformProvider adapter={createStubPlatform()}>
          <TooltipProvider>
            {kind === "agent" ? <RolesSettings /> : <SkillsSettings />}
          </TooltipProvider>
        </PlatformProvider>
      </AppI18nProvider>
    </Wrapper>,
  );
}

describe("atom settings content", () => {
  it("loads Agent content as read-only when editing", async () => {
    const user = userEvent.setup();
    renderSettings("agent");

    await user.click(await screen.findByRole("button", { name: "编辑" }));

    const content = await screen.findByLabelText("内容");
    expect(content.querySelector("strong")?.textContent).toBe("Agent instructions");
    expect(content.querySelector("textarea")).toBeNull();
  });

  it("uses explicit outline actions and loads Skill content as read-only", async () => {
    const user = userEvent.setup();
    renderSettings("skill");

    const importButton = await screen.findByRole("button", { name: "导入 Skill" });
    const newButton = screen.getByRole("button", { name: "新建 Skill" });
    expect(importButton).toHaveClass("border");
    expect(newButton).toHaveClass("border");

    await user.click(await screen.findByRole("button", { name: "编辑" }));

    const content = await screen.findByLabelText("内容");
    expect(content.querySelector("h2")?.textContent).toBe("Skill instructions");
    expect(content.querySelector("textarea")).toBeNull();
  });
  it("imports one Agent Markdown file with an overwrite decision", async () => {
    const user = userEvent.setup();
    const markdown = "---\nname: review-agent\ndescription: Imported review agent\n---\nReview carefully.";
    const prepare = vi.fn(async () => ({
      candidate: {
        name: "review-agent",
        description: "Imported review agent",
        status: "conflict" as const,
        existingAgent: {
          agentId: "agent-1",
          updatedAt: 42,
          description: "Existing agent",
        },
      },
    }));
    const commit = vi.fn(async () => ({
      status: "overwritten" as const,
      agent: { id: "agent-1", name: "review-agent", description: "Imported review agent" },
    }));
    renderSettings("agent", (client) => {
      client.agentImport.prepare = prepare;
      client.agentImport.commit = commit;
    });

    const importButton = await screen.findByRole("button", { name: "导入 Role" });
    expect(importButton).toHaveClass("border");
    await user.click(importButton);

    const dialog = await screen.findByRole("dialog");
    const input = dialog.querySelector<HTMLInputElement>('input[type="file"]');
    expect(input).not.toBeNull();
    expect(input).toHaveAttribute("accept", ".md,text/markdown");
    expect(input).not.toHaveAttribute("multiple");
    const file = new File([markdown], "review-agent.md", { type: "text/markdown" });
    Object.defineProperty(file, "text", { value: async () => markdown });
    await user.upload(input!, file);

    expect(await within(dialog).findByText("Imported review agent")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "覆盖" }));
    await user.click(within(dialog).getByRole("button", { name: "确认导入" }));

    await waitFor(() => expect(commit).toHaveBeenCalledWith({
      content: markdown,
      decision: "overwrite",
      expectedAgentId: "agent-1",
      expectedUpdatedAt: 42,
    }));
    expect(prepare).toHaveBeenCalledWith({ content: markdown });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("refreshes an Agent conflict when commit reports stale state", async () => {
    const user = userEvent.setup();
    const markdown = "---\nname: review-agent\ndescription: Imported review agent\n---\nReview carefully.";
    const prepare = vi
      .fn()
      .mockResolvedValueOnce({
        candidate: {
          name: "review-agent",
          description: "Imported review agent",
          status: "conflict",
          existingAgent: { agentId: "agent-1", updatedAt: 42, description: "Old description" },
        },
      })
      .mockResolvedValueOnce({
        candidate: {
          name: "review-agent",
          description: "Imported review agent",
          status: "conflict",
          existingAgent: { agentId: "agent-1", updatedAt: 43, description: "Changed description" },
        },
      });
    renderSettings("agent", (client) => {
      client.agentImport.prepare = prepare;
      client.agentImport.commit = async () => ({ status: "stale_conflict", agent: null });
    });

    await user.click(await screen.findByRole("button", { name: "导入 Role" }));
    const dialog = await screen.findByRole("dialog");
    const input = dialog.querySelector<HTMLInputElement>('input[type="file"]')!;
    const file = new File([markdown], "review-agent.md", { type: "text/markdown" });
    Object.defineProperty(file, "text", { value: async () => markdown });
    await user.upload(input, file);
    await user.click(await within(dialog).findByRole("button", { name: "覆盖" }));
    await user.click(within(dialog).getByRole("button", { name: "确认导入" }));

    expect(await within(dialog).findByText("目标 Agent 已发生变化，请重新确认处理方式。")).toBeInTheDocument();
    expect(await within(dialog).findByText("现有描述：Changed description")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "确认导入" })).toBeDisabled();
    expect(prepare).toHaveBeenCalledTimes(2);
  });
});
