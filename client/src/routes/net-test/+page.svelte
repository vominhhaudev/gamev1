<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { TransportHandle, TransportKind } from '$lib/transport';
  import { selectTransport } from '$lib/transport';

  type Sample = {
    seq: number;
    sentAt: number;
    receivedAt: number;
  };

  type Frame = {
    channel: string;
    sequence: number;
    timestamp_ms: number;
    kind: string;
    message: FrameMessage;
  };

  type FrameMessage = {
    type: string;
    nonce?: number;
    name?: string;
    data?: unknown;
  };

  const DEFAULT_ENDPOINT = 'ws://127.0.0.1:3000/ws';

  let endpoint = DEFAULT_ENDPOINT;
  let status: 'disconnected' | 'connecting' | 'connected' | 'error' = 'disconnected';
  let websocket: WebSocket | null = null;
  let transport: TransportHandle | null = null;
  let transportKind: TransportKind | null = null;
  let authToken: string | null = null;
  let authExpiresIn: number | null = null;
  let samples: Sample[] = [];
  let lastEventName: string | null = null;
  let lastEventData: unknown = null;
  let errorMessage = '';
  let nextSeq = 1;
  let tick: ReturnType<typeof setInterval> | undefined;

  async function handleMessage(event: MessageEvent) {
    const receivedAt = performance.now();
    const text = await readMessageAsText(event.data);
    if (!text) {
      return;
    }

    let frame: Frame | null = null;
    try {
      frame = JSON.parse(text) as Frame;
    } catch (err) {
      console.warn('invalid frame', err);
      return;
    }

    if (frame.kind === 'control' && frame.message?.type === 'pong') {
      const seq = frame.sequence ?? frame.message.nonce ?? 0;
      const sentAt = typeof frame.timestamp_ms === 'number' ? frame.timestamp_ms : receivedAt;
      samples = [{ seq, sentAt, receivedAt }, ...samples].slice(0, 40);
      return;
    }

    if (frame.kind === 'state' && frame.message?.type === 'event') {
      lastEventName = frame.message.name ?? frame.message.type ?? null;
      lastEventData = frame.message.data ?? null;
    }
  }

  function handleClose() {
    status = 'disconnected';
    cleanupInterval();
    transportKind = null;
    authToken = null;
    authExpiresIn = null;
    lastEventName = null;
    lastEventData = null;
  }

  function handleSocketError(event: Event) {
    console.error('ws error', event);
    status = 'error';
    errorMessage = 'WebSocket error, check gateway logs';
    cleanupInterval();
  }

  function reset() {
    samples = [];
    nextSeq = 1;
    errorMessage = '';
    lastEventName = null;
    lastEventData = null;
  }

  function detachListeners(ws: WebSocket | null) {
    if (!ws) return;
    ws.removeEventListener('message', handleMessage);
    ws.removeEventListener('close', handleClose);
    ws.removeEventListener('error', handleSocketError);
  }

  function attachListeners(ws: WebSocket) {
    ws.addEventListener('message', handleMessage);
    ws.addEventListener('close', handleClose);
    ws.addEventListener('error', handleSocketError);
  }

  async function connect() {
    reset();
    cleanupInterval();
    detachListeners(websocket);
    transport = null;
    transportKind = null;
    websocket = null;
    authToken = null;
    authExpiresIn = null;

    status = 'connecting';
    try {
      const handle = await selectTransport({ websocketEndpoint: endpoint });
      transport = handle;
      transportKind = handle.kind;
      authToken = handle.authToken ?? null;
      authExpiresIn = handle.authExpiresIn ?? null;
      websocket = handle.socket;
      attachListeners(websocket);
      status = 'connected';
      schedulePing();
    } catch (err) {
      console.error('connect failed', err);
      status = 'error';
      errorMessage = (err as Error).message ?? 'unknown error';
      detachListeners(websocket);
      transport = null;
      websocket = null;
      transportKind = null;
      authToken = null;
      authExpiresIn = null;
    }
  }

  function disconnect() {
    cleanupInterval();
    detachListeners(websocket);
    websocket?.close();
    websocket = null;
    transport = null;
    transportKind = null;
    authToken = null;
    authExpiresIn = null;
    status = 'disconnected';
  }

  function schedulePing() {
    cleanupInterval();
    tick = setInterval(() => {
      if (!websocket || websocket.readyState !== WebSocket.OPEN) {
        return;
      }
      const seq = nextSeq++;
      const timestampMs = Math.round(performance.now());
      const frame: Frame = {
        channel: 'control',
        sequence: seq,
        timestamp_ms: timestampMs,
        kind: 'control',
        message: {
          type: 'ping',
          nonce: seq,
        },
      };
      websocket.send(JSON.stringify(frame));
    }, 500);
  }

  function cleanupInterval() {
    if (tick) {
      clearInterval(tick);
      tick = undefined;
    }
  }

  $: averageRtt =
    samples.length === 0
      ? 0
      : samples.reduce((acc, sample) => acc + (sample.receivedAt - sample.sentAt), 0) /
        samples.length;

  onDestroy(() => {
    disconnect();
  });

  async function readMessageAsText(data: unknown): Promise<string | null> {
    if (typeof data === 'string') {
      return data;
    }
    if (data instanceof ArrayBuffer) {
      return new TextDecoder().decode(data);
    }
    if (data instanceof Blob) {
      try {
        return await data.text();
      } catch (err) {
        console.warn('blob read failed', err);
        return null;
      }
    }
    return null;
  }
</script>

<section class="container">
  <header>
    <h1>/net-test</h1>
    <p>Ðo round-trip networking theo roadmap Week 1/Week 2.</p>
  </header>

  <form class="endpoint" on:submit|preventDefault={connect}>
    <label>
      Gateway endpoint (uu tiên WS hi?n t?i)
      <input bind:value={endpoint} placeholder={DEFAULT_ENDPOINT} />
    </label>
    <div class="actions">
      {#if status === 'connected'}
        <button type="button" on:click={disconnect}>Disconnect</button>
      {:else}
        <button type="submit">Connect</button>
      {/if}
    </div>
  </form>

  <section class="telemetry">
    <div>
      <span class="label">Status</span>
      <span class={`badge badge--${status}`}>{status}</span>
    </div>
    <div>
      <span class="label">Transport</span>
      <span class="value">{transportKind ?? '-'}</span>
    </div>
    <div>
      <span class="label">Auth Token</span>
      <span class="value">{authToken ?? '-'}</span>
    </div>
    <div>
      <span class="label">Token TTL (s)</span>
      <span class="value">{authExpiresIn ?? '-'}</span>
    </div>
    <div>
      <span class="label">Average RTT (ms)</span>
      <span class="value">{averageRtt.toFixed(2)}</span>
    </div>
    <div>
      <span class="label">Samples</span>
      <span class="value">{samples.length}</span>
    </div>
    <div>
      <span class="label">Last Event</span>
      <span class="value">{lastEventName ?? '-'}</span>
    </div>
  </section>

  {#if errorMessage}
    <p class="error">{errorMessage}</p>
  {/if}

  <table>
    <thead>
      <tr>
        <th>#</th>
        <th>Sent (ms)</th>
        <th>RTT (ms)</th>
      </tr>
    </thead>
    <tbody>
      {#each samples as sample}
        <tr>
          <td>{sample.seq}</td>
          <td>{sample.sentAt.toFixed(3)}</td>
          <td>{(sample.receivedAt - sample.sentAt).toFixed(3)}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if lastEventData}
    <section class="event">
      <h2>Event Payload</h2>
      <pre>{JSON.stringify(lastEventData, null, 2)}</pre>
    </section>
  {/if}
</section>

<style>
  .container {
    max-width: 780px;
    margin: 2rem auto;
    padding: 1.5rem;
    border-radius: 12px;
    background: #0b0f1a;
    color: #f6f8ff;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.25);
    font-family: 'Segoe UI', system-ui, sans-serif;
  }
  header h1 {
    margin: 0;
    font-size: 2rem;
  }
  header p {
    margin-top: 0.25rem;
    color: #90a0d0;
  }
  .endpoint {
    display: flex;
    gap: 1rem;
    margin: 1.5rem 0;
    flex-wrap: wrap;
  }
  label {
    flex: 1 1 320px;
    display: flex;
    flex-direction: column;
    font-size: 0.9rem;
    color: #c3ccec;
    gap: 0.35rem;
  }
  input {
    padding: 0.6rem 0.8rem;
    border-radius: 8px;
    border: 1px solid #253157;
    background: #121a2b;
    color: inherit;
  }
  .actions {
    display: flex;
    align-items: flex-end;
  }
  button {
    padding: 0.65rem 1.1rem;
    border-radius: 8px;
    border: none;
    background: #446bff;
    color: #fff;
    cursor: pointer;
    font-weight: 600;
  }
  button:hover {
    background: #3359e0;
  }
  .telemetry {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .label {
    display: block;
    color: #7786b7;
    font-size: 0.8rem;
    margin-bottom: 0.4rem;
  }
  .value {
    font-size: 1.2rem;
    font-weight: 600;
    word-break: break-all;
  }
  .badge {
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    text-transform: capitalize;
  }
  .badge--connected {
    background: #1f8b4c;
  }
  .badge--connecting {
    background: #c47a10;
  }
  .badge--disconnected {
    background: #39435a;
  }
  .badge--error {
    background: #b9383a;
  }
  .error {
    color: #ff8d8f;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 1rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.5rem 0.25rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  tbody tr:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .event {
    margin-top: 1.5rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1rem;
  }
  .event h2 {
    margin-top: 0;
    font-size: 1.1rem;
    color: #c3ccec;
  }
  .event pre {
    margin: 0;
    font-size: 0.85rem;
    background: rgba(0, 0, 0, 0.3);
    padding: 0.75rem;
    border-radius: 6px;
    overflow-x: auto;
  }
</style>
