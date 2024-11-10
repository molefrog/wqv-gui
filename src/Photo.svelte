<script lang="ts">
  import { onMount } from "svelte";
  import { WatchDevice } from "./device.svelte";

  const FN_SZ = 24,
    DATE_SZ = 5;

  let {
    device,
    onCancel,
  }: {
    device: WatchDevice;
    onCancel: () => void;
  } = $props();

  let filename = $derived.by(() => {
    if (device.imgBytesReceived >= FN_SZ) {
      const bytes = device.imgData.slice(0, FN_SZ);
      return new TextDecoder("utf8").decode(bytes).trim();
    }
  });

  let date = $derived.by(() => {
    if (device.imgBytesReceived >= FN_SZ + DATE_SZ) {
      const bytes = device.imgData.slice(FN_SZ, FN_SZ + DATE_SZ);

      const year = 2000 + bytes[0];
      const month = bytes[1];
      const day = bytes[2];
      const hour = bytes[3];
      const minute = bytes[4];

      return new Date(year, month - 1, day, hour, minute);
    }
  });

  let formattedDate = $derived.by(() => {
    return date?.toLocaleString();
  });

  let isActionDisabled = $derived(device.status !== "ready");

  /**
   * rendering on a canvas
   */
  let canvas = $state<HTMLCanvasElement | null>(null);

  $effect(() => {
    if (!canvas) return;

    const ctx = canvas.getContext("2d")!;
    canvas.width = 120;
    canvas.height = 120;

    // Re-render when bytes received changes
    const buffer = device.imgData.slice(FN_SZ + DATE_SZ, device.imgBytesReceived);

    const imageData = ctx.createImageData(120, 120);
    const data = imageData.data;

    // Each byte contains two 4-bit pixels
    for (let i = 0; i < buffer.length; i++) {
      const byte = buffer[i];
      // Extract high and low nibbles
      const pixel1 = byte & 0xf;
      const pixel2 = (byte >> 4) & 0xf;

      // Convert 4-bit grayscale to RGBA
      const idx1 = i * 8; // First pixel position in RGBA array
      const idx2 = idx1 + 4; // Second pixel position in RGBA array

      // Set RGBA values for first pixel (inverted)
      const val1 = 255 - (pixel1 * 255) / 15;
      data[idx1] = val1; // R
      data[idx1 + 1] = val1; // G
      data[idx1 + 2] = val1; // B
      data[idx1 + 3] = 255; // A

      // Set RGBA values for second pixel (inverted)
      const val2 = 255 - (pixel2 * 255) / 15;
      data[idx2] = val2; // R
      data[idx2 + 1] = val2; // G
      data[idx2 + 2] = val2; // B
      data[idx2 + 3] = 255; // A
    }

    ctx.putImageData(imageData, 0, 0);
  });
</script>

{#if device.packetsReceived === 0}
  <i>Please please choose "IR Com" and then "Others" to put your watch in upload mode...</i>
{:else}
  <div class="fields">
    <div class="field-row-stacked" style="width: 120px">
      <label for="fname">Filename</label>
      <input class="input-field" id="fname" type="text" value={filename || "(unknown)"} disabled />
    </div>
    <div class="field-row-stacked" style="width: 120px">
      <label for="date">Date</label>
      <input class="input-field" id="date" type="text" value={formattedDate} disabled />
    </div>
  </div>

  <canvas bind:this={canvas} style="image-rendering: pixelated;"></canvas>
{/if}
<hr />

<button disabled={isActionDisabled}>Save</button>
<button disabled={isActionDisabled} onclick={onCancel}>Cancel</button>

<style>
  .fields {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 8px;

    & > .field-row-stacked {
      margin-top: 0px;
    }
  }

  input.input-field:disabled {
    background: white;
    color: #000;
  }
</style>
