import { invoke, Channel } from "@tauri-apps/api/core";

const IMG_BLOB_SZ = 7229;

type Status = "ready" | "downloading";

export async function getSerialPorts(): Promise<string[]> {
  const ports = await invoke<string[]>("list_serial_ports");
  return ports;
}

type DownloadEvent =
  | {
      event: "packetsReceived";
      data: {
        packets: number;
      };
    }
  | {
      event: "chunk";
      data: {
        offset: number;
        chunk: Uint8Array;
      };
    };

export class WatchDevice {
  status = $state<Status>("ready");

  packetsReceived = $state(0);
  imgBytesReceived = $state(0);

  port: string;

  constructor(private portName: string) {
    this.port = portName;
    this.imgData = new Uint8Array(IMG_BLOB_SZ);
  }

  imgData: Uint8Array;

  async download() {
    this.status = "downloading";

    const channel = new Channel<DownloadEvent>();
    channel.onmessage = ({ event, data }) => {
      if (event === "packetsReceived") {
        this.packetsReceived = data.packets;
      } else if (event === "chunk") {
        this.imgData.set(data.chunk, data.offset);
        this.imgBytesReceived = Math.max(this.imgBytesReceived, data.offset + data.chunk.length);
      }
    };

    try {
      const { blob } = await invoke<{ blob: Uint8Array }>("download_image_from_watch", {
        portPath: this.port,
        onEvent: channel,
      });

      this.imgData.set(blob.slice(0, IMG_BLOB_SZ));
      this.imgBytesReceived = IMG_BLOB_SZ;

      return blob;
    } finally {
      this.status = "ready";
    }
  }
}
