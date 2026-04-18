// Copyright 2026 OpenObserve Inc.
// Licensed under AGPL v3.

import { ref, onMounted, onBeforeUnmount, type Ref } from "vue";
import { useScreen } from "./useScreen";

export interface PullToRefreshOptions {
  threshold?: number;
  onRefresh: () => Promise<unknown> | unknown;
}

export function usePullToRefresh(
  containerRef: Ref<HTMLElement | null>,
  options: PullToRefreshOptions,
) {
  const { isMobile } = useScreen();
  const threshold = options.threshold ?? 70;

  const pullDistance = ref(0);
  const isRefreshing = ref(false);
  const isPulling = ref(false);

  let startY = 0;
  let pulling = false;

  const resetPull = () => {
    pullDistance.value = 0;
    isPulling.value = false;
    pulling = false;
    startY = 0;
  };

  const onTouchStart = (e: TouchEvent) => {
    if (!isMobile.value || isRefreshing.value) return;
    const el = containerRef.value;
    if (!el) return;
    if (el.scrollTop > 0) return;
    startY = e.touches[0].clientY;
    pulling = true;
  };

  const onTouchMove = (e: TouchEvent) => {
    if (!pulling || isRefreshing.value) return;
    const delta = e.touches[0].clientY - startY;
    if (delta <= 0) {
      resetPull();
      return;
    }
    isPulling.value = true;
    pullDistance.value = Math.min(delta * 0.5, threshold * 1.5);
    if (e.cancelable) e.preventDefault();
  };

  const onTouchEnd = async () => {
    if (!pulling) return;
    const triggered = pullDistance.value >= threshold;
    pulling = false;
    isPulling.value = false;

    if (triggered && !isRefreshing.value) {
      isRefreshing.value = true;
      try {
        await options.onRefresh();
      } finally {
        isRefreshing.value = false;
        pullDistance.value = 0;
      }
    } else {
      pullDistance.value = 0;
    }
  };

  onMounted(() => {
    const el = containerRef.value;
    if (!el) return;
    el.addEventListener("touchstart", onTouchStart, { passive: true });
    el.addEventListener("touchmove", onTouchMove, { passive: false });
    el.addEventListener("touchend", onTouchEnd);
    el.addEventListener("touchcancel", onTouchEnd);
  });

  onBeforeUnmount(() => {
    const el = containerRef.value;
    if (!el) return;
    el.removeEventListener("touchstart", onTouchStart);
    el.removeEventListener("touchmove", onTouchMove);
    el.removeEventListener("touchend", onTouchEnd);
    el.removeEventListener("touchcancel", onTouchEnd);
  });

  return {
    pullDistance,
    isRefreshing,
    isPulling,
    threshold,
  };
}
