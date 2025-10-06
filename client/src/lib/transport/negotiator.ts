export interface TransportCapabilities {
  quic: boolean;
  webrtc: boolean;
  websocket: boolean;
  http: boolean;
}

export type TransportType = 'quic' | 'webrtc' | 'websocket' | 'http';

export interface TransportTestResult {
  transport: TransportType;
  latency: number;
  available: boolean;
  error?: string;
}

export interface NetworkConfig {
  gatewayUrl: string;
  preferredTransport?: TransportType;
  timeout: number;
  retries: number;
}

export class TransportNegotiator {
  private config: NetworkConfig;

  constructor(config: NetworkConfig) {
    this.config = config;
  }

  /**
   * Negotiate best transport với server và test từng loại
   */
  async negotiateBestTransport(): Promise<TransportType> {
    console.log('🔍 Starting transport negotiation...');

    try {
      // 1. Get server capabilities
      const capabilities = await this.getServerCapabilities();
      console.log('📡 Server capabilities:', capabilities);

      // 2. Test available transports
      const results = await this.testAllTransports(capabilities);
      console.log('🧪 Transport test results:', results);

      // 3. Select best transport
      const bestTransport = this.selectBestTransport(results);
      console.log('🏆 Selected transport:', bestTransport);

      return bestTransport;
    } catch (error) {
      console.error('❌ Transport negotiation failed:', error);
      return 'websocket'; // Fallback to WebSocket
    }
  }

  /**
   * Get transport capabilities từ server
   */
  private async getServerCapabilities(): Promise<TransportCapabilities> {
    const response = await fetch(`${this.config.gatewayUrl}/api/transport/negotiate`);

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}`);
    }

    return await response.json();
  }

  /**
   * Test tất cả available transports
   */
  private async testAllTransports(capabilities: TransportCapabilities): Promise<TransportTestResult[]> {
    const transports: TransportType[] = [];

    if (capabilities.quic) transports.push('quic');
    if (capabilities.webrtc) transports.push('webrtc');
    if (capabilities.websocket) transports.push('websocket');

    const results: TransportTestResult[] = [];

    for (const transport of transports) {
      const result = await this.testTransport(transport);
      results.push(result);

      // Early return nếu tìm được transport tốt
      if (result.available && result.latency < 50) {
        break;
      }
    }

    return results;
  }

  /**
   * Test một transport cụ thể
   */
  private async testTransport(transport: TransportType): Promise<TransportTestResult> {
    const startTime = Date.now();

    try {
      switch (transport) {
        case 'websocket':
          return await this.testWebSocket();

        case 'webrtc':
          return await this.testWebRTC();

        case 'quic':
          return await this.testQUIC();

        default:
          return {
            transport,
            latency: Infinity,
            available: false,
            error: 'Unsupported transport'
          };
      }
    } catch (error) {
      return {
        transport,
        latency: Date.now() - startTime,
        available: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }

  /**
   * Test WebSocket connectivity
   */
  private async testWebSocket(): Promise<TransportTestResult> {
    return new Promise((resolve) => {
      const startTime = Date.now();
      const wsUrl = this.config.gatewayUrl.replace('http', 'ws') + '/api/transport/enhanced-ws';

      try {
        const ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          ws.close();
          resolve({
            transport: 'websocket',
            latency: Date.now() - startTime,
            available: true
          });
        };

        ws.onerror = () => {
          resolve({
            transport: 'websocket',
            latency: Date.now() - startTime,
            available: false,
            error: 'Connection failed'
          });
        };

        // Timeout
        setTimeout(() => {
          ws.close();
          resolve({
            transport: 'websocket',
            latency: Date.now() - startTime,
            available: false,
            error: 'Timeout'
          });
        }, this.config.timeout);

      } catch (error) {
        resolve({
          transport: 'websocket',
          latency: Date.now() - startTime,
          available: false,
          error: error instanceof Error ? error.message : 'Unknown error'
        });
      }
    });
  }

  /**
   * Test WebRTC connectivity với signaling server
   */
  private async testWebRTC(): Promise<TransportTestResult> {
    const startTime = Date.now();

    try {
      // Check browser support
      if (!('RTCPeerConnection' in window)) {
        return {
          transport: 'webrtc',
          latency: Date.now() - startTime,
          available: false,
          error: 'WebRTC not supported'
        };
      }

      // Test WebRTC signaling với server
      const testRoomId = `test-room-${Date.now()}`;
      const testPeerId = `test-peer-${Math.random().toString(36).substr(2, 9)}`;

      // Tạo offer SDP
      const pc = new RTCPeerConnection({
        iceServers: [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' }
        ]
      });

      // Tạo DataChannel để test
      const dc = pc.createDataChannel('test', {
        ordered: true,
        maxRetransmits: 0
      });

      return new Promise(async (resolve) => {
        // Handle ICE candidates
        pc.onicecandidate = async (event) => {
          if (event.candidate) {
            try {
              // Send ICE candidate to signaling server
              await fetch(`${this.config.gatewayUrl}/rtc/ice`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                  room_id: testRoomId,
                  peer_id: testPeerId,
                  candidate: {
                    candidate: event.candidate.candidate,
                    sdpMid: event.candidate.sdpMid,
                    sdpMLineIndex: event.candidate.sdpMLineIndex,
                    usernameFragment: event.candidate.usernameFragment
                  }
                })
              });
            } catch (error) {
              console.warn('Failed to send ICE candidate:', error);
            }
          }
        };

        // Handle DataChannel open
        dc.onopen = () => {
          pc.close();
          resolve({
            transport: 'webrtc',
            latency: Date.now() - startTime,
            available: true
          });
        };

        // Handle DataChannel error
        dc.onerror = (error) => {
          pc.close();
          resolve({
            transport: 'webrtc',
            latency: Date.now() - startTime,
            available: false,
            error: 'DataChannel error'
          });
        };

        try {
          // Create offer
          const offer = await pc.createOffer();

          // Set local description
          await pc.setLocalDescription(offer);

          // Send offer to signaling server
          const response = await fetch(`${this.config.gatewayUrl}/rtc/offer`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              room_id: testRoomId,
              peer_id: testPeerId,
              offer: {
                type: offer.type,
                sdp: offer.sdp
              }
            })
          });

          if (response.ok) {
            const result = await response.json();

            if (result.success && result.answer) {
              // Set remote description
              await pc.setRemoteDescription({
                type: 'answer',
                sdp: result.answer.sdp
              });

              // Add ICE candidates
              if (result.ice_candidates) {
                for (const candidate of result.ice_candidates) {
                  await pc.addIceCandidate({
                    candidate: candidate.candidate,
                    sdpMid: candidate.sdpMid,
                    sdpMLineIndex: candidate.sdpMLineIndex,
                    usernameFragment: candidate.usernameFragment
                  });
                }
              }
            }
          }

          // Timeout
          setTimeout(() => {
            pc.close();
            resolve({
              transport: 'webrtc',
              latency: Date.now() - startTime,
              available: false,
              error: 'Timeout'
            });
          }, this.config.timeout);

        } catch (error) {
          pc.close();
          resolve({
            transport: 'webrtc',
            latency: Date.now() - startTime,
            available: false,
            error: error instanceof Error ? error.message : 'Unknown error'
          });
        }
      });

    } catch (error) {
      return {
        transport: 'webrtc',
        latency: Date.now() - startTime,
        available: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }

  /**
   * Test QUIC connectivity (placeholder)
   */
  private async testQUIC(): Promise<TransportTestResult> {
    const startTime = Date.now();

    // QUIC testing would require WebTransport API
    // For now, assume QUIC is available nếu browser support WebTransport
    if ('WebTransport' in window) {
      return {
        transport: 'quic',
        latency: Date.now() - startTime,
        available: true
      };
    }

    return {
      transport: 'quic',
      latency: Date.now() - startTime,
      available: false,
      error: 'WebTransport not supported'
    };
  }

  /**
   * Select best transport dựa trên results
   */
  private selectBestTransport(results: TransportTestResult[]): TransportType {
    // Filter available transports và sort by priority then latency
    const available = results
      .filter(r => r.available)
      .sort((a, b) => {
        // Define transport priority (WebRTC > QUIC > WebSocket)
        const priority = { webrtc: 0, quic: 1, websocket: 2 };
        const aPriority = priority[a.transport as keyof typeof priority] ?? 3;
        const bPriority = priority[b.transport as keyof typeof priority] ?? 3;

        if (aPriority !== bPriority) {
          return aPriority - bPriority;
        }

        // If same priority, sort by latency
        return a.latency - b.latency;
      });

    if (available.length === 0) {
      throw new Error('No transports available');
    }

    // Prefer user's preferred transport nếu available
    if (this.config.preferredTransport) {
      const preferred = available.find(r => r.transport === this.config.preferredTransport);
      if (preferred) {
        return preferred.transport;
      }
    }

    // Otherwise return highest priority (fastest) available
    return available[0].transport;
  }

  /**
   * Get current transport capabilities từ browser
   */
  static getBrowserCapabilities(): TransportCapabilities {
    return {
      websocket: true, // WebSocket luôn available
      webrtc: 'RTCPeerConnection' in window,
      quic: 'WebTransport' in window
    };
  }
}

/**
 * Default configuration cho transport negotiation
 */
export const defaultNetworkConfig: NetworkConfig = {
  gatewayUrl: 'http://localhost:8080',
  timeout: 5000,
  retries: 3
};

/**
 * Utility function để negotiate transport với config mặc định
 */
export async function negotiateTransport(config?: Partial<NetworkConfig>): Promise<TransportType> {
  const fullConfig = { ...defaultNetworkConfig, ...config };
  const negotiator = new TransportNegotiator(fullConfig);
  return await negotiator.negotiateBestTransport();
}
