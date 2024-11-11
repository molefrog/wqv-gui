<script lang="ts">
  import { WatchDevice, getSerialPorts } from "./device.svelte";
  import Photo from "./Photo.svelte";
  import ico_faxWarn from "./icons/fax_warning.png";
  import Titlebar from "./Titlebar.svelte";

  let serialPorts = $state<string[]>([]);
  let device = $state<WatchDevice>();
  let selectedPort = $state<string>();

  async function pollPorts() {
    serialPorts = await getSerialPorts();

    if (!serialPorts.length && !device) {
      selectedPort = serialPorts[0];
    }

    if (device && !serialPorts.includes(device.port)) {
      device = undefined;
    }

    setTimeout(pollPorts, 1000);
  }
  pollPorts();

  async function download() {
    if (!selectedPort) return;
    device = new WatchDevice(selectedPort);

    try {
      await device.download();
    } catch (e) {
      device = undefined;
      alert(`Error: ${e}`);
    }
  }
</script>

<div class="app-win window window-main">
  <Titlebar />

  <div class="app-win-body window-body">
    {#if !device}
      {#if serialPorts.length === 0}
        <p class="connect-device">
          <img src={ico_faxWarn} alt="Warning" width="24" height="24" />
          Please connect the IR adapter to the computer...
        </p>
      {:else}
        <div class="port-select">
          <select bind:value={selectedPort}>
            {#each serialPorts as port}
              <option value={port}>{port}</option>
            {/each}
          </select>
        </div>

        <button onclick={download}>Download image</button>
      {/if}
    {:else}
      <Photo {device} onCancel={() => (device = undefined)} />
    {/if}
  </div>

  <!-- global app status bar -->
  <div class="app-win-status status-bar">
    <p class="status-bar-field">
      {#if device && device.status === "ready"}
        Ready
      {:else if device && device.status === "downloading"}
        Downloading...
      {:else if !device && serialPorts.length}
        Select serial port
      {:else}
        Connect device!
      {/if}
    </p>
    <p class="status-bar-field">
      {#if device && device.status === "downloading"}
        {device.packetsReceived} PKT
        {#if device.imgBytesReceived}
          - {device.imgBytesReceived}B / 7229B
        {/if}
      {/if}
    </p>
  </div>
</div>

<style>
  .app-win {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .app-win-body {
    flex: 1 1 auto;
    overflow-y: auto;
  }

  .app-win-status {
    flex-shrink: 0;
  }

  .port-select {
    margin-bottom: 4px;
  }

  .connect-device {
    display: flex;
    align-items: center;
    gap: 10px;
  }
</style>
