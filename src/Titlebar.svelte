<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  // when using `"withGlobalTauri": true`, you may use
  // const { getCurrentWindow } = window.__TAURI__.window;

  const appWindow = getCurrentWindow();
</script>

<div class="sv-title title-bar" data-tauri-drag-region>
  <div class="sv-text title-bar-text">Casio WQV dumper</div>
  <div class="title-bar-controls">
    <button aria-label="Minimize" on:click={() => appWindow.minimize()}></button>
    <button aria-label="Close" on:click={() => appWindow.close()}></button>
  </div>
</div>

<style>
  .sv-title {
    flex-shrink: 0;
    user-select: none;
  }

  .sv-text {
    pointer-events: none;
  }
</style>
