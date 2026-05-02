import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { Quasar } from "quasar";
import MobileDashboardCard from "./MobileDashboardCard.vue";

const mountCard = (row: Record<string, any>) =>
  mount(MobileDashboardCard, {
    props: { row },
    global: { plugins: [Quasar] },
  });

describe("MobileDashboardCard", () => {
  const baseRow = {
    id: "d-1",
    name: "Production Overview",
    description: "Traffic, errors, saturation across prod clusters",
    owner: "sre@example.com",
    created: "2026-04-10",
  };

  it("renders title and description", () => {
    const w = mountCard(baseRow);
    expect(w.find(".mobile-dashboard-card__title").text()).toBe(
      "Production Overview",
    );
    expect(w.find(".mobile-dashboard-card__desc").text()).toContain(
      "Traffic, errors",
    );
  });

  it("renders meta items when present", () => {
    const w = mountCard(baseRow);
    const meta = w.find(".mobile-dashboard-card__meta").text();
    expect(meta).toContain("sre@example.com");
    expect(meta).toContain("2026-04-10");
  });

  it("omits description when empty", () => {
    const w = mountCard({ ...baseRow, description: "" });
    expect(w.find(".mobile-dashboard-card__desc").exists()).toBe(false);
  });

  it("emits click with row on tap", async () => {
    const w = mountCard(baseRow);
    await w.trigger("click");
    expect(w.emitted("click")).toBeTruthy();
    expect(w.emitted("click")![0]).toEqual([baseRow]);
  });

  it("emits click on Enter keydown", async () => {
    const w = mountCard(baseRow);
    await w.trigger("keydown.enter");
    expect(w.emitted("click")).toBeTruthy();
  });
});
