// Copyright 2026 OpenObserve Inc.
// Licensed under AGPL v3.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { Quasar } from "quasar";
import { createStore } from "vuex";
import { createRouter, createMemoryHistory } from "vue-router";

const vibrateSpy = vi.fn();
vi.mock("@/composables/useHaptics", () => ({
  useHaptics: () => ({
    vibrate: vibrateSpy,
    supportsVibrate: { value: true },
  }),
}));

import MobileBottomNav from "./MobileBottomNav.vue";

const makeRouter = () =>
  createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: { template: "<div/>" }, name: "home" },
      { path: "/logs", component: { template: "<div/>" }, name: "logs" },
      {
        path: "/dashboards",
        component: { template: "<div/>" },
        name: "dashboards",
      },
      {
        path: "/alerts",
        component: { template: "<div/>" },
        name: "alertList",
      },
    ],
  });

const makeStore = () =>
  createStore({
    state: {
      selectedOrganization: { identifier: "default" },
    },
  });

const links = [
  { title: "Home", icon: "home", link: "/", name: "home", display: true },
  { title: "Logs", icon: "list", link: "/logs", name: "logs", display: true },
  {
    title: "Dashboards",
    icon: "dashboard",
    link: "/dashboards",
    name: "dashboards",
    display: true,
  },
  {
    title: "Alerts",
    icon: "alert",
    link: "/alerts",
    name: "alertList",
    display: true,
  },
];

describe("MobileBottomNav haptics", () => {
  beforeEach(() => {
    vibrateSpy.mockReset();
  });

  it("fires a selection haptic when tapping an inactive tab", async () => {
    const router = makeRouter();
    const store = makeStore();
    await router.push("/logs");
    const wrapper = mount(MobileBottomNav, {
      props: { links },
      global: { plugins: [Quasar, store, router] },
    });

    const homeTab = wrapper.find('[aria-label="Home"]');
    await homeTab.trigger("click");

    expect(vibrateSpy).toHaveBeenCalledWith("selection");
  });

  it("does not fire a haptic when tapping the already-active tab", async () => {
    const router = makeRouter();
    const store = makeStore();
    await router.push("/logs");
    const wrapper = mount(MobileBottomNav, {
      props: { links },
      global: { plugins: [Quasar, store, router] },
    });

    const logsTab = wrapper.find('[aria-label="Logs"]');
    await logsTab.trigger("click");

    expect(vibrateSpy).not.toHaveBeenCalled();
  });

  it("fires a selection haptic when opening the More sheet", async () => {
    const router = makeRouter();
    const store = makeStore();
    const wrapper = mount(MobileBottomNav, {
      props: { links },
      global: { plugins: [Quasar, store, router] },
    });

    const moreBtn = wrapper.find('[aria-label="More navigation options"]');
    await moreBtn.trigger("click");

    expect(vibrateSpy).toHaveBeenCalledWith("selection");
  });
});
