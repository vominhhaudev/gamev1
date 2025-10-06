export type TransportKind = 'webtransport' | 'webrtc' | 'websocket';

// Quantization configuration for client-side decompression
export interface QuantizationConfig {
  position_factor: number;
  rotation_factor: number;
  scale_factor: number;
  velocity_factor: number;
}

export const DEFAULT_QUANTIZATION_CONFIG: QuantizationConfig = {
  position_factor: 0.01,      // 0.01 units per step = ±327.68 units range
  rotation_factor: 1.0,       // 1 degree per step = ±128 degrees range
  scale_factor: 0.01,         // 0.01 scale per step = ±1.28 scale range
  velocity_factor: 0.1,       // 0.1 units/sec per step = ±3276.8 units/sec range
};

export interface TransportHandle {
  kind: TransportKind;
  socket: WebSocket;
  authToken?: string;
  authExpiresIn?: number;
  externalAuthToken?: string; // Token từ bên ngoài (auth store)
}

const DEFAULT_ORDER: TransportKind[] = ['webtransport', 'webrtc', 'websocket'];

interface NegotiationResponse {
  order?: string[];
  transports?: Array<{
    kind: string;
    available: boolean;
    endpoint?: string | null;
  }>;
  auth?: {
    access_token: string;
    expires_in: number;
  };
}

interface NegotiationPlan {
  order: TransportKind[];
  transports: Map<TransportKind, { available: boolean; endpoint?: string }>;
  auth?: {
    access_token: string;
    expires_in: number;
  };
}

export async function selectTransport(options?: {
  order?: TransportKind[];
  websocketEndpoint?: string;
  authToken?: string;
}): Promise<TransportHandle> {
  const fallbackWsEndpoint = options?.websocketEndpoint ?? 'ws://127.0.0.1:3000/ws';
  const wsUrl = safeParseUrl(fallbackWsEndpoint);
  const negotiationPlan = wsUrl ? await fetchNegotiationPlan(wsUrl, options?.order) : null;

  const order = negotiationPlan?.order ?? options?.order ?? DEFAULT_ORDER;

  for (const kind of order) {
    const info = negotiationPlan?.transports.get(kind);
    if (info && info.available === false) {
      continue;
    }

    try {
      switch (kind) {
        case 'webtransport':
          throw new Error('webtransport adapter not implemented yet');
        case 'webrtc':
          throw new Error('webrtc adapter not implemented yet');
        case 'websocket': {
          const endpoint = resolveWebSocketEndpoint(
            info?.endpoint,
            wsUrl,
            fallbackWsEndpoint
          );
          const handle = await connectWebSocket(endpoint, options?.authToken);
          if (negotiationPlan?.auth) {
            return {
              ...handle,
              authToken: negotiationPlan.auth.access_token,
              authExpiresIn: negotiationPlan.auth.expires_in,
            };
          }
          return handle;
        }
      }
    } catch (err) {
      console.warn(`[transport] ${kind} failed:`, err);
      continue;
    }
  }

  throw new Error('no transport succeeded');
}

async function fetchNegotiationPlan(
  wsUrl: URL,
  overrideOrder?: TransportKind[]
): Promise<NegotiationPlan | null> {
  const httpProtocol = wsUrl.protocol === 'wss:' ? 'https:' : 'http:';
  const httpBase = `${httpProtocol}//${wsUrl.host}`;
  const negotiateUrl = new URL(NEGOTIATE_PATH, httpBase).toString();

  try {
    const res = await fetch(negotiateUrl, { method: 'GET' });
    if (!res.ok) {
      return null;
    }
    const body = (await res.json()) as NegotiationResponse;
    const order = normalizeOrder(body.order, overrideOrder) ?? DEFAULT_ORDER;
    const transports = new Map<TransportKind, { available: boolean; endpoint?: string }>();
    if (Array.isArray(body.transports)) {
      for (const item of body.transports) {
        const kind = normalizeKind(item.kind);
        if (!kind) {
          continue;
        }
        transports.set(kind, {
          available: item.available,
          endpoint: item.endpoint ?? undefined,
        });
      }
    }
    return {
      order,
      transports,
      auth: body.auth,
    };
  } catch (err) {
    console.warn('fetch negotiate failed', err);
    return null;
  }
}

function normalizeOrder(
  incoming: string[] | undefined,
  overrideOrder?: TransportKind[]
): TransportKind[] | null {
  if (overrideOrder) {
    return overrideOrder;
  }
  if (!incoming) {
    return null;
  }
  const kinds = incoming
    .map(normalizeKind)
    .filter((kind): kind is TransportKind => kind !== null);
  return kinds.length > 0 ? kinds : null;
}

function normalizeKind(value: string | undefined): TransportKind | null {
  switch (value) {
    case 'websocket':
      return 'websocket';
    case 'webtransport':
      return 'webtransport';
    case 'webrtc':
      return 'webrtc';
    default:
      return null;
  }
}

function resolveWebSocketEndpoint(
  negotiatedEndpoint: string | undefined,
  wsUrl: URL | null,
  fallbackWsEndpoint: string
): string {
  if (!negotiatedEndpoint) {
    return fallbackWsEndpoint;
  }
  if (negotiatedEndpoint.startsWith('ws://') || negotiatedEndpoint.startsWith('wss://')) {
    return negotiatedEndpoint;
  }
  if (!wsUrl) {
    return fallbackWsEndpoint;
  }
  if (negotiatedEndpoint.startsWith('/')) {
    const protocol = wsUrl.protocol === 'wss:' ? 'wss:' : 'ws:';
    return `${protocol}//${wsUrl.host}${negotiatedEndpoint}`;
  }
  return negotiatedEndpoint;
}

function safeParseUrl(value: string | undefined | null): URL | null {
  if (!value) {
    return null;
  }
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

async function connectWebSocket(endpoint: string, externalAuthToken?: string): Promise<TransportHandle> {
  const socket = new WebSocket(endpoint);
  await new Promise<void>((resolve, reject) => {
    const onOpen = () => {
      cleanup();
      resolve();
    };
    const onError = (event: Event) => {
      cleanup();
      reject(event);
    };
    function cleanup() {
      socket.removeEventListener('open', onOpen);
      socket.removeEventListener('error', onError);
    }
    socket.addEventListener('open', onOpen);
    socket.addEventListener('error', onError);
  });

  return {
    kind: 'websocket',
    socket,
    externalAuthToken,
  };
}

const NEGOTIATE_PATH = '/negotiate';

// Quantization utilities for client-side decompression
export function dequantizePosition(position: [number, number, number], config: QuantizationConfig): [number, number, number] {
  return [
    position[0] * config.position_factor,
    position[1] * config.position_factor,
    position[2] * config.position_factor,
  ];
}

export function dequantizeRotation(rotation: number, config: QuantizationConfig): number {
  return rotation * config.rotation_factor;
}

export function dequantizeScale(scale: number, config: QuantizationConfig): number {
  return scale * config.scale_factor;
}

export function dequantizeVelocity(velocity: [number, number, number], config: QuantizationConfig): [number, number, number] {
  return [
    velocity[0] * config.velocity_factor,
    velocity[1] * config.velocity_factor,
    velocity[2] * config.velocity_factor,
  ];
}

export function dequantizeSmallInt(value: number, _config: QuantizationConfig): number {
  return value;
}

// Handle quantized snapshot message from server
export function handleQuantizedSnapshot(eventData: any): any {
  const config = DEFAULT_QUANTIZATION_CONFIG;

  if (eventData.encoding === 'bincode_quantized' && eventData.tick !== undefined) {
    // In a full implementation, this would:
    // 1. Receive the binary quantized data
    // 2. Decompress it using bincode
    // 3. Dequantize the values
    // 4. Return the reconstructed entities

    console.log(`📦 Received quantized snapshot: tick=${eventData.tick}, size=${eventData.size} bytes`);

    // For now, return a mock structure
    return {
      tick: eventData.tick,
      entities: [],
      quantized: true,
      compression_ratio: eventData.size > 0 ? (1 - eventData.size / 1000) * 100 : 0 // Mock calculation
    };
  }

  return null;
}

// Handle quantized delta message from server
export function handleQuantizedDelta(eventData: any): any {
  const config = DEFAULT_QUANTIZATION_CONFIG;

  if (eventData.encoding === 'bincode_quantized' && eventData.tick !== undefined) {
    // In a full implementation, this would:
    // 1. Receive the binary quantized data
    // 2. Decompress it using bincode
    // 3. Dequantize the values
    // 4. Return the reconstructed changes

    console.log(`🔄 Received quantized delta: tick=${eventData.tick}, size=${eventData.size} bytes`);

    // For now, return a mock structure
    return {
      tick: eventData.tick,
      changes: [],
      quantized: true,
      compression_ratio: eventData.size > 0 ? (1 - eventData.size / 500) * 100 : 0 // Mock calculation
    };
  }

  return null;
}

// Calculate bandwidth savings from quantization
export function calculateBandwidthSavings(originalBytes: number, quantizedBytes: number): number {
  if (originalBytes === 0) return 0;
  return ((originalBytes - quantizedBytes) / originalBytes) * 100;
}
