<!-- Copyright 2026 OpenObserve Inc.
Licensed under AGPL v3. -->

<template>
  <div
    class="mobile-alert-card"
    :class="{ 'mobile-alert-card--disabled': !row.enabled }"
    @click="$emit('click', row)"
    @keydown.enter="$emit('click', row)"
    @keydown.space.prevent="$emit('click', row)"
    role="button"
    tabindex="0"
    :aria-label="`Alert ${row.name}`"
  >
    <div class="mobile-alert-card__row">
      <div class="mobile-alert-card__title-wrap">
        <span
          class="mobile-alert-card__status-dot"
          :class="row.enabled ? 'is-on' : 'is-off'"
          :aria-label="row.enabled ? 'Enabled' : 'Paused'"
        />
        <span class="mobile-alert-card__title" :title="row.name">{{
          row.name
        }}</span>
      </div>
      <q-btn
        :icon="moreIcon"
        round
        flat
        dense
        size="sm"
        class="mobile-alert-card__more"
        aria-label="More actions"
        @click.stop
        @keydown.stop
      >
        <q-menu>
          <q-list dense style="min-width: 180px">
            <q-item clickable v-close-popup @click="$emit('toggle', row)">
              <q-item-section avatar>
                <q-icon :name="row.enabled ? 'pause' : 'play_arrow'" />
              </q-item-section>
              <q-item-section>{{
                row.enabled ? "Pause" : "Start"
              }}</q-item-section>
            </q-item>
            <q-item clickable v-close-popup @click="$emit('edit', row)">
              <q-item-section avatar><q-icon name="edit" /></q-item-section>
              <q-item-section>Edit</q-item-section>
            </q-item>
            <q-item clickable v-close-popup @click="$emit('clone', row)">
              <q-item-section avatar
                ><q-icon name="content_copy"
              /></q-item-section>
              <q-item-section>Clone</q-item-section>
            </q-item>
            <q-item clickable v-close-popup @click="$emit('move', row)">
              <q-item-section avatar
                ><q-icon name="drive_file_move"
              /></q-item-section>
              <q-item-section>Move</q-item-section>
            </q-item>
            <q-item clickable v-close-popup @click="$emit('trigger', row)">
              <q-item-section avatar><q-icon name="send" /></q-item-section>
              <q-item-section>Trigger</q-item-section>
            </q-item>
            <q-separator />
            <q-item
              clickable
              v-close-popup
              class="text-negative"
              @click="$emit('delete', row)"
            >
              <q-item-section avatar><q-icon name="delete" /></q-item-section>
              <q-item-section>Delete</q-item-section>
            </q-item>
          </q-list>
        </q-menu>
      </q-btn>
    </div>

    <div v-if="subtitle" class="mobile-alert-card__subtitle">{{ subtitle }}</div>

    <div class="mobile-alert-card__meta">
      <span v-if="row.owner" class="mobile-alert-card__meta-item">
        <q-icon name="person" size="12px" />{{ row.owner }}
      </span>
      <span v-if="formattedPeriod" class="mobile-alert-card__meta-item">
        <q-icon name="history" size="12px" />{{ formattedPeriod }}
      </span>
      <span v-if="formattedFrequency" class="mobile-alert-card__meta-item">
        <q-icon name="schedule" size="12px" />{{ formattedFrequency }}
      </span>
      <span
        class="mobile-alert-card__state"
        :class="row.enabled ? 'is-on' : 'is-off'"
      >
        {{ row.enabled ? "Enabled" : "Paused" }}
      </span>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from "vue";
import { outlinedMoreVert } from "@quasar/extras/material-icons-outlined";

export default defineComponent({
  name: "MobileAlertCard",
  props: {
    row: {
      type: Object as PropType<Record<string, any>>,
      required: true,
    },
  },
  emits: ["click", "toggle", "edit", "clone", "move", "trigger", "delete"],
  setup(props) {
    const moreIcon = outlinedMoreVert;
    const subtitle = computed(() => {
      const parts: string[] = [];
      if (props.row.stream_name) parts.push(String(props.row.stream_name));
      if (props.row.type) parts.push(String(props.row.type));
      return parts.join(" · ");
    });
    // Mirror the desktop q-table period/frequency column formatters so the
    // same numeric row values render as human-readable strings on mobile.
    const formattedPeriod = computed(() => {
      const v = props.row.period;
      if (v === undefined || v === null || v === "") return "";
      const n = Number(v);
      if (Number.isNaN(n)) return String(v);
      if (n >= 60) {
        const hours = Math.floor(n / 60);
        const mins = n % 60;
        return mins === 0 ? `${hours} Hours` : `${hours} Hours ${mins} Mins`;
      }
      return `${n} Mins`;
    });
    const formattedFrequency = computed(() => {
      const v = props.row.frequency;
      if (v === undefined || v === null || v === "") return "";
      return props.row.frequency_type === "cron" ? String(v) : `${v} Mins`;
    });
    return { moreIcon, subtitle, formattedPeriod, formattedFrequency };
  },
});
</script>

<style scoped lang="scss">
.mobile-alert-card {
  background: var(--o2-card-bg);
  border: 1px solid var(--o2-border-color);
  border-radius: 8px;
  padding: 10px 12px;
  margin-bottom: 8px;
  cursor: pointer;
  -webkit-tap-highlight-color: transparent;
  transition:
    background 150ms ease,
    transform 120ms ease;

  &:active {
    background: var(--o2-hover-accent);
    transform: scale(0.995);
  }

  &--disabled {
    opacity: 0.65;
  }

  &__row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  &__title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  &__status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;

    &.is-on {
      background: var(--o2-status-success-text, #21ba45);
      box-shadow: 0 0 0 3px rgba(33, 186, 69, 0.15);
    }
    &.is-off {
      background: var(--o2-text-muted, #818594);
    }
  }

  &__title {
    font-weight: 600;
    font-size: 14px;
    color: var(--o2-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  &__more {
    flex-shrink: 0;
    min-width: 40px;
    min-height: 40px;
  }

  &__subtitle {
    margin-top: 3px;
    margin-left: 16px;
    font-family: monospace;
    font-size: 11px;
    color: var(--o2-text-muted, #818594);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  &__meta {
    margin-top: 8px;
    margin-left: 16px;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-size: 11px;
    color: var(--o2-text-muted, #818594);
  }

  &__meta-item {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  &__state {
    margin-left: auto;
    padding: 1px 8px;
    border-radius: 10px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;

    &.is-on {
      background: rgba(33, 186, 69, 0.12);
      color: var(--o2-status-success-text, #21ba45);
    }
    &.is-off {
      background: rgba(129, 133, 148, 0.12);
      color: var(--o2-text-muted, #818594);
    }
  }
}
</style>
