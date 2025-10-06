import EventEmitter from 'eventemitter3';
import { NetworkMessage } from './negotiator';
import { handleQuantizedSnapshot, handleQuantizedDelta } from '../transport';

export interface WebRTCConfig {
  roomId: string;
  peerId: string;
  signalingUrl: string;
  iceServers: RTCIceServer[];
  dataChannelConfig?: RTCDataChannelInit;
}

export interface WebRTCStats {
  latency: number;
  bytesSent: number;
  bytesReceived: number;
  reconnectCount: number;
  uptime: number;
  connected: boolean;
  datachannelState: string;
}

export class WebRTCTransport extends EventEmitter {
  private config: WebRTCConfig;
  private peerConnection: RTCPeerConnection | null = null;
  private controlChannel: RTCDataChannel | null = null;
  private stateChannel: RTCDataChannel | null = null;
  private stats: WebRTCStats = {
    latency: 0,
    bytesSent: 0,
    bytesReceived: 0,
    reconnectCount: 0,
    uptime: 0,
    connected: false,
    datachannelState: 'closed'
  };
  private connectedAt = 0;
  private messageQueue: NetworkMessage[] = [];
  private signalingInProgress = false;

  constructor(config: WebRTCConfig) {
    super();
    this.config = config;
  }

  /**
   * Connect to WebRTC signaling server and establish peer connection
   */
  async connect(): Promise<void> {
    if (this.peerConnection?.connectionState === 'connected') {
      return;
    }

    console.log('🔗 Connecting WebRTC transport...');

    try {
      // Create peer connection
      this.peerConnection = new RTCPeerConnection({
        iceServers: this.config.iceServers
      });

      // Set up event handlers
      this.setupPeerConnectionHandlers();

      // Create offer and send to signaling server
      await this.createAndSendOffer();

      // Wait for connection to be established
      await this.waitForConnection();

      console.log('✅ WebRTC transport connected successfully');
      this.emit('connected');

    } catch (error) {
      console.error('❌ WebRTC connection failed:', error);
      this.emit('error', error);
      throw error;
    }
  }

  /**
   * Disconnect from WebRTC
   */
  disconnect(): void {
    console.log('🔌 Disconnecting WebRTC transport...');

    if (this.peerConnection) {
      this.peerConnection.close();
      this.peerConnection = null;
    }

    this.controlChannel = null;
    this.stateChannel = null;

    this.stats.connected = false;
    this.signalingInProgress = false;

    this.emit('disconnected');
  }

  /**
   * Send message through appropriate DataChannel
   */
  send(message: NetworkMessage): void {
    if (!this.isConnected()) {
      console.warn('⚠️ Cannot send message: WebRTC not connected');
      return;
    }

    try {
      const jsonMessage = JSON.stringify(message);

      // Route to appropriate DataChannel based on message type
      if (message.type === 'input' || message.type === 'control') {
        if (this.controlChannel?.readyState === 'open') {
          this.controlChannel.send(jsonMessage);
          this.updateStats('sent', jsonMessage.length);
        } else {
          console.warn('⚠️ Control DataChannel not ready');
        }
      } else {
        if (this.stateChannel?.readyState === 'open') {
          this.stateChannel.send(jsonMessage);
          this.updateStats('sent', jsonMessage.length);
        } else {
          console.warn('⚠️ State DataChannel not ready');
        }
      }
    } catch (error) {
      console.error('❌ Failed to send WebRTC message:', error);
      this.emit('error', error);
    }
  }

  /**
   * Check if WebRTC is connected
   */
  isConnected(): boolean {
    return this.stats.connected && this.peerConnection?.connectionState === 'connected';
  }

  /**
   * Get current connection statistics
   */
  getStats(): WebRTCStats {
    return { ...this.stats };
  }

  /**
   * Set up RTCPeerConnection event handlers
   */
  private setupPeerConnectionHandlers(): void {
    if (!this.peerConnection) return;

    this.peerConnection.onconnectionstatechange = () => {
      console.log('🔄 WebRTC connection state:', this.peerConnection?.connectionState);
      this.stats.connected = this.peerConnection?.connectionState === 'connected';

      if (this.peerConnection?.connectionState === 'connected') {
        this.connectedAt = Date.now();
        this.emit('connected');
      } else if (this.peerConnection?.connectionState === 'disconnected' ||
                 this.peerConnection?.connectionState === 'failed' ||
                 this.peerConnection?.connectionState === 'closed') {
        this.emit('disconnected');
      }
    };

    this.peerConnection.ondatachannel = (event) => {
      console.log('📡 DataChannel received:', event.channel.label);
      this.setupDataChannelHandlers(event.channel);
    };

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate) {
        this.sendIceCandidate(event.candidate);
      }
    };

    this.peerConnection.oniceconnectionstatechange = () => {
      console.log('🧊 ICE connection state:', this.peerConnection?.iceConnectionState);
    };
  }

  /**
   * Create offer and send to signaling server
   */
  private async createAndSendOffer(): Promise<void> {
    if (!this.peerConnection) throw new Error('Peer connection not initialized');

    this.signalingInProgress = true;

    try {
      // Create DataChannels
      this.controlChannel = this.peerConnection.createDataChannel('control', {
        ordered: true,
        maxRetransmits: 0 // Reliable for control messages
      });

      this.stateChannel = this.peerConnection.createDataChannel('state', {
        ordered: false,
        maxRetransmits: 0 // Unreliable for state updates
      });

      // Set up DataChannel handlers
      this.setupDataChannelHandlers(this.controlChannel);
      this.setupDataChannelHandlers(this.stateChannel);

      // Create offer
      const offer = await this.peerConnection.createOffer();
      await this.peerConnection.setLocalDescription(offer);

      // Send offer to signaling server
      const response = await fetch(`${this.config.signalingUrl}/rtc/offer`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          room_id: this.config.roomId,
          peer_id: this.config.peerId,
          offer: {
            type: offer.type,
            sdp: offer.sdp
          }
        })
      });

      if (!response.ok) {
        throw new Error(`Signaling server error: ${response.status}`);
      }

      const result = await response.json();

      if (result.success && result.answer) {
        // Set remote description
        await this.peerConnection.setRemoteDescription({
          type: 'answer',
          sdp: result.answer.sdp
        });

        // Add ICE candidates
        if (result.ice_candidates) {
          for (const candidate of result.ice_candidates) {
            await this.peerConnection.addIceCandidate({
              candidate: candidate.candidate,
              sdpMid: candidate.sdpMid,
              sdpMLineIndex: candidate.sdpMLineIndex,
              usernameFragment: candidate.usernameFragment
            });
          }
        }
      }

    } catch (error) {
      this.signalingInProgress = false;
      throw error;
    }

    this.signalingInProgress = false;
  }

  /**
   * Set up DataChannel event handlers
   */
  private setupDataChannelHandlers(channel: RTCDataChannel): void {
    channel.onopen = () => {
      console.log(`✅ DataChannel ${channel.label} opened`);
      this.stats.datachannelState = 'open';

      // Process queued messages
      this.processMessageQueue();
    };

    channel.onclose = () => {
      console.log(`❌ DataChannel ${channel.label} closed`);
      this.stats.datachannelState = 'closed';
    };

    channel.onerror = (error) => {
      console.error(`❌ DataChannel ${channel.label} error:`, error);
      this.emit('error', error);
    };

    channel.onmessage = (event) => {
      try {
        const message: NetworkMessage = JSON.parse(event.data);
        this.updateStats('received', event.data.length);

        // Handle state messages (snapshot/delta events)
        if (message.type === 'event') {
          const eventData = message.data;

          // Handle snapshot events
          if (eventData.name === 'snapshot') {
            console.log(`📦 Received WebRTC snapshot event: tick=${eventData.tick}, entities=${eventData.entity_count}`);
            this.emit('snapshot', eventData);
            return;
          }

          // Handle delta events
          if (eventData.name === 'delta') {
            console.log(`🔄 Received WebRTC delta event: tick=${eventData.tick}, changes=${eventData.change_count}`);
            this.emit('delta', eventData);
            return;
          }

          // Handle quantized snapshot events (when implemented)
          if (eventData.name === 'quantized_snapshot') {
            console.log(`📦 Received WebRTC quantized snapshot: tick=${eventData.tick}, size=${eventData.size} bytes`);
            this.emit('quantizedSnapshot', eventData);
            return;
          }

          // Handle quantized delta events (when implemented)
          if (eventData.name === 'quantized_delta') {
            console.log(`🔄 Received WebRTC quantized delta: tick=${eventData.tick}, size=${eventData.size} bytes`);
            this.emit('quantizedDelta', eventData);
            return;
          }
        }

        this.emit('message', message);
      } catch (error) {
        console.error('❌ Failed to parse WebRTC message:', error);
        this.emit('error', error);
      }
    };
  }

  /**
   * Send ICE candidate to signaling server
   */
  private async sendIceCandidate(candidate: RTCIceCandidate): Promise<void> {
    try {
      await fetch(`${this.config.signalingUrl}/rtc/ice`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          room_id: this.config.roomId,
          peer_id: this.config.peerId,
          candidate: {
            candidate: candidate.candidate,
            sdpMid: candidate.sdpMid,
            sdpMLineIndex: candidate.sdpMLineIndex,
            usernameFragment: candidate.usernameFragment
          }
        })
      });
    } catch (error) {
      console.warn('⚠️ Failed to send ICE candidate:', error);
    }
  }

  /**
   * Wait for WebRTC connection to be established
   */
  private async waitForConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.peerConnection) {
        reject(new Error('Peer connection not initialized'));
        return;
      }

      const timeout = setTimeout(() => {
        reject(new Error('WebRTC connection timeout'));
      }, 10000); // 10 second timeout

      const checkConnection = () => {
        if (this.peerConnection?.connectionState === 'connected') {
          clearTimeout(timeout);
          resolve();
        } else if (this.peerConnection?.connectionState === 'failed' ||
                   this.peerConnection?.connectionState === 'disconnected') {
          clearTimeout(timeout);
          reject(new Error('WebRTC connection failed'));
        } else {
          setTimeout(checkConnection, 100);
        }
      };

      checkConnection();
    });
  }

  /**
   * Process queued messages when DataChannels are ready
   */
  private processMessageQueue(): void {
    if (this.messageQueue.length === 0) return;

    console.log(`📨 Processing ${this.messageQueue.length} queued messages`);

    for (const message of this.messageQueue) {
      this.send(message);
    }

    this.messageQueue = [];
  }

  /**
   * Update connection statistics
   */
  private updateStats(type: 'sent' | 'received', bytes: number): void {
    if (type === 'sent') {
      this.stats.bytesSent += bytes;
    } else {
      this.stats.bytesReceived += bytes;
    }

    if (this.connectedAt > 0) {
      this.stats.uptime = Date.now() - this.connectedAt;
    }
  }
}

/**
 * Default WebRTC configuration
 */
export const defaultWebRTCConfig: WebRTCConfig = {
  roomId: 'default-room',
  peerId: 'client-peer',
  signalingUrl: 'http://localhost:8080',
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun1.l.google.com:19302' }
  ],
  dataChannelConfig: {
    ordered: true,
    maxRetransmits: 0
  }
};
