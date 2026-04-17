// Copyright 2026 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { computed } from "vue";
import { useQuasar } from "quasar";

/**
 * Reactive screen size composable wrapping Quasar's $q.screen API.
 * All mobile-responsive gating in the app should use this single source of truth.
 *
 * Breakpoints align with Quasar defaults:
 *   mobile:  < 600px  (xs)
 *   tablet:  600px – 1023px  (sm)
 *   desktop: >= 1024px (md+)
 */
export function useScreen() {
  const $q = useQuasar();

  const isMobile = computed(() => $q.screen.lt.sm);
  const isTablet = computed(() => $q.screen.gt.xs && $q.screen.lt.md);
  const isDesktop = computed(() => $q.screen.gt.sm);
  const isTouch = computed(
    () =>
      $q.platform.is.mobile ||
      (typeof globalThis !== "undefined" && "ontouchstart" in globalThis),
  );
  const screenSize = computed<"mobile" | "tablet" | "desktop">(() => {
    if (isMobile.value) return "mobile";
    if (isTablet.value) return "tablet";
    return "desktop";
  });
  const screenWidth = computed(() => $q.screen.width);

  return {
    isMobile,
    isTablet,
    isDesktop,
    isTouch,
    screenSize,
    screenWidth,
  };
}
