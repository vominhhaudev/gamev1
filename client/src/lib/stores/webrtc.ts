import { writable } from 'svelte/store';

export interface WebRTCConfig {
  iceServers: RTCIceServer[];
  sessionId: string;
}

export interface DataChannelMessage {
  type: 'control' | 'state';
  data: any;
  timestamp: number;
}

export interface WebRTCState {
  isConnected: boolean;
  connectionState: RTCPeerConnectionState;
  iceConnectionState: RTCIceConnectionState;
  signalingState: RTCSignalingState;
  controlChannel: RTCDataChannel | null;
  stateChannel: RTCDataChannel | null;
  sessionId: string | null;
  error: string | null;
  isFallback: boolean;
  fallbackReason: string | null;
  transportType: 'webrtc' | 'websocket' | 'none';
  stats: {
    packetsSent: number;
    packetsReceived: number;
    bytesSent: number;
    bytesReceived: number;
    roundTripTime: number;
  };
}

const initialState: WebRTCState = {
  isConnected: false,
  connectionState: 'new',
  iceConnectionState: 'new',
  signalingState: 'stable',
  controlChannel: null,
  stateChannel: null,
  sessionId: null,
  error: null,
  isFallback: false,
  fallbackReason: null,
  transportType: 'none',
  stats: {
    packetsSent: 0,
    packetsReceived: 0,
    bytesSent: 0,
    bytesReceived: 0,
    roundTripTime: 0,
  },
};

export const webrtcStore = writable<WebRTCState>(initialState);

class WebRTCService {
  private peerConnection: RTCPeerConnection | null = null;
  private controlChannel: RTCDataChannel | null = null;
  private stateChannel: RTCDataChannel | null = null;
  private sessionId: string | null = null;
  private config: WebRTCConfig | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 3;

  // Fallback WebSocket connection
  private fallbackWebSocket: WebSocket | null = null;
  private isUsingFallback = false;
  private fallbackReconnectTimer: NodeJS.Timeout | null = null;

  constructor() {
    this.setupEventListeners();
  }

  private setupEventListeners() {
    // Listen for messages from the page to handle WebRTC events
    if (typeof window !== 'undefined') {
      window.addEventListener('message', this.handleWindowMessage.bind(this));
    }
  }

  private handleWindowMessage(event: MessageEvent) {
    if (event.data.type === 'webrtc-signaling') {
      this.handleSignalingMessage(event.data);
    }
  }

  async initialize(config: WebRTCConfig): Promise<void> {
    this.config = config;

    // Try WebRTC first
    const webrtcSuccess = await this.tryWebRTCConnection(config);

    if (!webrtcSuccess) {
      console.log('WebRTC failed, trying WebSocket fallback...');
      await this.initializeWebSocketFallback(config);
    }
  }

  private async tryWebRTCConnection(config: WebRTCConfig): Promise<boolean> {
    try {
      // Create RTCPeerConnection with ICE servers
      this.peerConnection = new RTCPeerConnection({
        iceServers: config.iceServers.length > 0 ? config.iceServers : [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' }
        ]
      });

      // Set up event listeners
      this.setupPeerConnectionListeners();

      // Create DataChannels
      await this.createDataChannels();

      // Update session ID
      this.sessionId = config.sessionId;

      webrtcStore.update(state => ({
        ...state,
        sessionId: config.sessionId,
        signalingState: 'have-local-offer',
        transportType: 'webrtc',
      }));

      // Set a timeout to check if WebRTC connection succeeds
      return new Promise((resolve) => {
        const timeout = setTimeout(() => {
          console.log('WebRTC connection timeout, will fallback to WebSocket');
          resolve(false);
        }, 10000); // 10 second timeout

        // Listen for successful connection
        if (this.peerConnection) {
          this.peerConnection.onconnectionstatechange = () => {
            if (this.peerConnection?.connectionState === 'connected') {
              clearTimeout(timeout);
              console.log('WebRTC connection successful');
              resolve(true);
            } else if (this.peerConnection?.connectionState === 'failed') {
              clearTimeout(timeout);
              console.log('WebRTC connection failed');
              resolve(false);
            }
          };
        }
      });
    } catch (error) {
      console.error('Failed to initialize WebRTC:', error);
      return false;
    }
  }

  private async initializeWebSocketFallback(config: WebRTCConfig): Promise<void> {
    try {
      this.isUsingFallback = true;

      // Initialize WebSocket connection as fallback
      this.fallbackWebSocket = new WebSocket('ws://localhost:8080/ws');

      this.fallbackWebSocket.onopen = () => {
        console.log('WebSocket fallback connected successfully');
        webrtcStore.update(state => ({
          ...state,
          isConnected: true,
          connectionState: 'connected',
          transportType: 'websocket',
          isFallback: true,
          fallbackReason: 'WebRTC connection failed or timed out',
        }));

        // Send fallback notification
        this.sendFallbackNotification();
      };

      this.fallbackWebSocket.onmessage = (event) => {
        // Handle WebSocket messages as fallback
        this.handleFallbackMessage(event.data);
      };

      this.fallbackWebSocket.onclose = () => {
        console.log('WebSocket fallback disconnected');
        webrtcStore.update(state => ({
          ...state,
          isConnected: false,
          connectionState: 'disconnected',
        }));

        // Try to reconnect WebRTC
        this.scheduleWebRTCReconnection(config);
      };

      this.fallbackWebSocket.onerror = (error) => {
        console.error('WebSocket fallback error:', error);
        webrtcStore.update(state => ({
          ...state,
          error: `WebSocket fallback error: ${error}`,
          isFallback: true,
          fallbackReason: 'WebSocket connection failed',
        }));
      };
    } catch (error) {
      console.error('Failed to initialize WebSocket fallback:', error);
      webrtcStore.update(state => ({
        ...state,
        error: `Failed to initialize WebSocket fallback: ${error.message}`,
        isFallback: true,
        fallbackReason: 'Both WebRTC and WebSocket failed',
      }));
    }
  }

  private sendFallbackNotification(): void {
    if (this.fallbackWebSocket && this.fallbackWebSocket.readyState === WebSocket.OPEN) {
      this.fallbackWebSocket.send(JSON.stringify({
        type: 'fallback_notification',
        data: {
          reason: 'WebRTC connection failed, using WebSocket fallback',
          timestamp: Date.now(),
        }
      }));
    }
  }

  private handleFallbackMessage(data: string): void {
    try {
      const message = JSON.parse(data);

      // Update stats for fallback WebSocket
      webrtcStore.update(state => ({
        ...state,
        stats: {
          ...state.stats,
          packetsReceived: state.stats.packetsReceived + 1,
          bytesReceived: state.stats.bytesReceived + data.length,
        },
      }));

      console.log('Received fallback message:', message);
    } catch (error) {
      console.error('Failed to parse fallback message:', error);
    }
  }

  private scheduleWebRTCReconnection(config: WebRTCConfig): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      console.log(`Scheduling WebRTC reconnection attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts}`);

      this.fallbackReconnectTimer = setTimeout(async () => {
        const success = await this.tryWebRTCConnection(config);
        if (success) {
          // WebRTC reconnected successfully, close WebSocket fallback
          this.closeWebSocketFallback();
          this.reconnectAttempts = 0;
        } else {
          // Still failed, schedule another attempt
          this.scheduleWebRTCReconnection(config);
        }
      }, 5000); // Retry after 5 seconds
    } else {
      console.log('Max reconnection attempts reached, staying with WebSocket fallback');
      webrtcStore.update(state => ({
        ...state,
        error: 'WebRTC reconnection failed after maximum attempts, using WebSocket fallback',
      }));
    }
  }

  private closeWebSocketFallback(): void {
    if (this.fallbackWebSocket) {
      this.fallbackWebSocket.close();
      this.fallbackWebSocket = null;
    }

    if (this.fallbackReconnectTimer) {
      clearTimeout(this.fallbackReconnectTimer);
      this.fallbackReconnectTimer = null;
    }

    this.isUsingFallback = false;
    this.reconnectAttempts = 0;

    webrtcStore.update(state => ({
      ...state,
      isFallback: false,
      fallbackReason: null,
      transportType: 'webrtc',
    }));
  }

  private setupPeerConnectionListeners() {
    if (!this.peerConnection) return;

    this.peerConnection.onconnectionstatechange = () => {
      console.log('Connection state changed:', this.peerConnection?.connectionState);
      webrtcStore.update(state => ({
        ...state,
        connectionState: this.peerConnection?.connectionState || 'new',
        isConnected: this.peerConnection?.connectionState === 'connected',
      }));
    };

    this.peerConnection.oniceconnectionstatechange = () => {
      console.log('ICE connection state changed:', this.peerConnection?.iceConnectionState);
      webrtcStore.update(state => ({
        ...state,
        iceConnectionState: this.peerConnection?.iceConnectionState || 'new',
      }));
    };

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate) {
        this.sendSignalingMessage({
          type: 'ice-candidate',
          candidate: event.candidate,
          sessionId: this.sessionId,
        });
      }
    };

    this.peerConnection.ondatachannel = (event) => {
      console.log('DataChannel received:', event.channel.label);
      this.setupDataChannelListeners(event.channel);
    };
  }

  private async createDataChannels() {
    if (!this.peerConnection) return;

    try {
      // Create control channel (ordered, reliable)
      this.controlChannel = this.peerConnection.createDataChannel('control', {
        ordered: true,
        maxRetransmits: 3,
      });

      // Create state channel (unordered, unreliable for position data)
      this.stateChannel = this.peerConnection.createDataChannel('state', {
        ordered: false,
        maxRetransmits: 0, // Unreliable
      });

      this.setupDataChannelListeners(this.controlChannel);
      this.setupDataChannelListeners(this.stateChannel);

      console.log('DataChannels created successfully');
    } catch (error) {
      console.error('Failed to create DataChannels:', error);
      webrtcStore.update(state => ({
        ...state,
        error: `Failed to create DataChannels: ${error.message}`,
      }));
    }
  }

  private setupDataChannelListeners(channel: RTCDataChannel) {
    channel.onopen = () => {
      console.log(`${channel.label} channel opened`);
      if (channel.label === 'control') {
        webrtcStore.update(state => ({
          ...state,
          controlChannel: channel,
        }));
      } else if (channel.label === 'state') {
        webrtcStore.update(state => ({
          ...state,
          stateChannel: channel,
        }));
      }
    };

    channel.onclose = () => {
      console.log(`${channel.label} channel closed`);
    };

    channel.onmessage = (event) => {
      try {
        const message: DataChannelMessage = JSON.parse(event.data);
        this.handleDataChannelMessage(message);
      } catch (error) {
        console.error('Failed to parse DataChannel message:', error);
      }
    };

    channel.onerror = (error) => {
      console.error(`${channel.label} channel error:`, error);
    };
  }

  private handleDataChannelMessage(message: DataChannelMessage) {
    console.log('Received DataChannel message:', message);

    // Update stats
    webrtcStore.update(state => ({
      ...state,
      stats: {
        ...state.stats,
        packetsReceived: state.stats.packetsReceived + 1,
        bytesReceived: state.stats.bytesReceived + JSON.stringify(message).length,
      },
    }));

    // Handle different message types
    switch (message.type) {
      case 'control':
        this.handleControlMessage(message.data);
        break;
      case 'state':
        this.handleStateMessage(message.data);
        break;
    }
  }

  private handleControlMessage(data: any) {
    console.log('Control message:', data);
    // Handle game control events (player actions, etc.)
  }

  private handleStateMessage(data: any) {
    console.log('State message:', data);
    // Handle game state updates (positions, physics, etc.)
  }

  async createOffer(): Promise<void> {
    if (!this.peerConnection) {
      throw new Error('PeerConnection not initialized');
    }

    try {
      console.log('Creating WebRTC offer...');

      // Create actual SDP offer
      const offer = await this.peerConnection.createOffer();
      await this.peerConnection.setLocalDescription(offer);

      console.log('Local description set, sending offer via signaling');

      // Send offer through signaling server
      const response = await this.sendSignalingMessage({
        type: 'offer',
        sdp: offer.sdp!,
        sessionId: this.sessionId,
      });

      if (response && response.session_id) {
        this.sessionId = response.session_id;
        webrtcStore.update(state => ({
          ...state,
          sessionId: response.session_id,
          signalingState: 'have-remote-offer',
        }));
      }

      console.log('Offer created and sent successfully');
    } catch (error) {
      console.error('Failed to create offer:', error);
      webrtcStore.update(state => ({
        ...state,
        error: `Failed to create offer: ${error.message}`,
      }));
    }
  }

  async handleOffer(offer: { sdp: string; sessionId: string }): Promise<void> {
    if (!this.peerConnection) {
      throw new Error('PeerConnection not initialized');
    }

    try {
      await this.peerConnection.setRemoteDescription({
        type: 'offer',
        sdp: offer.sdp,
      });

      const answer = await this.peerConnection.createAnswer();
      await this.peerConnection.setLocalDescription(answer);

      // Send answer through signaling
      await this.sendSignalingMessage({
        type: 'answer',
        sdp: answer.sdp!,
        sessionId: offer.sessionId,
      });

      console.log('Answer created and sent');
    } catch (error) {
      console.error('Failed to handle offer:', error);
      webrtcStore.update(state => ({
        ...state,
        error: `Failed to handle offer: ${error.message}`,
      }));
    }
  }

  async handleAnswer(answer: { sdp: string; sessionId: string }): Promise<void> {
    if (!this.peerConnection) {
      throw new Error('PeerConnection not initialized');
    }

    try {
      await this.peerConnection.setRemoteDescription({
        type: 'answer',
        sdp: answer.sdp,
      });

      console.log('Answer set successfully');
    } catch (error) {
      console.error('Failed to handle answer:', error);
      webrtcStore.update(state => ({
        ...state,
        error: `Failed to handle answer: ${error.message}`,
      }));
    }
  }

  async handleIceCandidate(candidate: RTCIceCandidate, sessionId: string): Promise<void> {
    if (!this.peerConnection) {
      throw new Error('PeerConnection not initialized');
    }

    try {
      await this.peerConnection.addIceCandidate(candidate);
      console.log('ICE candidate added successfully');
    } catch (error) {
      console.error('Failed to handle ICE candidate:', error);
    }
  }

  private async sendSignalingMessage(message: any) {
    console.log('Sending signaling message:', message);

    // Send directly to signaling server (primary method)
    try {
      const endpoint = this.getSignalingEndpoint(message.type);
      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(message.data),
      });

      if (response.ok) {
        const responseData = await response.json();
        console.log('Signaling response:', responseData);

        // Handle the response based on message type
        if (message.type === 'offer' && responseData.session_id) {
          this.sessionId = responseData.session_id;
          webrtcStore.update(state => ({
            ...state,
            sessionId: responseData.session_id,
          }));
        }

        return responseData;
      } else {
        console.error('Signaling request failed:', response.status, response.statusText);
      }
    } catch (error) {
      console.error('Failed to send signaling message:', error);
    }

    return null;
  }

  private getSignalingEndpoint(type: string): string {
    switch (type) {
      case 'offer':
        return '/rtc/offer';
      case 'answer':
        return '/rtc/answer';
      case 'ice-candidate':
        return '/rtc/ice';
      default:
        throw new Error(`Unknown signaling message type: ${type}`);
    }
  }

  sendControlMessage(data: any): void {
    if (this.isUsingFallback && this.fallbackWebSocket && this.fallbackWebSocket.readyState === WebSocket.OPEN) {
      // Use WebSocket fallback
      const message = {
        type: 'control',
        data,
        timestamp: Date.now(),
      };

      this.fallbackWebSocket.send(JSON.stringify(message));

      // Update stats
      webrtcStore.update(state => ({
        ...state,
        stats: {
          ...state.stats,
          packetsSent: state.stats.packetsSent + 1,
          bytesSent: state.stats.bytesSent + JSON.stringify(message).length,
        },
      }));
    } else if (this.controlChannel && this.controlChannel.readyState === 'open') {
      // Use WebRTC DataChannel
      const message: DataChannelMessage = {
        type: 'control',
        data,
        timestamp: Date.now(),
      };

      this.controlChannel.send(JSON.stringify(message));

      // Update stats
      webrtcStore.update(state => ({
        ...state,
        stats: {
          ...state.stats,
          packetsSent: state.stats.packetsSent + 1,
          bytesSent: state.stats.bytesSent + JSON.stringify(message).length,
        },
      }));
    } else {
      console.warn('Neither WebRTC control channel nor WebSocket fallback is ready');
    }
  }

  sendStateMessage(data: any): void {
    if (this.isUsingFallback && this.fallbackWebSocket && this.fallbackWebSocket.readyState === WebSocket.OPEN) {
      // Use WebSocket fallback
      const message = {
        type: 'state',
        data,
        timestamp: Date.now(),
      };

      this.fallbackWebSocket.send(JSON.stringify(message));

      // Update stats
      webrtcStore.update(state => ({
        ...state,
        stats: {
          ...state.stats,
          packetsSent: state.stats.packetsSent + 1,
          bytesSent: state.stats.bytesSent + JSON.stringify(message).length,
        },
      }));
    } else if (this.stateChannel && this.stateChannel.readyState === 'open') {
      // Use WebRTC DataChannel
      const message: DataChannelMessage = {
        type: 'state',
        data,
        timestamp: Date.now(),
      };

      this.stateChannel.send(JSON.stringify(message));

      // Update stats
      webrtcStore.update(state => ({
        ...state,
        stats: {
          ...state.stats,
          packetsSent: state.stats.packetsSent + 1,
          bytesSent: state.stats.bytesSent + JSON.stringify(message).length,
        },
      }));
    } else {
      console.warn('Neither WebRTC state channel nor WebSocket fallback is ready');
    }
  }

  async close(): Promise<void> {
    // Close WebRTC connection
    if (this.controlChannel) {
      this.controlChannel.close();
    }
    if (this.stateChannel) {
      this.stateChannel.close();
    }
    if (this.peerConnection) {
      this.peerConnection.close();
    }

    // Close WebSocket fallback connection
    if (this.fallbackWebSocket) {
      this.fallbackWebSocket.close();
    }

    // Clear reconnection timer
    if (this.fallbackReconnectTimer) {
      clearTimeout(this.fallbackReconnectTimer);
    }

    this.peerConnection = null;
    this.controlChannel = null;
    this.stateChannel = null;
    this.fallbackWebSocket = null;
    this.fallbackReconnectTimer = null;
    this.sessionId = null;
    this.isUsingFallback = false;
    this.reconnectAttempts = 0;

    webrtcStore.set(initialState);
  }

  getConnectionStats(): Promise<RTCStatsReport | null> {
    if (this.isUsingFallback) {
      // For WebSocket fallback, return mock stats or null
      return Promise.resolve(null);
    }
    return this.peerConnection?.getStats() || Promise.resolve(null);
  }
}

// Export singleton instance
export const webrtcService = new WebRTCService();

// Export store for reactive updates
export const webrtcActions = {
  async initialize(config: WebRTCConfig) {
    await webrtcService.initialize(config);
  },

  async createOffer() {
    await webrtcService.createOffer();
  },

  async handleOffer(offer: { sdp: string; sessionId: string }) {
    await webrtcService.handleOffer(offer);
  },

  async handleAnswer(answer: { sdp: string; sessionId: string }) {
    await webrtcService.handleAnswer(answer);
  },

  async handleIceCandidate(candidate: RTCIceCandidate, sessionId: string) {
    await webrtcService.handleIceCandidate(candidate, sessionId);
  },

  sendControlMessage(data: any) {
    webrtcService.sendControlMessage(data);
  },

  sendStateMessage(data: any) {
    webrtcService.sendStateMessage(data);
  },

  async close() {
    await webrtcService.close();
  },

  async getStats() {
    return await webrtcService.getConnectionStats();
  },
};
