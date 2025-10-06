export interface WebTransportConfig {
  serverUrl: string;
  sessionId: string;
  timeout?: number;
  maxRetries?: number;
}

export interface WebTransportMessage {
  id: string;
  type: 'control' | 'state' | 'system';
  payload: any;
  timestamp: number;
}

export class WebTransportClient {
  private transport: WebTransport | null = null;
  private config: WebTransportConfig;
  private isConnected = false;
  private messageHandlers: ((message: WebTransportMessage) => void)[] = [];
  private reconnectTimer: NodeJS.Timeout | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts: number;

  constructor(config: WebTransportConfig) {
    this.config = {
      timeout: 5000,
      maxRetries: 3,
      ...config
    };
    this.maxReconnectAttempts = this.config.maxRetries!;
  }

  async connect(): Promise<void> {
    try {
      console.log('🔗 Connecting to WebTransport server:', this.config.serverUrl);

      // Create WebTransport connection
      this.transport = new WebTransport(this.config.serverUrl);

      // Wait for connection
      await this.transport.ready;
      this.isConnected = true;
      this.reconnectAttempts = 0;

      console.log('✅ WebTransport connected successfully');

      // Set up datagram handling
      this.setupDatagramHandling();

      // Set up stream handling
      this.setupStreamHandling();

      // Handle connection close
      this.transport.closed.then(() => {
        console.log('🔌 WebTransport connection closed');
        this.isConnected = false;
        this.handleReconnect();
      }).catch((error) => {
        console.error('❌ WebTransport connection error:', error);
        this.isConnected = false;
        this.handleReconnect();
      });

    } catch (error) {
      console.error('❌ Failed to connect to WebTransport:', error);
      this.isConnected = false;
      throw error;
    }
  }

  private setupDatagramHandling(): void {
    if (!this.transport) return;

    try {
      const datagramReader = this.transport.datagrams.readable.getReader();

      // Handle incoming datagrams
      this.readDatagrams(datagramReader);
    } catch (error) {
      console.error('❌ Failed to setup datagram handling:', error);
    }
  }

  private async readDatagrams(reader: ReadableStreamDefaultReader<Uint8Array>): Promise<void> {
    try {
      while (this.isConnected) {
        const { value: datagram, done } = await reader.read();

        if (done) {
          console.log('📡 Datagram stream ended');
          break;
        }

        // Process incoming datagram
        this.processDatagram(datagram);
      }
    } catch (error) {
      console.error('❌ Error reading datagrams:', error);
    } finally {
      reader.releaseLock();
    }
  }

  private processDatagram(data: Uint8Array): void {
    try {
      // Decode message (assume JSON for simplicity)
      const messageString = new TextDecoder().decode(data);
      const message: WebTransportMessage = JSON.parse(messageString);

      console.log('📨 Received WebTransport message:', message);

      // Notify handlers
      this.messageHandlers.forEach(handler => {
        try {
          handler(message);
        } catch (error) {
          console.error('❌ Error in message handler:', error);
        }
      });
    } catch (error) {
      console.error('❌ Failed to process datagram:', error);
    }
  }

  private setupStreamHandling(): void {
    if (!this.transport) return;

    // Create bidirectional stream for control messages
    this.createBidirectionalStream();
  }

  private async createBidirectionalStream(): Promise<void> {
    if (!this.transport) return;

    try {
      const stream = await this.transport.createBidirectionalStream();

      // Handle incoming stream data
      this.handleIncomingStream(stream.readable);

      // Store writable for sending messages
      (this.transport as any)._controlStream = stream.writable;

      console.log('🔄 Bidirectional stream created');
    } catch (error) {
      console.error('❌ Failed to create bidirectional stream:', error);
    }
  }

  private async handleIncomingStream(readable: ReadableStream<Uint8Array>): Promise<void> {
    const reader = readable.getReader();

    try {
      while (this.isConnected) {
        const { value, done } = await reader.read();

        if (done) {
          console.log('📡 Control stream ended');
          break;
        }

        // Process stream data (JSON messages)
        const messageString = new TextDecoder().decode(value);
        const message: WebTransportMessage = JSON.parse(messageString);

        console.log('📨 Received stream message:', message);

        // Handle special control messages
        if (message.type === 'control') {
          await this.handleControlMessage(message);
        } else {
          // Notify regular message handlers
          this.messageHandlers.forEach(handler => {
            try {
              handler(message);
            } catch (error) {
              console.error('❌ Error in message handler:', error);
            }
          });
        }
      }
    } catch (error) {
      console.error('❌ Error reading control stream:', error);
    } finally {
      reader.releaseLock();
    }
  }

  private async handleControlMessage(message: WebTransportMessage): Promise<void> {
    switch (message.payload?.command) {
      case 'ping':
        await this.sendControlMessage({
          type: 'control',
          payload: { command: 'pong', timestamp: Date.now() }
        });
        break;

      case 'close':
        console.log('🔌 Received close command');
        await this.disconnect();
        break;

      default:
        console.log('📨 Received control message:', message);
    }
  }

  async sendMessage(message: WebTransportMessage): Promise<void> {
    if (!this.isConnected || !this.transport) {
      throw new Error('WebTransport not connected');
    }

    try {
      // Send as datagram for state messages (unreliable, but fast)
      if (message.type === 'state') {
        await this.sendDatagram(message);
      }
      // Send as stream for control messages (reliable)
      else {
        await this.sendControlMessage(message);
      }

      console.log('📤 Sent WebTransport message:', message.type);
    } catch (error) {
      console.error('❌ Failed to send message:', error);
      throw error;
    }
  }

  private async sendDatagram(message: WebTransportMessage): Promise<void> {
    if (!this.transport) return;

    try {
      const messageString = JSON.stringify(message);
      const data = new TextEncoder().encode(messageString);

      // Send as datagram (unreliable transport)
      await this.transport.sendDatagram(data);
    } catch (error) {
      console.error('❌ Failed to send datagram:', error);
      throw error;
    }
  }

  private async sendControlMessage(message: WebTransportMessage): Promise<void> {
    if (!this.transport) return;

    try {
      const messageString = JSON.stringify(message);
      const data = new TextEncoder().encode(messageString);

      // Send via bidirectional stream (reliable transport)
      if ((this.transport as any)._controlStream) {
        const writer = (this.transport as any)._controlStream.getWriter();
        await writer.write(data);
        writer.releaseLock();
      } else {
        // Fallback to datagram for control messages if stream not available
        await this.sendDatagram(message);
      }
    } catch (error) {
      console.error('❌ Failed to send control message:', error);
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    console.log('🔌 Disconnecting WebTransport...');

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    this.isConnected = false;

    if (this.transport) {
      try {
        await this.transport.close();
      } catch (error) {
        console.error('❌ Error closing WebTransport:', error);
      }
      this.transport = null;
    }
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('❌ Max reconnection attempts reached');
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 10000); // Exponential backoff

    console.log(`🔄 Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

    this.reconnectTimer = setTimeout(async () => {
      try {
        await this.connect();
      } catch (error) {
        console.error('❌ Reconnection failed:', error);
      }
    }, delay);
  }

  // Event handling
  onMessage(handler: (message: WebTransportMessage) => void): () => void {
    this.messageHandlers.push(handler);

    return () => {
      const index = this.messageHandlers.indexOf(handler);
      if (index > -1) {
        this.messageHandlers.splice(index, 1);
      }
    };
  }

  // Getters
  getConnectionState(): 'disconnected' | 'connecting' | 'connected' {
    if (!this.transport) return 'disconnected';
    if (this.isConnected) return 'connected';
    return 'connecting';
  }

  isAvailable(): boolean {
    return typeof WebTransport !== 'undefined';
  }

  getStats(): { type: string; connected: boolean; reconnectAttempts: number } {
    return {
      type: 'webtransport',
      connected: this.isConnected,
      reconnectAttempts: this.reconnectAttempts
    };
  }
}
