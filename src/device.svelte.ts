import { invoke } from "@tauri-apps/api/core";

type Status = "ready" | "downloading";

export async function getSerialPorts(): Promise<string[]> {
  const ports = await invoke<string[]>("list_serial_ports");
  return ports;
}

const IMG_SIZE = 7229;

export class WatchDevice {
  status = $state<Status>("ready");
  packetsReceived = $state(0);
  imgBytesReceived = $state(0);

  port: string;

  constructor(private portName: string) {
    this.port = portName;
    this.imgData = new Uint8Array(IMG_SIZE);
  }

  imgData: Uint8Array;

  async download() {
    const { blob } = await invoke<{ blob: Uint8Array }>("download_image_from_watch", {
      portPath: this.port,
    });

    this.imgData.set(blob.slice(0, IMG_SIZE));
    this.imgBytesReceived = IMG_SIZE;
    this.packetsReceived++;
  }
}
