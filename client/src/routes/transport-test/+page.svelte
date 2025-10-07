<script>
  import { onMount } from 'svelte';
  import { negotiateTransport, TransportNegotiator, defaultNetworkConfig } from '$lib/transport/negotiator';
  import { createEnhancedWebSocket, defaultWebSocketConfig } from '$lib/transport/enhanced-websocket';
  import { transportActions, transportStore } from '$lib/stores/transport';
  import { WebTransportClient } from '$lib/transport/webtransport-client';

  let negotiationResult = null;
  let isNegotiating = false;
  let wsConnection = null;
  let wsStats = null;
  let wsMessages = [];
  let transportCapabilities = null;

  // Negotiation results
  let serverCapabilities = null;
  let browserCapabilities = null;
  let testResults = [];
  let selectedTransport = null;

  // Transport manager state
  let transportManagerState = null;
  let transportManagerMessages = [];
  let webTransportClient = null;
  let webTransportStats = null;

  onMount(() => {
    // Get browser capabilities
    browserCapabilities = TransportNegotiator.getBrowserCapabilities();
  });

  async function runTransportNegotiation() {
    isNegotiating = true;
    wsMessages = [];

    try {
      console.log('🚀 Starting transport negotiation...');

      // 1. Get server capabilities
      const response = await fetch(`${defaultNetworkConfig.gatewayUrl}/api/transport/negotiate`);
      serverCapabilities = await response.json();
      console.log('📡 Server capabilities:', serverCapabilities);

      // 2. Test all available transports
      const negotiator = new TransportNegotiator(defaultNetworkConfig);
      testResults = [];

      for (const transport of ['websocket', 'webrtc', 'quic']) {
        if ((transport === 'websocket' && serverCapabilities.websocket) ||
            (transport === 'webrtc' && serverCapabilities.webrtc) ||
            (transport === 'quic' && serverCapabilities.quic)) {

          const testResult = await negotiator.testTransport(transport);
          testResults.push(testResult);
          console.log(`🧪 ${transport} test result:`, testResult);
        }
      }

      // 3. Select best transport
      selectedTransport = await negotiateTransport();
      console.log('🏆 Selected transport:', selectedTransport);

      negotiationResult = {
        success: true,
        serverCapabilities,
        testResults,
        selectedTransport,
        timestamp: new Date().toISOString()
      };

    } catch (error) {
      console.error('❌ Negotiation failed:', error);
      negotiationResult = {
        success: false,
        error: error.message,
        timestamp: new Date().toISOString()
      };
    } finally {
      isNegotiating = false;
    }
  }

  async function testEnhancedWebSocket() {
    try {
      wsConnection = createEnhancedWebSocket();

      // Listen for events
      wsConnection.on('connected', () => {
        wsMessages.push({ type: 'connected', timestamp: new Date() });
        wsMessages = [...wsMessages];
      });

      wsConnection.on('disconnected', (data) => {
        wsMessages.push({ type: 'disconnected', data, timestamp: new Date() });
        wsMessages = [...wsMessages];
      });

      wsConnection.on('message', (message) => {
        wsMessages.push({ type: 'message', message, timestamp: new Date() });
        wsMessages = [...wsMessages];
      });

      wsConnection.on('latencyUpdated', (latency) => {
        wsStats = wsConnection.getStats();
      });

      wsConnection.on('error', (error) => {
        wsMessages.push({ type: 'error', error: error.message, timestamp: new Date() });
        wsMessages = [...wsMessages];
      });

      // Connect
      await wsConnection.connect();
      wsStats = wsConnection.getStats();

    } catch (error) {
      console.error('❌ WebSocket connection failed:', error);
      wsMessages.push({ type: 'error', error: error.message, timestamp: new Date() });
      wsMessages = [...wsMessages];
    }
  }

  function disconnectWebSocket() {
    if (wsConnection) {
      wsConnection.disconnect();
      wsConnection = null;
    }
  }

  function sendTestMessage() {
    if (wsConnection && wsConnection.isConnected()) {
      wsConnection.send({
        type: 'test_message',
        data: 'Hello from transport test!',
        timestamp: Date.now()
      });
    }
  }

  function clearMessages() {
    wsMessages = [];
  }

  // Transport Manager Testing Functions
  async function testTransportManager() {
    console.log('🧪 Testing Transport Manager...');

    try {
      // Subscribe to transport store updates
      const unsubscribe = transportStore.subscribe(state => {
        transportManagerState = state;
      });

      // Add WebSocket transport
      const wsTransportId = await transportActions.addTransport({
        type: 'websocket',
        endpoint: 'ws://localhost:8080/ws',
        priority: 1
      });

      console.log('✅ Added WebSocket transport:', wsTransportId);

      // Add WebRTC transport if available
      if (browserCapabilities.webrtc) {
        const webrtcTransportId = await transportActions.addTransport({
          type: 'webrtc',
          priority: 2
        });

        console.log('✅ Added WebRTC transport:', webrtcTransportId);
      }

      // Add QUIC/WebTransport if available
      if (browserCapabilities.quic) {
        const quicTransportId = await transportActions.addTransport({
          type: 'quic',
          serverUrl: 'https://localhost:8080',
          priority: 3
        });

        console.log('✅ Added QUIC transport:', quicTransportId);
      }

      // Test message sending
      setTimeout(async () => {
        try {
          await transportActions.sendMessage({
            id: 'test_' + Date.now(),
            type: 'state',
            payload: { test: 'Hello from transport manager' },
            timestamp: Date.now(),
            transportType: 'test'
          });
          console.log('✅ Sent test message via transport manager');
        } catch (error) {
          console.error('❌ Failed to send message via transport manager:', error);
        }
      }, 2000);

    } catch (error) {
      console.error('❌ Transport manager test failed:', error);
    }
  }

  async function testWebTransportDirectly() {
    console.log('🔗 Testing WebTransport directly...');

    try {
      webTransportClient = new WebTransportClient({
        serverUrl: 'https://localhost:8080',
        sessionId: 'test_session_' + Date.now(),
        timeout: 5000,
        maxRetries: 3
      });

      // Subscribe to messages
      webTransportClient.onMessage((message) => {
        transportManagerMessages.push({
          type: 'received',
          message,
          timestamp: Date.now()
        });
        transportManagerMessages = [...transportManagerMessages];
        console.log('📨 Received WebTransport message:', message);
      });

      // Connect
      await webTransportClient.connect();

      webTransportStats = webTransportClient.getStats();
      console.log('✅ WebTransport connected directly');

      // Send test message
      setTimeout(async () => {
        try {
          await webTransportClient.sendMessage({
            id: 'direct_test_' + Date.now(),
            type: 'state',
            payload: { directTest: 'Hello from WebTransport client' },
            timestamp: Date.now()
          });
          console.log('✅ Sent message directly via WebTransport');
        } catch (error) {
          console.error('❌ Failed to send message via WebTransport:', error);
        }
      }, 1000);

    } catch (error) {
      console.error('❌ WebTransport direct test failed:', error);
    }
  }

  function cleanupAllTransports() {
    console.log('🧹 Cleaning up all transports...');

    // Close WebTransport if connected
    if (webTransportClient) {
      webTransportClient.disconnect();
      webTransportClient = null;
    }

    // Close WebSocket if connected
    if (wsConnection) {
      wsConnection.close();
      wsConnection = null;
    }

    // Remove all transport manager transports
    if (transportManagerState) {
      transportManagerState.activeTransports.forEach((_, transportId) => {
        transportActions.removeTransport(transportId);
      });
    }

    console.log('✅ All transports cleaned up');
  }
</script>

<svelte:head>
  <title>Transport Test - GameV1</title>
</svelte:head>

<section class="container">
  <header>
    <h1>Transport Test - Phase 1</h1>
    <p>Testing QUIC/WebTransport, Enhanced WebSocket, and Transport Negotiation</p>
  </header>

  <div class="content">
    <!-- Transport Negotiation Section -->
    <div class="test-section">
      <h2>Transport Negotiation</h2>

      <div class="capabilities-grid">
        <div class="capability-card">
          <h3>Browser Capabilities</h3>
          {#if browserCapabilities}
            <div class="capabilities">
              <div class="capability-item">
                <span class="icon">WS</span>
                <span>WebSocket: {browserCapabilities.websocket ? 'YES' : 'NO'}</span>
              </div>
              <div class="capability-item">
                <span class="icon">RTC</span>
                <span>WebRTC: {browserCapabilities.webrtc ? 'YES' : 'NO'}</span>
              </div>
              <div class="capability-item">
                <span class="icon">QUIC</span>
                <span>QUIC: {browserCapabilities.quic ? 'YES' : 'NO'}</span>
              </div>
            </div>
          {/if}
        </div>

        <div class="capability-card">
          <h3>Server Capabilities</h3>
          {#if serverCapabilities}
            <div class="capabilities">
              <div class="capability-item">
                <span class="icon">WS</span>
                <span>WebSocket: {serverCapabilities.websocket ? 'YES' : 'NO'}</span>
              </div>
              <div class="capability-item">
                <span class="icon">RTC</span>
                <span>WebRTC: {serverCapabilities.webrtc ? 'YES' : 'NO'}</span>
              </div>
              <div class="capability-item">
                <span class="icon">QUIC</span>
                <span>QUIC: {serverCapabilities.quic ? 'YES' : 'NO'}</span>
              </div>
            </div>
          {/if}
        </div>
      </div>

      <div class="controls">
        <button on:click={runTransportNegotiation} disabled={isNegotiating}>
          {#if isNegotiating}
            Negotiating...
          {:else}
            Run Transport Negotiation
          {/if}
        </button>
      </div>

      {#if negotiationResult}
        <div class="negotiation-results">
          <h3>Negotiation Results</h3>

          {#if negotiationResult.success}
            <div class="success-section">
              <div class="test-results">
                <h4>Transport Test Results</h4>
                {#each testResults as result}
                  <div class="test-result {result.available ? 'available' : 'unavailable'}">
                    <span class="transport-name">{result.transport}</span>
                    <span class="latency">{result.latency}ms</span>
                    <span class="status">{result.available ? 'Available' : 'Unavailable'}</span>
                    {#if result.error}
                      <span class="error">Error: {result.error}</span>
                    {/if}
                  </div>
                {/each}
              </div>

              <div class="selection-result">
                <h4>Best Transport Selected</h4>
                <div class="selected-transport {selectedTransport}">
                  {selectedTransport}
                </div>
              </div>
            </div>
          {:else}
            <div class="error-section">
              <p>ERROR: Negotiation failed: {negotiationResult.error}</p>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Enhanced WebSocket Section -->
    <div class="test-section">
      <h2>Enhanced WebSocket Test</h2>

      <div class="controls">
        <button on:click={testEnhancedWebSocket} disabled={!!wsConnection}>
          Connect Enhanced WebSocket
        </button>
        <button on:click={disconnectWebSocket} disabled={!wsConnection}>
          Disconnect
        </button>
        <button on:click={sendTestMessage} disabled={!wsConnection || !wsConnection.isConnected()}>
          Send Test Message
        </button>
        <button on:click={clearMessages}>Clear Messages</button>
      </div>

      {#if wsStats}
        <div class="websocket-stats">
          <h3>Connection Stats</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="label">Latency:</span>
              <span class="value">{wsStats.latency}ms</span>
            </div>
            <div class="stat-item">
              <span class="label">Uptime:</span>
              <span class="value">{Math.floor(wsStats.uptime / 1000)}s</span>
            </div>
            <div class="stat-item">
              <span class="label">Bytes Sent:</span>
              <span class="value">{wsStats.bytesSent}</span>
            </div>
            <div class="stat-item">
              <span class="label">Bytes Received:</span>
              <span class="value">{wsStats.bytesReceived}</span>
            </div>
            <div class="stat-item">
              <span class="label">Reconnects:</span>
              <span class="value">{wsStats.reconnectCount}</span>
            </div>
          </div>
        </div>
      {/if}

      <div class="messages-section">
        <h3>📨 Messages</h3>
        <div class="messages-container">
          {#each wsMessages as message (message.timestamp)}
            <div class="message-item {message.type}">
              <span class="timestamp">{message.timestamp.toLocaleTimeString()}</span>
              <span class="type">{message.type}</span>
              {#if message.type === 'message'}
                <pre class="message-data">{JSON.stringify(message.message, null, 2)}</pre>
              {:else if message.type === 'error'}
                <span class="error-message">{message.error}</span>
              {:else if message.data}
                <span class="event-data">{JSON.stringify(message.data)}</span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Transport Manager Test Section -->
    <div class="test-section">
      <h2>Transport Manager Test</h2>

      <div class="controls">
        <button on:click={testTransportManager} disabled={!!transportManagerState?.isConnected}>
          Test Transport Manager
        </button>
        <button on:click={cleanupAllTransports}>
          Cleanup All Transports
        </button>
      </div>

      {#if transportManagerState}
        <div class="transport-manager-stats">
          <h3>Transport Manager State</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="label">Connected:</span>
              <span class="value">{transportManagerState.isConnected ? 'YES' : 'NO'}</span>
            </div>
            <div class="stat-item">
              <span class="label">Active Transports:</span>
              <span class="value">{transportManagerState.activeTransports.size}</span>
            </div>
            <div class="stat-item">
              <span class="label">Messages Sent:</span>
              <span class="value">{transportManagerState.totalMessagesSent}</span>
            </div>
            <div class="stat-item">
              <span class="label">Messages Received:</span>
              <span class="value">{transportManagerState.totalMessagesReceived}</span>
            </div>
            <div class="stat-item">
              <span class="label">Bytes Sent:</span>
              <span class="value">{transportManagerState.totalBytesSent}</span>
            </div>
            <div class="stat-item">
              <span class="label">Bytes Received:</span>
              <span class="value">{transportManagerState.totalBytesReceived}</span>
            </div>
            <div class="stat-item">
              <span class="label">Reconnects:</span>
              <span class="value">{transportManagerState.reconnectCount}</span>
            </div>
            <div class="stat-item">
              <span class="label">Errors:</span>
              <span class="value">{transportManagerState.errorCount}</span>
            </div>
          </div>
        </div>
      {/if}

      <div class="messages-section">
        <h3>📨 Transport Manager Messages</h3>
        <div class="messages-container">
          {#each transportManagerMessages as message (message.timestamp)}
            <div class="message-item {message.type}">
              <span class="timestamp">{new Date(message.timestamp).toLocaleTimeString()}</span>
              <span class="type">{message.type}</span>
              {#if message.message}
                <pre class="message-data">{JSON.stringify(message.message, null, 2)}</pre>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- WebTransport Direct Test Section -->
    <div class="test-section">
      <h2>WebTransport Direct Test</h2>

      <div class="controls">
        <button on:click={testWebTransportDirectly} disabled={!!webTransportClient}>
          Connect WebTransport Directly
        </button>
        <button on:click={cleanupAllTransports}>
          Cleanup All Transports
        </button>
      </div>

      {#if webTransportStats}
        <div class="webtransport-stats">
          <h3>WebTransport Stats</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="label">Connected:</span>
              <span class="value">{webTransportStats.connected ? 'YES' : 'NO'}</span>
            </div>
            <div class="stat-item">
              <span class="label">Type:</span>
              <span class="value">{webTransportStats.type}</span>
            </div>
            <div class="stat-item">
              <span class="label">Reconnect Attempts:</span>
              <span class="value">{webTransportStats.reconnectAttempts}</span>
            </div>
          </div>
        </div>
      {/if}

      <div class="messages-section">
        <h3>📨 WebTransport Messages</h3>
        <div class="messages-container">
          {#each transportManagerMessages as message (message.timestamp)}
            <div class="message-item {message.type}">
              <span class="timestamp">{new Date(message.timestamp).toLocaleTimeString()}</span>
              <span class="type">{message.type}</span>
              {#if message.message}
                <pre class="message-data">{JSON.stringify(message.message, null, 2)}</pre>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>

<style>
  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
  }

  header {
    text-align: center;
    margin-bottom: 3rem;
  }

  h1 {
    color: #333;
    margin-bottom: 0.5rem;
  }

  h2 {
    color: #444;
    border-bottom: 2px solid #eee;
    padding-bottom: 0.5rem;
    margin-bottom: 1.5rem;
  }

  .test-section {
    margin-bottom: 3rem;
    padding: 2rem;
    border: 1px solid #ddd;
    border-radius: 8px;
    background: #fafafa;
  }

  .capabilities-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    margin-bottom: 2rem;
  }

  .capability-card {
    padding: 1.5rem;
    background: white;
    border-radius: 6px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .capability-card h3 {
    margin-top: 0;
    color: #555;
  }

  .capabilities {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .capability-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0;
  }

  .icon {
    font-size: 1.2em;
  }

  .controls {
    display: flex;
    gap: 1rem;
    margin-bottom: 2rem;
    flex-wrap: wrap;
  }

  button {
    padding: 0.75rem 1.5rem;
    background: #007acc;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.2s;
  }

  button:hover:not(:disabled) {
    background: #005a9e;
  }

  button:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .negotiation-results {
    margin-top: 2rem;
  }

  .test-results {
    margin-bottom: 2rem;
  }

  .test-result {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr auto;
    gap: 1rem;
    padding: 1rem;
    margin-bottom: 0.5rem;
    border-radius: 4px;
    border-left: 4px solid;
  }

  .test-result.available {
    background: #f0f8f0;
    border-left-color: #4caf50;
  }

  .test-result.unavailable {
    background: #f8f0f0;
    border-left-color: #f44336;
  }

  .transport-name {
    font-weight: bold;
  }

  .latency {
    color: #666;
  }

  .status {
    color: #333;
  }

  .error {
    color: #f44336;
    font-size: 0.9em;
  }

  .selection-result {
    padding: 1.5rem;
    background: #e8f4fd;
    border-radius: 6px;
    border-left: 4px solid #2196f3;
  }

  .selected-transport {
    font-size: 1.5em;
    font-weight: bold;
    text-align: center;
    padding: 1rem;
    border-radius: 4px;
  }

  .selected-transport.quic {
    background: #4caf50;
    color: white;
  }

  .selected-transport.webrtc {
    background: #ff9800;
    color: white;
  }

  .selected-transport.websocket {
    background: #2196f3;
    color: white;
  }

  .websocket-stats, .transport-manager-stats, .webtransport-stats {
    margin-bottom: 2rem;
    padding: 1.5rem;
    background: white;
    border-radius: 6px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1rem;
  }

  .stat-item {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid #eee;
  }

  .stat-item:last-child {
    border-bottom: none;
  }

  .messages-section {
    background: white;
    border-radius: 6px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .messages-container {
    max-height: 400px;
    overflow-y: auto;
    padding: 1rem;
  }

  .message-item {
    margin-bottom: 1rem;
    padding: 0.75rem;
    border-radius: 4px;
    border-left: 3px solid;
  }

  .message-item.connected {
    background: #e8f5e8;
    border-left-color: #4caf50;
  }

  .message-item.disconnected {
    background: #fce8e8;
    border-left-color: #f44336;
  }

  .message-item.message {
    background: #f0f8ff;
    border-left-color: #2196f3;
  }

  .message-item.error {
    background: #fff0f0;
    border-left-color: #f44336;
  }

  .timestamp {
    font-size: 0.8em;
    color: #666;
    margin-right: 1rem;
  }

  .type {
    font-weight: bold;
    margin-right: 1rem;
  }

  .event-data, .error-message {
    font-family: monospace;
    font-size: 0.9em;
    color: #666;
  }

  .message-data {
    font-family: monospace;
    font-size: 0.8em;
    color: #333;
    margin: 0.5rem 0 0 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .error-message {
    color: #d32f2f;
  }

  @media (max-width: 768px) {
    .container {
      padding: 1rem;
    }

    .capabilities-grid {
      grid-template-columns: 1fr;
    }

    .controls {
      flex-direction: column;
    }

    button {
      width: 100%;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
