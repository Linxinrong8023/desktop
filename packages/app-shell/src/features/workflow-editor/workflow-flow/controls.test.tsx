import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppI18nProvider } from "../../../i18n/i18n";
import { appI18n } from "../../../i18n/i18n-instance";
import { WorkflowCanvasControls } from "./controls";

const { fitView, setViewport, zoomTo } = vi.hoisted(() => ({
  fitView: vi.fn(),
  setViewport: vi.fn(),
  zoomTo: vi.fn(),
}));

vi.mock("@xyflow/react", () => ({
  useReactFlow: () => ({ fitView, setViewport, zoomTo }),
  useViewport: () => ({ zoom: 1 }),
}));

describe("workflow canvas viewport controls", () => {
  beforeEach(async () => {
    await appI18n.changeLanguage("en-US");
    fitView.mockReset();
    setViewport.mockReset();
    zoomTo.mockReset();
  });

  it("uses the compact Dify-style bottom-right control group", async () => {
    const user = userEvent.setup();
    render(
      <AppI18nProvider>
        <WorkflowCanvasControls defaultViewport={{ x: 12, y: 20, zoom: 1 }} />
      </AppI18nProvider>,
    );

    const toolbar = screen.getByRole("toolbar", {
      name: "Canvas view controls",
    });
    expect(toolbar).toHaveAttribute("data-workflow-viewport-controls");
    expect(toolbar).toHaveClass(
      "absolute",
      "bottom-3",
      "right-3",
      "z-40",
      "rounded-xl",
      "p-1",
      "shadow-lg",
    );
    await user.click(screen.getByRole("button", { name: "Zoom out" }));
    expect(zoomTo).toHaveBeenCalledWith(0.9);

    await user.click(
      screen.getByRole("button", { name: "Fit workflow to view" }),
    );
    expect(fitView).toHaveBeenCalledWith({
      duration: 220,
      maxZoom: 1,
      minZoom: 0.4,
      padding: 0.16,
    });

    await user.click(screen.getByRole("button", { name: "Reset canvas view" }));
    expect(setViewport).toHaveBeenCalledWith(
      { x: 12, y: 20, zoom: 1 },
      { duration: 180 },
    );
  });
});
