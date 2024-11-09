<script lang="ts">
  import { onMount } from "svelte";
  import { WatchDevice } from "./device.svelte";

  const FN_SZ = 24,
    DATE_SZ = 5;

  let {
    device,
  }: {
    device: WatchDevice;
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

  /**
   * rendering on a canvas
   */
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;

  onMount(() => {
    ctx = canvas.getContext("2d")!;
    canvas.width = 120;
    canvas.height = 120;
  });

  $effect(() => {
    // Re-render when bytes received changes
    const buffer = device.imgData.slice(FN_SZ + DATE_SZ, device.imgBytesReceived);
    if (!ctx) return;

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

<div>
  Filename: {filename}
  {date}
</div>
<canvas bind:this={canvas} style="image-rendering: pixelated;"></canvas>
