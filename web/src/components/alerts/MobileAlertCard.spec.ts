import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { Quasar } from "quasar";
import MobileAlertCard from "./MobileAlertCard.vue";

const mountCard = (row: Record<string, any>) =>
  mount(MobileAlertCard, {
    props: { row },
    global: { plugins: [Quasar] },
  });

describe("MobileAlertCard", () => {
  const baseRow = {
    alert_id: "a-1",
    name: "HighErrorRate",
    owner: "ops@example.com",
    period: "5m",
    frequency: "1m",
    stream_name: "prod_logs",
    type: "scheduled",
    enabled: true,
  };

  it("renders title and subtitle", () => {
    const w = mountCard(baseRow);
    expect(w.find(".mobile-alert-card__title").text()).toBe("HighErrorRate");
    expect(w.find(".mobile-alert-card__subtitle").text()).toContain("prod_logs");
    expect(w.find(".mobile-alert-card__subtitle").text()).toContain("scheduled");
  });

  it("shows Enabled state when row.enabled is true", () => {
    const w = mountCard(baseRow);
    expect(w.find(".mobile-alert-card__state").text()).toBe("Enabled");
    expect(w.find(".mobile-alert-card__state").classes()).toContain("is-on");
  });

  it("shows Paused state and applies disabled modifier when row.enabled is false", () => {
    const w = mountCard({ ...baseRow, enabled: false });
    expect(w.find(".mobile-alert-card__state").text()).toBe("Paused");
    expect(w.classes()).toContain("mobile-alert-card--disabled");
  });

  it("renders meta items when present", () => {
    const w = mountCard(baseRow);
    const meta = w.find(".mobile-alert-card__meta").text();
    expect(meta).toContain("ops@example.com");
    expect(meta).toContain("5m");
    expect(meta).toContain("1m");
  });

  it("emits click with row on card click", async () => {
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

  it("omits subtitle when stream_name and type are absent", () => {
    const w = mountCard({ ...baseRow, stream_name: undefined, type: undefined });
    expect(w.find(".mobile-alert-card__subtitle").exists()).toBe(false);
  });
});
