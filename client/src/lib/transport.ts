export type TransportKind = 'webtransport' | 'webrtc' | 'websocket';

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
