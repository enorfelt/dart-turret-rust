import { InjectionToken } from '@angular/core';

export interface VisionListener {
  frame(data: ArrayBuffer): void;
  closed(): void;
}

export interface VisionLink {
  open(url: string, listener: VisionListener): void;
  close(): void;
}

interface LocalWebSocketInit {
  targetAddressSpace: 'local';
}

// The camera starts streaming to a client once it receives a message from it.
const SUBSCRIBE = new Uint8Array([1]);

type LocalWebSocketConstructor = new (url: string, init: LocalWebSocketInit) => WebSocket;

function openLocalWebSocket(url: string): WebSocket {
  try {
    // Chrome 154+ takes an options bag; the camera is a plain-HTTP device on the local network.
    return new (WebSocket as unknown as LocalWebSocketConstructor)(url, {
      targetAddressSpace: 'local',
    });
  } catch {
    return new WebSocket(url);
  }
}

export class WebSocketVisionLink implements VisionLink {
  private socket: WebSocket | undefined;

  open(url: string, listener: VisionListener): void {
    this.close();
    const socket = openLocalWebSocket(url);
    socket.binaryType = 'arraybuffer';
    socket.onopen = () => socket.send(SUBSCRIBE);
    socket.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) listener.frame(event.data);
    };
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.socket = undefined;
      listener.closed();
    };
    this.socket = socket;
  }

  close(): void {
    const socket = this.socket;
    this.socket = undefined;
    socket?.close();
  }
}

export const VISION_LINK = new InjectionToken<VisionLink>('VisionLink', {
  factory: () => new WebSocketVisionLink(),
});
