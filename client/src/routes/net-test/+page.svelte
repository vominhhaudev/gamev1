<section class="container">
  <header>
    <h1>/net-test</h1>
    <p>Network testing page with WebRTC signaling</p>
  </header>

  <div class="content">
    <div class="status-grid">
      <div class="status-card">
        <h3>WebSocket Status</h3>
        <p class="status {isConnected ? 'connected' : 'disconnected'}">{connectionStatus}</p>
        <div class="controls">
          <button on:click={connectWebSocket} disabled={isConnected}>Connect WS</button>
          <button on:click={disconnect} disabled={!isConnected}>Disconnect</button>
        </div>
      </div>

      <div class="status-card">
        <h3>WebRTC Connection</h3>
        <p class="status {webrtcState?.isConnected ? 'connected' : 'disconnected'}">
          {webrtcState?.transportType === 'webrtc' ? 'WebRTC' : webrtcState?.transportType === 'websocket' ? 'WebSocket (Fallback)' : 'Not initialized'}
          {#if webrtcState?.isFallback}
            <span class="fallback-indicator">FALLBACK</span>
          {/if}
        </p>
        <div class="connection-info">
          <div>State: {webrtcState?.connectionState || 'Not initialized'}</div>
          <div>Transport: {webrtcState?.transportType || 'none'}</div>
          {#if webrtcState?.isFallback}
            <div class="fallback-reason">Fallback: {webrtcState.fallbackReason}</div>
          {/if}
        </div>
        <div class="controls">
          <button on:click={createWebRTCOffer} disabled={!!webrtcState?.sessionId}>Init WebRTC</button>
          <button on:click={createWebRTCOfferReal}>Create Offer</button>
          <button on:click={sendICECandidate} disabled={!webrtcState?.isConnected}>Send ICE</button>
          <button on:click={listWebRTCSessions}>List Sessions</button>
          <button on:click={() => webrtcActions.close()}>Close</button>
        </div>
        {#if webrtcState?.controlChannel}
          <div class="channel-status">
            <span class="channel-indicator control"></span>
            Control Channel: {webrtcState.controlChannel.readyState}
          </div>
        {/if}
        {#if webrtcState?.stateChannel}
          <div class="channel-status">
            <span class="channel-indicator state"></span>
            State Channel: {webrtcState.stateChannel.readyState}
          </div>
        {/if}
      </div>

      <div class="status-card">
        <h3>Transport Manager</h3>
        <p class="status {transportState?.isConnected ? 'connected' : 'disconnected'}">
          {activeTransportsCount} active transports
        </p>
        <div class="controls">
          <button on:click={addWebRTCTransport}>Add WebRTC Transport</button>
          <button on:click={addWebSocketTransport}>Add WebSocket Transport</button>
          <button on:click={sendTransportMessage}>Send Game Input</button>
          <button on:click={sendMovementInput}>Send Movement (1,0,0)</button>
          <button on:click={sendJumpInput}>Send Jump (0,1,0)</button>
          <button on:click={getTransportStats}>Get Transport Stats</button>
        </div>
      </div>
    </div>

    <div class="websocket-section">
      <h3>WebSocket Chat</h3>
      <div class="chat-container">
        <div class="messages">
          {#each messages as message (message.timestamp)}
            <div class="message {message.sender === 'You' ? 'own' : ''}">
              <span class="sender">{message.sender}:</span>
              <span class="content">{message.message}</span>
            </div>
          {/each}
        </div>
        <div class="input-container">
          <input
            type="text"
            bind:value={inputMessage}
            placeholder="Type a message..."
            on:keydown={(e) => e.key === 'Enter' && sendMessage()}
            disabled={!isConnected}
          />
          <button on:click={sendMessage} disabled={!isConnected}>Send</button>
        </div>
      </div>
    </div>

    <div class="info">
      <h3>Current Status:</h3>
      <ul>
        <li>Client: Running on http://localhost:5173/</li>
        <li>WebSocket: {connectionStatus}</li>
        <li>WebRTC Session: {webrtcState?.sessionId || 'None'}</li>
        <li>WebRTC Connection: {webrtcState?.connectionState || 'Not initialized'}</li>
        <li>WebRTC Transport: {webrtcState?.transportType || 'None'}
          {#if webrtcState?.isFallback}
            <span class="fallback-badge">FALLBACK</span>
          {/if}
        </li>
        <li>ICE Connection: {webrtcState?.iceConnectionState || 'Not initialized'}</li>
        <li>Signaling State: {webrtcState?.signalingState || 'Not initialized'}</li>
        <li>Active Transports: {activeTransportsCount}</li>
        <li>Best Transport: {transportState?.bestTransport || 'None'}</li>
        <li>Framework: SvelteKit</li>
      </ul>

      {#if webrtcState}
        <div class="webrtc-stats">
          <h4>WebRTC Statistics:</h4>
          <ul>
            <li>Packets Sent: {webrtcState.stats.packetsSent}</li>
            <li>Packets Received: {webrtcState.stats.packetsReceived}</li>
            <li>Bytes Sent: {webrtcState.stats.bytesSent}</li>
            <li>Bytes Received: {webrtcState.stats.bytesReceived}</li>
            <li>Round Trip Time: {webrtcState.stats.roundTripTime}ms</li>
          </ul>
        </div>
      {/if}

      {#if transportState}
        <div class="transport-stats">
          <h4>Transport Manager Statistics:</h4>
          <ul>
            <li>Total Messages Sent: {transportState.totalMessagesSent}</li>
            <li>Total Messages Received: {transportState.totalMessagesReceived}</li>
            <li>Total Bytes Sent: {transportState.totalBytesSent}</li>
            <li>Total Bytes Received: {transportState.totalBytesReceived}</li>
            <li>Average Latency: {transportState.averageLatencyMs}ms</li>
            <li>Error Count: {transportState.errorCount}</li>
            <li>Reconnect Count: {transportState.reconnectCount}</li>
          </ul>
        </div>
      {/if}
    </div>

    <div class="datachannel-test">
      <h3>DataChannel Testing</h3>
      <div class="controls">
        <button on:click={() => webrtcActions.sendControlMessage({action: 'test', data: 'Hello Control'})} disabled={!webrtcState?.controlChannel}>
          Send Control Message
        </button>
        <button on:click={() => webrtcActions.sendStateMessage({x: Math.random() * 100, y: Math.random() * 100})} disabled={!webrtcState?.stateChannel}>
          Send State Update
        </button>
        <button on:click={() => webrtcActions.getStats().then(stats => console.log('WebRTC Stats:', stats))}>
          Get Connection Stats
        </button>
        <button on:click={testAllConnections} class="test-btn">
          Test All Connections
        </button>
      </div>
    </div>

    <div class="connections-overview">
      <h3>All Project Connections</h3>
      <div class="connections-grid">
        <div class="connection-card">
          <h4>Client Connections</h4>
          <div class="connection-list">
            <div class="connection-item">
              <span class="connection-type">WebSocket</span>
              <span class="connection-status {isConnected ? 'connected' : 'disconnected'}">
                {isConnected ? 'Connected' : 'Disconnected'}
              </span>
              <span class="connection-endpoint">ws://localhost:8080/ws</span>
            </div>
            <div class="connection-item">
              <span class="connection-type">WebRTC</span>
              <span class="connection-status {webrtcState?.isConnected ? 'connected' : 'disconnected'}">
                {webrtcState?.isConnected ? 'Connected' : 'Disconnected'}
              </span>
              <span class="connection-endpoint">
                {webrtcState?.transportType === 'webrtc' ? 'P2P Connection' : 'WebSocket Fallback'}
              </span>
            </div>
            <div class="connection-item">
              <span class="connection-type">Transport Manager</span>
              <span class="connection-status {transportState?.isConnected ? 'connected' : 'disconnected'}">
                {transportState?.isConnected ? 'Connected' : 'Disconnected'}
              </span>
              <span class="connection-endpoint">
                {transportState?.bestTransport ? transportState.bestTransport : 'No active transport'}
              </span>
            </div>
          </div>
        </div>

        <div class="connection-card">
          <h4>Server Connections</h4>
          <div class="connection-list">
            <div class="connection-item">
              <span class="connection-type">Gateway HTTP</span>
              <span class="connection-status connected">Running</span>
              <span class="connection-endpoint">http://localhost:8080</span>
            </div>
            <div class="connection-item">
              <span class="connection-type">Worker gRPC</span>
              <span class="connection-status connected">Running</span>
              <span class="connection-endpoint">grpc://localhost:50051</span>
            </div>
            <div class="connection-card">
              <h4>Database Connections</h4>
              <div class="connection-list">
                <div class="connection-item">
                  <span class="connection-type">PocketBase</span>
                  <span class="connection-status connected">Running</span>
                  <span class="connection-endpoint">http://localhost:8090</span>
                </div>
                <div class="connection-item">
                  <span class="connection-type">PocketBase Data</span>
                  <span class="connection-status connected">Connected</span>
                  <span class="connection-endpoint">SQLite Database</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="connection-card">
          <h4>Network Architecture</h4>
          <div class="architecture-diagram">
            <div class="flow-diagram">
              <div class="node client">
                <span>Client (Port 5173)</span>
                <div class="connections">
                  <div class="conn-line">WebSocket → Gateway</div>
                  <div class="conn-line">WebRTC → P2P</div>
                  <div class="conn-line">HTTP → APIs</div>
                </div>
              </div>
              <div class="arrow">↓</div>
              <div class="node gateway">
                <span>Gateway (Port 8080)</span>
                <div class="connections">
                  <div class="conn-line">gRPC → Worker</div>
                  <div class="conn-line">HTTP → PocketBase</div>
                  <div class="conn-line">WebSocket ← Client</div>
                </div>
              </div>
              <div class="arrow">↓</div>
              <div class="node worker">
                <span>Worker (Port 50051)</span>
                <div class="connections">
                  <div class="conn-line">gRPC ← Gateway</div>
                  <div class="conn-line">Game Logic</div>
                </div>
              </div>
              <div class="arrow">↓</div>
              <div class="node database">
                <span>PocketBase (Port 8090)</span>
                <div class="connections">
                  <div class="conn-line">SQLite</div>
                  <div class="conn-line">User Data</div>
                  <div class="conn-line">Room Data</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="connection-card">
          <h4>Connection Statistics</h4>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="stat-label">Active Transports</span>
              <span class="stat-value">{activeTransportsCount}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">WebRTC Sessions</span>
              <span class="stat-value">{webrtcState?.sessionId ? 1 : 0}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Messages Sent</span>
              <span class="stat-value">{transportState?.totalMessagesSent || 0}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Messages Received</span>
              <span class="stat-value">{transportState?.totalMessagesReceived || 0}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Bytes Sent</span>
              <span class="stat-value">{transportState?.totalBytesSent || 0}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Bytes Received</span>
              <span class="stat-value">{transportState?.totalBytesReceived || 0}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Errors</span>
              <span class="stat-value">{transportState?.errorCount || 0}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Reconnections</span>
              <span class="stat-value">{transportState?.reconnectCount || 0}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="actions">
      <a href="/">← Back to Home</a>
    </div>
  </div>
</section>

<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { webrtcStore, webrtcActions } from '$lib/stores/webrtc';
  import { transportStore, transportActions } from '$lib/stores/transport';

  // SvelteKit route props

  // WebSocket state
  let ws = null;
  let messages = [];
  let inputMessage = '';
  let isConnected = false;
  let connectionStatus = 'Disconnected';

  // WebRTC state
  let webrtcSessionId = null;
  let signalingStatus = 'No session';
  let webrtcState = null;

  // Transport state
  let transportState = null;
  let activeTransportsCount = 0;

  // Subscribe to WebRTC store
  webrtcStore.subscribe(state => {
    webrtcState = state;
    if (state.sessionId) {
      webrtcSessionId = state.sessionId;
      signalingStatus = `Session: ${state.sessionId} (${state.connectionState})`;
    } else {
      signalingStatus = 'No session';
    }
  });

  // Subscribe to Transport store
  transportStore.subscribe(state => {
    transportState = state;
    activeTransportsCount = state.activeTransports.size;
  });

  onMount(async () => {
    if (browser) {
      connectWebSocket();
    }
  });

  function connectWebSocket() {
    try {
      console.log('Attempting to connect to WebSocket at ws://localhost:8080/ws');
      ws = new WebSocket('ws://localhost:8080/ws');

      ws.onopen = () => {
        console.log('WebSocket opened successfully');
        isConnected = true;
        connectionStatus = 'Connected';
        addMessage('System', 'WebSocket connected successfully');
      };

      ws.onmessage = (event) => {
        console.log('Received WebSocket message:', event.data);
        try {
          // Try to parse as JSON first
          const data = JSON.parse(event.data);
          addMessage('Server', `JSON: ${JSON.stringify(data)}`);
        } catch {
          // If not JSON, treat as text
          addMessage('Server', event.data);
        }
      };

      ws.onclose = (event) => {
        console.log('WebSocket closed:', event.code, event.reason);
        isConnected = false;
        connectionStatus = 'Disconnected';
        addMessage('System', `WebSocket disconnected (Code: ${event.code}, Reason: ${event.reason})`);
      };

      ws.onerror = (error) => {
        console.error('WebSocket error event:', error);
        addMessage('Error', `WebSocket error: ${error.type || 'Unknown error'}`);
      };

      // Add timeout to detect connection issues
      setTimeout(() => {
        if (!isConnected && ws.readyState !== WebSocket.OPEN) {
          console.log('WebSocket connection timeout or failed');
          addMessage('Error', 'WebSocket connection timeout');
          if (ws) ws.close();
        }
      }, 5000);

    } catch (error) {
      console.error('Failed to create WebSocket:', error);
      addMessage('Error', `Failed to create WebSocket: ${error.message}`);
    }
  }

  async function createWebRTCOffer() {
    try {
      signalingStatus = 'Initializing WebRTC with fallback...';

      // Initialize WebRTC service with fallback support
      await webrtcActions.initialize({
        iceServers: [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' }
        ],
        sessionId: 'temp_' + Date.now()
      });

      signalingStatus = 'WebRTC initialized';
      addMessage('WebRTC', 'WebRTC initialized successfully (will fallback to WebSocket if needed)');
    } catch (error) {
      signalingStatus = 'Error initializing WebRTC';
      addMessage('WebRTC', `Error: ${error.message}`);
    }
  }

  async function createWebRTCOfferReal() {
    try {
      signalingStatus = 'Creating WebRTC offer...';

      // Use WebRTC service to create actual offer
      await webrtcActions.createOffer();
      signalingStatus = 'WebRTC offer created and sent';
      addMessage('WebRTC', 'WebRTC offer created and sent via signaling');
    } catch (error) {
      signalingStatus = 'Error creating WebRTC offer';
      addMessage('WebRTC', `Error: ${error.message}`);
    }
  }

  async function sendWebRTCAnswer() {
    if (!webrtcState?.isConnected) {
      addMessage('WebRTC', 'WebRTC not connected');
      return;
    }

    try {
      signalingStatus = 'Sending answer...';

      // Send answer via signaling API
      const response = await fetch('/rtc/answer', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          sdp: 'test_sdp_answer',
          session_id: webrtcSessionId
        })
      });

      if (response.ok) {
        signalingStatus = 'Answer sent successfully';
        addMessage('WebRTC', 'Answer sent successfully');
      } else {
        signalingStatus = 'Failed to send answer';
        addMessage('WebRTC', 'Failed to send answer');
      }
    } catch (error) {
      signalingStatus = 'Error sending answer';
      addMessage('WebRTC', `Error: ${error.message}`);
    }
  }

  async function sendICECandidate() {
    if (!webrtcState?.isConnected) {
      addMessage('WebRTC', 'WebRTC not connected');
      return;
    }

    try {
      signalingStatus = 'Sending test ICE candidate...';

      // Send test ICE candidate via signaling
      const response = await fetch('/rtc/ice', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          candidate: 'test_ice_candidate',
          sdp_m_line_index: 0,
          sdp_mid: '0',
          session_id: webrtcSessionId
        })
      });

      if (response.ok) {
        signalingStatus = 'ICE candidate sent';
        addMessage('WebRTC', 'ICE candidate sent');
      } else {
        signalingStatus = 'Failed to send ICE candidate';
        addMessage('WebRTC', 'Failed to send ICE candidate');
      }
    } catch (error) {
      signalingStatus = 'Error sending ICE candidate';
      addMessage('WebRTC', `Error: ${error.message}`);
    }
  }

  async function listWebRTCSessions() {
    try {
      const response = await fetch('/rtc/sessions');
      if (response.ok) {
        const data = await response.json();
        addMessage('WebRTC', `Found ${data.total} sessions`);
        if (data.sessions.length > 0) {
          data.sessions.forEach(session => {
            addMessage('WebRTC', `Session: ${session.session_id} (${session.status})`);
          });
        }
      }
    } catch (error) {
      addMessage('WebRTC', `Error listing sessions: ${error.message}`);
    }
  }

  function sendMessage() {
    if (ws && ws.readyState === WebSocket.OPEN && inputMessage.trim()) {
      ws.send(inputMessage);
      addMessage('You', inputMessage);
      inputMessage = '';
    }
  }

  function addMessage(sender, message) {
    messages = [...messages, { sender, message, timestamp: new Date() }];
  }

  function disconnect() {
    if (ws) {
      ws.close();
    }
  }

  async function addWebRTCTransport() {
    try {
      addMessage('Transport', 'Adding WebRTC transport with fallback...');
      const transportId = await transportActions.addTransport({
        type: 'webrtc',
        iceServers: [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' }
        ],
        priority: 10
      });
      addMessage('Transport', `WebRTC transport added: ${transportId}`);
      addMessage('Transport', 'WebRTC will try to connect first, fallback to WebSocket if needed');
    } catch (error) {
      addMessage('Transport', `Error adding WebRTC transport: ${error.message}`);
    }
  }

  async function addWebSocketTransport() {
    try {
      addMessage('Transport', 'Adding WebSocket transport...');
      const transportId = await transportActions.addTransport({
        type: 'websocket',
        endpoint: 'ws://localhost:8080/ws',
        priority: 20
      });
      addMessage('Transport', `WebSocket transport added: ${transportId}`);
    } catch (error) {
      addMessage('Transport', `Error adding WebSocket transport: ${error.message}`);
    }
  }

  async function sendMovementInput() {
    try {
      // Send movement input (1,0,0)
      await transportActions.sendMessage({
        id: `input_${Date.now()}`,
        type: 'control',
        payload: {
          player_id: 'c3e0umn9yysaxu9',
          input_sequence: Date.now(),
          movement: [1.0, 0.0, 0.0] // x, y, z movement
        },
        timestamp: Date.now(),
        transportType: 'websocket'
      });
      addMessage('Transport', 'Movement input sent: (1,0,0)');
    } catch (error) {
      addMessage('Transport', `Error sending movement input: ${error.message}`);
    }
  }

  async function sendJumpInput() {
    try {
      // Send jump input (0,1,0)
      await transportActions.sendMessage({
        id: `input_${Date.now()}`,
        type: 'control',
        payload: {
          player_id: 'c3e0umn9yysaxu9',
          input_sequence: Date.now(),
          movement: [0.0, 1.0, 0.0] // x, y, z jump
        },
        timestamp: Date.now(),
        transportType: 'websocket'
      });
      addMessage('Transport', 'Jump input sent: (0,1,0)');
    } catch (error) {
      addMessage('Transport', `Error sending jump input: ${error.message}`);
    }
  }

  async function sendTransportMessage() {
    try {
      // Send properly formatted PlayerInput for game logic
      await transportActions.sendMessage({
        id: `input_${Date.now()}`,
        type: 'control',
        payload: {
          player_id: 'c3e0umn9yysaxu9',
          input_sequence: Date.now(),
          movement: [1.0, 0.0, 0.0] // x, y, z movement
        },
        timestamp: Date.now(),
        transportType: 'webrtc'
      });
      addMessage('Transport', 'Game input sent to Worker');
    } catch (error) {
      addMessage('Transport', `Error sending game input: ${error.message}`);
    }
  }

  async function getTransportStats() {
    try {
      const stats = transportActions.getStats();
      addMessage('Transport', `Stats: ${stats.totalMessagesSent} sent, ${stats.totalMessagesReceived} received`);
    } catch (error) {
      addMessage('Transport', `Error getting stats: ${error.message}`);
    }
  }

  async function testAllConnections() {
    addMessage('System', 'Testing all project connections...');

    // Test Gateway HTTP
    try {
      const gatewayResponse = await fetch('http://localhost:8080/healthz');
      if (gatewayResponse.ok) {
        addMessage('Gateway', '✅ Gateway HTTP is responding');
      } else {
        addMessage('Gateway', '❌ Gateway HTTP error: ' + gatewayResponse.status);
      }
    } catch (error) {
      addMessage('Gateway', '❌ Gateway HTTP connection failed: ' + error.message);
    }

    // Test Worker gRPC (via Gateway)
    try {
      const workerResponse = await fetch('http://localhost:8080/version');
      if (workerResponse.ok) {
        addMessage('Worker', '✅ Worker gRPC is responding via Gateway');
      } else {
        addMessage('Worker', '❌ Worker gRPC error: ' + workerResponse.status);
      }
    } catch (error) {
      addMessage('Worker', '❌ Worker gRPC connection failed: ' + error.message);
    }

    // Test PocketBase
    try {
      const pbResponse = await fetch('http://localhost:8090/api/health');
      if (pbResponse.ok) {
        addMessage('PocketBase', '✅ PocketBase is responding');
      } else {
        addMessage('PocketBase', '❌ PocketBase error: ' + pbResponse.status);
      }
    } catch (error) {
      addMessage('PocketBase', '❌ PocketBase connection failed: ' + error.message);
    }

    // Test Leaderboard API
    try {
      const lbResponse = await fetch('http://localhost:8080/api/leaderboard');
      if (lbResponse.ok) {
        const data = await lbResponse.json();
        addMessage('Leaderboard', `✅ Leaderboard API responding (${data.total} entries)`);
      } else {
        addMessage('Leaderboard', '❌ Leaderboard API error: ' + lbResponse.status);
      }
    } catch (error) {
      addMessage('Leaderboard', '❌ Leaderboard API connection failed: ' + error.message);
    }

    addMessage('System', 'Connection testing completed');
  }
</script>

<style>
  .container {
    max-width: 1200px;
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
    color: #f6f8ff;
  }

  header p {
    margin-top: 0.25rem;
    color: #90a0d0;
  }

  .content {
    margin-top: 2rem;
  }

  .status-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    margin-bottom: 3rem;
  }

  .status-card {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid #253157;
  }

  .status-card h3 {
    margin: 0 0 1rem 0;
    color: #f6f8ff;
    font-size: 1.2rem;
  }

  .status {
    margin: 1rem 0;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-weight: 600;
  }

  .status.connected {
    background: #1a4d3a;
    color: #4ade80;
  }

  .status.disconnected {
    background: #4d1a1a;
    color: #f87171;
  }

  .controls {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .controls button {
    padding: 0.5rem 1rem;
    background: #446bff;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.2s;
  }

  .controls button:hover:not(:disabled) {
    background: #3359e0;
  }

  .controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .test-btn {
    background: linear-gradient(135deg, #10b981, #059669) !important;
    font-weight: 600;
    border: 2px solid rgba(16, 185, 129, 0.6) !important;
  }

  .test-btn:hover:not(:disabled) {
    background: linear-gradient(135deg, #059669, #047857) !important;
    border-color: #10b981 !important;
    box-shadow: 0 4px 15px rgba(16, 185, 129, 0.3);
  }

  .websocket-section {
    margin: 3rem 0;
  }

  .websocket-section h3 {
    color: #f6f8ff;
    margin-bottom: 1rem;
  }

  .chat-container {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid #253157;
    overflow: hidden;
  }

  .messages {
    height: 300px;
    overflow-y: auto;
    padding: 1rem;
  }

  .message {
    margin-bottom: 0.5rem;
    padding: 0.5rem;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
  }

  .message.own {
    background: rgba(68, 107, 255, 0.1);
    text-align: right;
  }

  .message .sender {
    font-weight: 600;
    color: #90a0d0;
  }

  .message.own .sender {
    color: #446bff;
  }

  .input-container {
    display: flex;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.02);
    border-top: 1px solid #253157;
  }

  .input-container input {
    flex: 1;
    padding: 0.5rem;
    background: #121a2b;
    border: 1px solid #253157;
    border-radius: 4px;
    color: #f6f8ff;
    margin-right: 0.5rem;
  }

  .input-container input:focus {
    outline: none;
    border-color: #446bff;
  }

  .input-container button {
    padding: 0.5rem 1rem;
    background: #446bff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .input-container button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .info {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1.5rem;
    margin: 2rem 0;
  }

  .info h3 {
    margin: 0 0 1rem 0;
    color: #f6f8ff;
  }

  .info ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .info li {
    padding: 0.5rem 0;
    color: #c3ccec;
  }

  .actions {
    margin-top: 2rem;
    text-align: center;
  }

  .actions a {
    display: inline-block;
    padding: 1rem 2rem;
    background: #446bff;
    color: white;
    text-decoration: none;
    border-radius: 8px;
  }

  .actions a:hover {
    background: #3359e0;
  }

  .channel-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    font-size: 0.9rem;
  }

  .channel-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .channel-indicator.control {
    background: #4ade80;
  }

  .channel-indicator.state {
    background: #60a5fa;
  }

  .fallback-indicator {
    display: inline-block;
    margin-left: 0.5rem;
    padding: 0.2rem 0.5rem;
    background: #f59e0b;
    color: white;
    font-size: 0.7rem;
    font-weight: bold;
    border-radius: 3px;
    text-transform: uppercase;
  }

  .fallback-badge {
    display: inline-block;
    margin-left: 0.5rem;
    padding: 0.2rem 0.5rem;
    background: #ef4444;
    color: white;
    font-size: 0.7rem;
    font-weight: bold;
    border-radius: 3px;
    text-transform: uppercase;
  }

  .connection-info {
    margin: 0.5rem 0;
    font-size: 0.9rem;
  }

  .connection-info div {
    margin: 0.2rem 0;
    color: #c3ccec;
  }

  .fallback-reason {
    color: #f59e0b;
    font-style: italic;
  }

  .webrtc-stats {
    margin-top: 1rem;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
  }

  .webrtc-stats h4 {
    margin: 0 0 0.5rem 0;
    color: #f6f8ff;
  }

  .webrtc-stats ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .webrtc-stats li {
    padding: 0.25rem 0;
    color: #c3ccec;
    font-size: 0.9rem;
  }

  .transport-stats {
    margin-top: 1rem;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
  }

  .transport-stats h4 {
    margin: 0 0 0.5rem 0;
    color: #f6f8ff;
  }

  .transport-stats ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .transport-stats li {
    padding: 0.25rem 0;
    color: #c3ccec;
    font-size: 0.9rem;
  }

  .datachannel-test {
    margin: 2rem 0;
    padding: 1.5rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid #253157;
  }

  .datachannel-test h3 {
    margin: 0 0 1rem 0;
    color: #f6f8ff;
  }

  .connections-overview {
    margin: 3rem 0;
    padding: 2rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    border: 1px solid #253157;
  }

  .connections-overview h3 {
    margin: 0 0 2rem 0;
    color: #f6f8ff;
    font-size: 1.5rem;
    text-align: center;
  }

  .connections-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    margin-bottom: 2rem;
  }

  .connection-card {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1.5rem;
    border: 1px solid #253157;
  }

  .connection-card h4 {
    margin: 0 0 1rem 0;
    color: #f6f8ff;
    font-size: 1.1rem;
    border-bottom: 1px solid #446bff;
    padding-bottom: 0.5rem;
  }

  .connection-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .connection-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 4px;
  }

  .connection-type {
    font-weight: 600;
    color: #90a0d0;
    min-width: 120px;
  }

  .connection-status {
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 600;
    min-width: 80px;
    text-align: center;
  }

  .connection-status.connected {
    background: #1a4d3a;
    color: #4ade80;
  }

  .connection-status.disconnected {
    background: #4d1a1a;
    color: #f87171;
  }

  .connection-endpoint {
    color: #c3ccec;
    font-family: monospace;
    font-size: 0.9rem;
  }

  .architecture-diagram {
    padding: 1rem;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 8px;
  }

  .flow-diagram {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .node {
    background: #1e293b;
    border: 1px solid #475569;
    border-radius: 8px;
    padding: 1rem;
    min-width: 200px;
    text-align: center;
  }

  .node span {
    color: #f6f8ff;
    font-weight: 600;
    display: block;
    margin-bottom: 0.5rem;
  }

  .node .connections {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .conn-line {
    color: #94a3b8;
    font-size: 0.8rem;
    text-align: left;
  }

  .arrow {
    color: #446bff;
    font-size: 1.5rem;
    font-weight: bold;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .stat-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 4px;
  }

  .stat-label {
    color: #90a0d0;
    font-size: 0.9rem;
  }

  .stat-value {
    color: #f6f8ff;
    font-weight: 600;
    font-family: monospace;
  }

  @media (max-width: 768px) {
    .status-grid {
      grid-template-columns: 1fr;
    }

    .controls {
      flex-direction: column;
    }

    .controls button {
      width: 100%;
    }

    .connections-grid {
      grid-template-columns: 1fr;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }

    .connection-item {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.5rem;
    }

    .connection-type {
      min-width: auto;
    }

    .connection-status {
      min-width: auto;
    }
  }
</style>
