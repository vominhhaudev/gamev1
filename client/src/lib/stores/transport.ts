import { writable } from 'svelte/store';
import { webrtcStore, webrtcActions } from './webrtc';

export interface TransportConfig {
  type: 'webrtc' | 'websocket' | 'quic';
  endpoint?: string;
  iceServers?: RTCIceServer[];
  sessionId?: string;
  priority: number;
}

export interface TransportMessage {
  id: string;
  type: 'control' | 'state' | 'system';
  payload: any;
  timestamp: number;
  transportType: string;
  sessionId?: string;
}

export interface TransportStats {
  transportId: string;
  transportType: string;
  connectionState: 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'failed';
  uptimeSeconds: number;
  bytesSent: number;
  bytesReceived: number;
  messagesSent: number;
  messagesReceived: number;
  packetsSent: number;
  packetsReceived: number;
  averageLatencyMs: number;
  packetLossRate: number;
  reconnectCount: number;
  lastActivity: number;
  errorCount: number;
}

export interface TransportState {
  activeTransports: Map<string, TransportStats>;
  bestTransport: string | null;
  isConnected: boolean;
  totalMessagesSent: number;
  totalMessagesReceived: number;
  totalBytesSent: number;
  totalBytesReceived: number;
  averageLatencyMs: number;
  errorCount: number;
  reconnectCount: number;
}

const initialState: TransportState = {
  activeTransports: new Map(),
  bestTransport: null,
  isConnected: false,
  totalMessagesSent: 0,
  totalMessagesReceived: 0,
  totalBytesSent: 0,
  totalBytesReceived: 0,
  averageLatencyMs: 0,
  errorCount: 0,
  reconnectCount: 0,
};

export const transportStore = writable<TransportState>(initialState);

class TransportManager {
  private transports: Map<string, any> = new Map();
  private messageHandlers: Map<string, (message: TransportMessage) => void> = new Map();
  private eventListeners: ((event: any) => void)[] = [];
  private reconnectTimers: Map<string, NodeJS.Timeout> = new Map();

  constructor() {
    this.setupEventListeners();
  }

  private setupEventListeners() {
    // Listen for transport events from the page
    if (typeof window !== 'undefined') {
      window.addEventListener('message', this.handleWindowMessage.bind(this));
    }
  }

  private handleWindowMessage(event: MessageEvent) {
    if (event.data.type === 'transport-event') {
      this.handleTransportEvent(event.data.event);
    }
  }

  private handleTransportEvent(event: any) {
    console.log('Transport event:', event);

    // Update transport state
    transportStore.update(state => {
      const newState = { ...state };

      switch (event.type) {
        case 'connected':
          newState.isConnected = true;
          newState.activeTransports.set(event.transportId, {
            transportId: event.transportId,
            transportType: event.transportType,
            connectionState: 'connected',
            uptimeSeconds: 0,
            bytesSent: 0,
            bytesReceived: 0,
            messagesSent: 0,
            messagesReceived: 0,
            packetsSent: 0,
            packetsReceived: 0,
            averageLatencyMs: 0,
            packetLossRate: 0,
            reconnectCount: 0,
            lastActivity: Date.now(),
            errorCount: 0,
          });
          break;

        case 'disconnected':
          newState.isConnected = Array.from(newState.activeTransports.values()).some(t => t.connectionState === 'connected');
          newState.activeTransports.delete(event.transportId);
          break;

        case 'messageSent':
          newState.totalMessagesSent++;
          newState.totalBytesSent += event.size;
          break;

        case 'messageReceived':
          newState.totalMessagesReceived++;
          newState.totalBytesReceived += event.size;
          break;

        case 'error':
          newState.errorCount++;
          break;

        case 'reconnecting':
          newState.reconnectCount++;
          break;
      }

      return newState;
    });

    // Notify listeners
    this.eventListeners.forEach(listener => listener(event));
  }

  async addTransport(config: TransportConfig): Promise<string> {
    const transportId = `transport_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    console.log('Adding transport:', transportId, config);

    try {
      if (config.type === 'webrtc') {
        // Handle WebRTC transport creation
        await this.createWebRTCTransport(transportId, config);
      } else if (config.type === 'websocket') {
        // Handle WebSocket transport creation
        await this.createWebSocketTransport(transportId, config);
      } else {
        // Send message to parent window for other transport types
        if (typeof window !== 'undefined') {
          window.postMessage({
            type: 'transport-init',
            transportId,
            config,
          }, '*');
        }
      }

      return transportId;
    } catch (error) {
      console.error('Failed to add transport:', error);
      throw error;
    }
  }

  private async createWebRTCTransport(transportId: string, config: TransportConfig): Promise<void> {
    try {
      // Initialize WebRTC with fallback
      await webrtcActions.initialize({
        iceServers: config.iceServers || [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' }
        ],
        sessionId: `session_${transportId}`,
      });

      // Subscribe to WebRTC events and forward to transport manager
      webrtcStore.subscribe(state => {
        if (state.isConnected) {
          this.handleTransportEvent({
            type: 'connected',
            transportId,
            transportType: state.isFallback ? 'websocket' : 'webrtc',
          });
        } else if (state.error) {
          this.handleTransportEvent({
            type: 'error',
            transportId,
            error: state.error,
          });
        }
      });

      console.log('WebRTC transport initialized:', transportId);
    } catch (error) {
      console.error('Failed to create WebRTC transport:', error);
      throw error;
    }
  }

  private async createWebSocketTransport(transportId: string, config: TransportConfig): Promise<void> {
    try {
      // Create WebSocket connection directly
      const ws = new WebSocket(config.endpoint || 'ws://localhost:8080/ws');

      ws.onopen = () => {
        this.handleTransportEvent({
          type: 'connected',
          transportId,
          transportType: 'websocket',
        });

        // Set up message handling
        ws.onmessage = (event) => {
          this.handleTransportEvent({
            type: 'messageReceived',
            transportId,
            size: event.data.length,
          });
        };

        ws.onclose = () => {
          this.handleTransportEvent({
            type: 'disconnected',
            transportId,
          });
        };

        ws.onerror = (error) => {
          this.handleTransportEvent({
            type: 'error',
            transportId,
            error: error.toString(),
          });
        };
      };

      // Store WebSocket reference for later use
      this.transports.set(transportId, { type: 'websocket', connection: ws });

      console.log('WebSocket transport initialized:', transportId);
    } catch (error) {
      console.error('Failed to create WebSocket transport:', error);
      throw error;
    }
  }

  async removeTransport(transportId: string): Promise<void> {
    console.log('Removing transport:', transportId);

    // Send message to parent window to close transport
    if (typeof window !== 'undefined') {
      window.postMessage({
        type: 'transport-close',
        transportId,
      }, '*');
    }

    this.transports.delete(transportId);
  }

  async sendMessage(message: TransportMessage): Promise<void> {
    console.log('Sending transport message:', message);

    // Check if we have WebRTC available and use it for game input
    const webrtcState = webrtcStore.getState?.();
    if (webrtcState?.isConnected && webrtcState?.transportType === 'webrtc') {
      // Send as game input to worker via gateway
      await this.sendGameInput(message);
      return;
    }

    // Fallback to transport manager for other transport types
    const state = transportStore.getState?.();
    if (state?.bestTransport) {
      if (typeof window !== 'undefined') {
        window.postMessage({
          type: 'transport-message',
          transportId: state.bestTransport,
          message,
        }, '*');
      }
    } else {
      console.warn('No active transport available');
    }
  }

  private async sendGameInput(message: TransportMessage): Promise<void> {
    try {
      const response = await fetch('/game/input', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          room_id: 'default_room', // TODO: Get from actual room
          sequence: Date.now(),
          input: message.payload,
        }),
      });

      if (response.ok) {
        const result = await response.json();
        if (result.success && result.snapshot) {
          // Handle snapshot from worker
          console.log('Received game snapshot:', result.snapshot);
          // TODO: Process snapshot and update game state
        }
      } else {
        console.error('Failed to send game input:', response.statusText);
      }
    } catch (error) {
      console.error('Error sending game input:', error);
    }
  }

  onMessage(handler: (message: TransportMessage) => void): () => void {
    const id = Math.random().toString(36).substr(2, 9);
    this.messageHandlers.set(id, handler);

    return () => {
      this.messageHandlers.delete(id);
    };
  }

  onEvent(listener: (event: any) => void): () => void {
    this.eventListeners.push(listener);

    return () => {
      const index = this.eventListeners.indexOf(listener);
      if (index > -1) {
        this.eventListeners.splice(index, 1);
      }
    };
  }

  getActiveTransports(): string[] {
    const state = transportStore.getState?.();
    return Array.from(state?.activeTransports.keys() || []);
  }

  getBestTransport(): string | null {
    const state = transportStore.getState?.();
    return state?.bestTransport || null;
  }

  getStats(): TransportState {
    return transportStore.getState?.() || initialState;
  }
}

// Export singleton instance
export const transportManager = new TransportManager();

// Export actions for easy use
export const transportActions = {
  async addTransport(config: TransportConfig): Promise<string> {
    return await transportManager.addTransport(config);
  },

  async removeTransport(transportId: string): Promise<void> {
    return await transportManager.removeTransport(transportId);
  },

  async sendMessage(message: TransportMessage): Promise<void> {
    return await transportManager.sendMessage(message);
  },

  onMessage(handler: (message: TransportMessage) => void): () => void {
    return transportManager.onMessage(handler);
  },

  onEvent(listener: (event: any) => void): () => void {
    return transportManager.onEvent(listener);
  },

  getActiveTransports(): string[] {
    return transportManager.getActiveTransports();
  },

  getBestTransport(): string | null {
    return transportManager.getBestTransport();
  },

  getStats(): TransportState {
    return transportManager.getStats();
  },
};
