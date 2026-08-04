import { QueryClientProvider } from "@tanstack/react-query";
import { PlatformProvider, type PlatformAdapter } from "@ora/platform";
import { RemoteContractError, type ContractsClient, type Skill } from "@ora/contracts";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { ContractsClientContext } from "../../contracts-client-context";
import { appI18n } from "../../i18n/i18n-instance";
import { createMockClient, createMockClientState } from "../../test/mock-client";
import { createStubPlatform } from "../../test/stub-platform";
import { createTestQueryClient } from "../../test/hook-harness";
import { RolesSettings, SkillsSettings } from "./atoms-settings";

const IMPORTED_SKILL: Skill = {
  id: "skill-1",
  name: "demo",
  description: "Demo",
};

/** Renders one settings pane with real query, client, platform, and i18n providers. */
function renderSettings(
  view: ReactNode,
  platform: PlatformAdapter,
): { client: ContractsClient; listSkills: ReturnType<typeof vi.fn> } {
  const state = createMockClientState();
  const client = createMockClient(state);
  const listSkills = vi.fn(client.skill.list);
  const clientWithSpy = {
    ...client,
    skill: { ...client.skill, list: listSkills },
  } as ContractsClient;

  render(
    <I18nextProvider i18n={appI18n}>
      <QueryClientProvider client={createTestQueryClient()}>
        <ContractsClientContext.Provider value={clientWithSpy}>
          <PlatformProvider adapter={platform}>
            {view}
          </PlatformProvider>
        </ContractsClientContext.Provider>
      </QueryClientProvider>
    </I18nextProvider>,
  );
  return { client: clientWithSpy, listSkills };
}

describe("SkillsSettings import", () => {
  it("shows the import action only for skills and refreshes after success", async () => {
    await appI18n.changeLanguage("zh-CN");
    const importFolder = vi.fn().mockResolvedValue(IMPORTED_SKILL);
    const platform: PlatformAdapter = {
      ...createStubPlatform(),
      skillFolderImport: { kind: "supported", importFolder },
    };
    const { listSkills } = renderSettings(<SkillsSettings />, platform);
    const user = userEvent.setup();

    await user.click(await screen.findByRole("button", { name: "导入 Skill" }));

    await screen.findByText("Skill 导入成功。");
    expect(importFolder).toHaveBeenCalledOnce();
    await waitFor(() => expect(listSkills.mock.calls.length).toBeGreaterThan(1));

    renderSettings(<RolesSettings />, platform);
    expect(screen.getAllByRole("button", { name: "导入 Skill" })).toHaveLength(1);
  });

  it("shows the localized contract error when import fails", async () => {
    await appI18n.changeLanguage("zh-CN");
    const importFolder = vi.fn().mockRejectedValue(new RemoteContractError(
      {
        code: "skill_manifest_missing",
        params: {},
        requestId: "550e8400-e29b-41d4-a716-446655440000",
      },
      422,
      null,
    ));
    const platform: PlatformAdapter = {
      ...createStubPlatform(),
      skillFolderImport: { kind: "supported", importFolder },
    };
    renderSettings(<SkillsSettings />, platform);
    const user = userEvent.setup();

    await user.click(await screen.findByRole("button", { name: "导入 Skill" }));

    await screen.findByText("缺少技能清单。");
  });
});
