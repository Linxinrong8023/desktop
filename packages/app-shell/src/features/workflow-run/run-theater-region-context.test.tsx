import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { AppI18nProvider } from "../../i18n/i18n";
import { appI18n } from "../../i18n/i18n-instance";
import { RunTheaterRegionContext } from "./run-theater-region-context";

describe("RunTheaterRegionContext", () => {
  beforeEach(async () => {
    await appI18n.changeLanguage("zh-CN");
  });

  it("keeps iteration, round, and parallel position visible above node details", () => {
    render(
      <AppI18nProvider>
        <RunTheaterRegionContext
          regionTitle="逐项处理"
          nodeTitle="Agent 1"
          round={1}
          roundCount={3}
          phaseKind="parallel"
          peerIndex={0}
          peerCount={2}
        />
      </AppI18nProvider>,
    );

    expect(
      screen.getByRole("navigation", { name: "节点执行上下文" }),
    ).toHaveTextContent("逐项处理/第 2/3 轮/并行 1/2/Agent 1");
  });
});
