import EventEmitter from 'eventemitter3';
import { TransportType, NetworkMessage } from './negotiator';
import { WebRTCTransport, WebRTCConfig, defaultWebRTCConfig } from './webrtc-transport';
import { handleQuantizedSnapshot, handleQuantizedDelta, calculateBandwidthSavings } from '../transport';

export interface WebSocketConfig {
  url: string;
  reconnectInterval: number;
  maxReconnectAttempts: number;
  heartbeatInterval: number;
  compressionThreshold: number;
  // WebRTC fallback configuration
  webrtcConfig?: WebRTCConfig;
  enableWebRTC?: boolean;
}

export interface ConnectionStats {
  latency: number;
  bytesSent: number;
  bytesReceived: number;
  reconnectCount: number;
  uptime: number;
}

export class EnhancedWebSocket extends EventEmitter {
  private ws: WebSocket | null = null;
  private webrtc: WebRTCTransport | null = null;
  private config: WebSocketConfig;
  private currentTransport: TransportType = 'websocket';
  private reconnectAttempts = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private heartbeatTimer: NodeJS.Timeout | null = null;
  private lastHeartbeat = 0;
  private stats: ConnectionStats = {
    latency: 0,
    bytesSent: 0,
    bytesReceived: 0,
    reconnectCount: 0,
    uptime: 0
  };
  private connectedAt = 0;
  private messageQueue: NetworkMessage[] = [];

  constructor(config: WebSocketConfig) {
    super();
    this.config = config;
  }

  /**
   * Connect to transport (WebSocket primary, WebRTC fallback)
   */
  async connect(): Promise<void> {
    if (this.isConnected()) {
      return;
    }

    console.log('🔗 Connecting to transport...');

    try {
      // Try WebSocket first
      await this.connectWebSocket();

      // If WebSocket fails and WebRTC is enabled, try WebRTC
      if (!this.isConnected() && this.config.enableWebRTC && this.config.webrtcConfig) {
        console.log('🔄 WebSocket failed, trying WebRTC fallback...');
        await this.connectWebRTC();
      }

      if (this.isConnected()) {
        console.log(`✅ Connected via ${this.currentTransport}`);
        this.emit('connected');
      } else {
        throw new Error('All transport methods failed');
      }

    } catch (error) {
      console.error('❌ Transport connection failed:', error);
      this.emit('error', error);
      throw error;
    }
  }

  /**
   * Connect to WebRTC transport
   */
  private async connectWebRTC(): Promise<void> {
    if (!this.config.webrtcConfig) {
      throw new Error('WebRTC config not provided');
    }

    console.log('🔗 Connecting WebRTC transport...');

    this.webrtc = new WebRTCTransport(this.config.webrtcConfig);
    this.currentTransport = 'webrtc';

    // Set up WebRTC event handlers
    this.webrtc.on('connected', () => {
      this.connectedAt = Date.now();
      this.stats.connected = true;
      this.emit('connected');
    });

    this.webrtc.on('disconnected', () => {
      this.stats.connected = false;
      this.emit('disconnected');
    });

    this.webrtc.on('message', (message: NetworkMessage) => {
      this.stats.bytesReceived += JSON.stringify(message).length;
      this.emit('message', message);
    });

    this.webrtc.on('error', (error: any) => {
      this.emit('error', error);
    });

    await this.webrtc.connect();
  }

  /**
   * Connect to WebSocket server
   */
  private async connectWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        console.log(`🔌 Connecting to ${this.config.url}...`);
        this.ws = new WebSocket(this.config.url);

        this.ws.onopen = () => {
          console.log('✅ Enhanced WebSocket connected');
          this.connectedAt = Date.now();
          this.reconnectAttempts = 0;
          this.startHeartbeat();

          // Send queued messages
          this.flushMessageQueue();

          this.emit('connected');
          resolve();
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event);
        };

        this.ws.onclose = (event) => {
          console.log(`❌ WebSocket closed: ${event.code} ${event.reason}`);
          this.stopHeartbeat();

          if (event.code !== 1000) { // Not a normal closure
            this.scheduleReconnect();
          }

          this.emit('disconnected', { code: event.code, reason: event.reason });
        };

        this.ws.onerror = (error) => {
          console.error('❌ WebSocket error:', error);
          this.emit('error', error);
          reject(error);
        };

      } catch (error) {
        console.error('❌ Failed to create WebSocket:', error);
        reject(error);
      }
    });
  }

  /**
   * Send message với compression nếu cần
   */
  send(message: NetworkMessage): void {
    // Check if connected via any transport
    if (!this.isConnected()) {
      // Queue message nếu chưa connected
      this.messageQueue.push(message);
      return;
    }

    try {
      const jsonString = JSON.stringify(message);

      // Send via current transport
      if (this.currentTransport === 'webrtc' && this.webrtc) {
        this.webrtc.send(message);
      } else if (this.currentTransport === 'websocket' && this.ws?.readyState === WebSocket.OPEN) {
        if (jsonString.length > this.config.compressionThreshold) {
          // Compress large messages
          this.sendCompressed(jsonString);
        } else {
          this.ws.send(jsonString);
        }
      }

      this.stats.bytesSent += jsonString.length;
      this.emit('messageSent', message);

    } catch (error) {
      console.error('❌ Failed to send message:', error);
      this.emit('error', error);
    }
  }

  /**
   * Send compressed binary message
   */
  private async sendCompressed(text: string): Promise<void> {
    if (!this.ws) return;

    try {
      // Simple compression với lz4
      const compressed = this.compressMessage(text);

      // Send với magic bytes để identify compressed messages
      const messageWithMagic = new Uint8Array([
        0x04, 0x22, // LZ4 magic bytes
        ...compressed
      ]);

      this.ws.send(messageWithMagic);
      this.stats.bytesSent += messageWithMagic.length;

    } catch (error) {
      console.error('❌ Compression failed, sending uncompressed:', error);
      this.ws.send(text);
      this.stats.bytesSent += text.length;
    }
  }

  /**
   * Simple compression (placeholder - would use lz4 library)
   */
  private compressMessage(text: string): Uint8Array {
    // Placeholder - trong thực tế dùng lz4 library
    // return lz4.compress(text);
    return new TextEncoder().encode(text);
  }

  /**
   * Handle incoming messages
   */
  private handleMessage(event: MessageEvent): void {
    try {
      let data: string | ArrayBuffer;

      if (event.data instanceof ArrayBuffer) {
        // Binary message - check nếu compressed
        const bytes = new Uint8Array(event.data);
        if (bytes.length >= 2 && bytes[0] === 0x04 && bytes[1] === 0x22) {
          // Decompress LZ4
          data = this.decompressMessage(bytes.slice(2));
        } else {
          data = event.data;
        }
      } else {
        data = event.data;
      }

      // Parse JSON
      const message: NetworkMessage = typeof data === 'string'
        ? JSON.parse(data)
        : JSON.parse(new TextDecoder().decode(data));

      this.stats.bytesReceived += JSON.stringify(message).length;

      // Handle heartbeat
      if (message.type === 'pong') {
        this.updateLatency();
        return;
      }

      // Handle state messages (snapshot/delta events)
      if (message.type === 'event') {
        const eventData = message.data;

        // Handle snapshot events
        if (eventData.name === 'snapshot') {
          console.log(`📦 Received snapshot event: tick=${eventData.tick}, entities=${eventData.entity_count}`);
          this.emit('snapshot', eventData);
          return;
        }

        // Handle delta events
        if (eventData.name === 'delta') {
          console.log(`🔄 Received delta event: tick=${eventData.tick}, changes=${eventData.change_count}`);
          this.emit('delta', eventData);
          return;
        }

        // Handle quantized snapshot events (when implemented)
        if (eventData.name === 'quantized_snapshot') {
          console.log(`📦 Received quantized snapshot: tick=${eventData.tick}, size=${eventData.size} bytes`);
          this.emit('quantizedSnapshot', eventData);
          return;
        }

        // Handle quantized delta events (when implemented)
        if (eventData.name === 'quantized_delta') {
          console.log(`🔄 Received quantized delta: tick=${eventData.tick}, size=${eventData.size} bytes`);
          this.emit('quantizedDelta', eventData);
          return;
        }
      }

      this.emit('message', message);

    } catch (error) {
      console.error('❌ Failed to parse message:', error);
      this.emit('error', error);
    }
  }

  /**
   * Simple decompression (placeholder)
   */
  private decompressMessage(compressed: Uint8Array): string {
    // Placeholder - trong thực tế dùng lz4 library
    // return lz4.decompress(compressed);
    return new TextDecoder().decode(compressed);
  }

  /**
   * Update latency từ heartbeat
   */
  private updateLatency(): void {
    const now = Date.now();
    this.stats.latency = now - this.lastHeartbeat;
    this.emit('latencyUpdated', this.stats.latency);
  }

  /**
   * Start heartbeat để đo latency và keep connection alive
   */
  private startHeartbeat(): void {
    this.heartbeatTimer = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.lastHeartbeat = Date.now();
        this.send({
          type: 'ping',
          timestamp: this.lastHeartbeat
        } as any);
      }
    }, this.config.heartbeatInterval);
  }

  /**
   * Stop heartbeat timer
   */
  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * Schedule reconnection với exponential backoff
   */
  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      console.error('❌ Max reconnection attempts reached');
      this.emit('maxReconnectAttemptsReached');
      return;
    }

    this.reconnectAttempts++;
    this.stats.reconnectCount++;

    const delay = Math.min(
      this.config.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1),
      30000 // Max 30 seconds
    );

    console.log(`🔄 Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(error => {
        console.error('❌ Reconnection failed:', error);
      });
    }, delay);
  }

  /**
   * Flush queued messages sau khi connect
   */
  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      if (message) {
        this.send(message);
      }
    }
  }

  /**
   * Disconnect và cleanup
   */
  disconnect(): void {
    console.log('🔌 Disconnecting WebSocket...');

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    this.stopHeartbeat();

    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }

    this.messageQueue = [];
    this.emit('disconnected', { code: 1000, reason: 'Client disconnect' });
  }

  /**
   * Get current connection stats
   */
  getStats(): ConnectionStats {
    return {
      ...this.stats,
      uptime: this.connectedAt > 0 ? Date.now() - this.connectedAt : 0
    };
  }

  /**
   * Check nếu connected via any transport
   */
  isConnected(): boolean {
    if (this.currentTransport === 'websocket') {
      return this.ws?.readyState === WebSocket.OPEN;
    } else if (this.currentTransport === 'webrtc') {
      return this.webrtc?.isConnected() ?? false;
    }
    return false;
  }

  /**
   * Get current transport type
   */
  getTransportType(): TransportType {
    return this.currentTransport;
  }
}

/**
 * Default WebSocket configuration with WebRTC fallback
 */
export const defaultWebSocketConfig: WebSocketConfig = {
  url: 'ws://localhost:8080/ws',
  reconnectInterval: 1000,
  maxReconnectAttempts: 5,
  heartbeatInterval: 30000,
  compressionThreshold: 1000,
  enableWebRTC: true,
  webrtcConfig: {
    ...defaultWebRTCConfig,
    signalingUrl: 'http://localhost:8080'
  }
};

/**
 * Create enhanced WebSocket với config mặc định
 */
export function createEnhancedWebSocket(config?: Partial<WebSocketConfig>): EnhancedWebSocket {
  const fullConfig = { ...defaultWebSocketConfig, ...config };
  return new EnhancedWebSocket(fullConfig);
}
