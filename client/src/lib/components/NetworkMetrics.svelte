<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // Network metrics state
  let metrics = {
    connectionStatus: 'disconnected',
    ping: 0,
    jitter: 0,
    packetLoss: 0,
    bandwidth: {
      upload: 0,
      download: 0
    },
    compression: {
      enabled: false,
      algorithm: 'none',
      ratio: 1.0,
      effectiveness: 0,
      totalMessages: 0,
      compressedMessages: 0
    },
    websocket: {
      messagesSent: 0,
      messagesReceived: 0,
      bytesSent: 0,
      bytesReceived: 0,
      reconnectCount: 0
    }
  };

  let updateInterval: number;
  let isVisible = false;

  onMount(() => {
    // Start metrics collection
    startMetricsCollection();

    // Auto-hide after 10 seconds
    setTimeout(() => {
      isVisible = false;
    }, 10000);
  });

  onDestroy(() => {
    if (updateInterval) {
      clearInterval(updateInterval);
    }
  });

  function startMetricsCollection() {
    updateInterval = window.setInterval(() => {
      collectMetrics();
    }, 1000);
  }

  function collectMetrics() {
    // Simulate collecting real metrics (in real implementation, this would come from transport layer)
    metrics.connectionStatus = navigator.onLine ? 'connected' : 'disconnected';
    metrics.ping = Math.random() * 100 + 10; // Simulated ping 10-110ms
    metrics.jitter = Math.random() * 20; // Simulated jitter 0-20ms
    metrics.packetLoss = Math.random() * 5; // Simulated packet loss 0-5%

    // Simulate bandwidth (bytes per second)
    metrics.bandwidth.upload = Math.floor(Math.random() * 10000) + 1000;
    metrics.bandwidth.download = Math.floor(Math.random() * 50000) + 5000;

    // Compression metrics (would come from transport layer in real implementation)
    metrics.compression.enabled = Math.random() > 0.3;
    if (metrics.compression.enabled) {
      const algorithms = ['lz4', 'zstd', 'snappy'];
      metrics.compression.algorithm = algorithms[Math.floor(Math.random() * algorithms.length)];
      metrics.compression.ratio = 0.3 + Math.random() * 0.5; // 30-80% compression ratio
      metrics.compression.effectiveness = Math.floor(Math.random() * 100);
    }

    // WebSocket metrics (would come from transport layer)
    metrics.websocket.messagesSent = Math.floor(Math.random() * 1000) + 100;
    metrics.websocket.messagesReceived = Math.floor(Math.random() * 2000) + 200;
    metrics.websocket.bytesSent = metrics.websocket.messagesSent * 150; // Assume avg 150 bytes per message
    metrics.websocket.bytesReceived = metrics.websocket.messagesReceived * 300; // Assume avg 300 bytes per message

    metrics = metrics; // Trigger reactivity
  }

  function toggleVisibility() {
    isVisible = !isVisible;
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatBandwidth(bytesPerSecond: number): string {
    return formatBytes(bytesPerSecond) + '/s';
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'connected': return '#2ecc71';
      case 'connecting': return '#f39c12';
      case 'disconnected': return '#e74c3c';
      default: return '#95a5a6';
    }
  }

  function getCompressionColor(ratio: number): string {
    if (ratio < 0.5) return '#2ecc71'; // Good compression
    if (ratio < 0.7) return '#f39c12'; // Moderate compression
    return '#e74c3c'; // Poor compression
  }
</script>

<div class="network-metrics" class:visible={isVisible}>
  <div class="metrics-header">
    <h3>📊 Network Metrics</h3>
    <button class="toggle-btn" on:click={toggleVisibility}>
      {isVisible ? '👁️' : '📊'}
    </button>
  </div>

  {#if isVisible}
    <div class="metrics-content">
      <!-- Connection Status -->
      <div class="metrics-section">
        <h4>🔗 Connection</h4>
        <div class="metric-item">
          <span class="metric-label">Status:</span>
          <span class="metric-value" style="color: {getStatusColor(metrics.connectionStatus)}">
            {metrics.connectionStatus.toUpperCase()}
          </span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Ping:</span>
          <span class="metric-value">{Math.round(metrics.ping)}ms</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Jitter:</span>
          <span class="metric-value">{Math.round(metrics.jitter)}ms</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Packet Loss:</span>
          <span class="metric-value">{Math.round(metrics.packetLoss)}%</span>
        </div>
      </div>

      <!-- Bandwidth -->
      <div class="metrics-section">
        <h4>📡 Bandwidth</h4>
        <div class="metric-item">
          <span class="metric-label">Upload:</span>
          <span class="metric-value">{formatBandwidth(metrics.bandwidth.upload)}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Download:</span>
          <span class="metric-value">{formatBandwidth(metrics.bandwidth.download)}</span>
        </div>
      </div>

      <!-- Compression -->
      <div class="metrics-section">
        <h4>🗜️ Compression</h4>
        <div class="metric-item">
          <span class="metric-label">Enabled:</span>
          <span class="metric-value">{metrics.compression.enabled ? '✅ Yes' : '❌ No'}</span>
        </div>
        {#if metrics.compression.enabled}
          <div class="metric-item">
            <span class="metric-label">Algorithm:</span>
            <span class="metric-value">{metrics.compression.algorithm.toUpperCase()}</span>
          </div>
          <div class="metric-item">
            <span class="metric-label">Ratio:</span>
            <span class="metric-value" style="color: {getCompressionColor(metrics.compression.ratio)}">
              {Math.round(metrics.compression.ratio * 100)}%
            </span>
          </div>
          <div class="metric-item">
            <span class="metric-label">Effectiveness:</span>
            <span class="metric-value">{metrics.compression.effectiveness}%</span>
          </div>
          <div class="metric-item">
            <span class="metric-label">Messages:</span>
            <span class="metric-value">
              {metrics.compression.compressedMessages}/{metrics.compression.totalMessages}
            </span>
          </div>
        {/if}
      </div>

      <!-- WebSocket Stats -->
      <div class="metrics-section">
        <h4>🔌 WebSocket</h4>
        <div class="metric-item">
          <span class="metric-label">Sent:</span>
          <span class="metric-value">{metrics.websocket.messagesSent} msgs</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Received:</span>
          <span class="metric-value">{metrics.websocket.messagesReceived} msgs</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Data Sent:</span>
          <span class="metric-value">{formatBytes(metrics.websocket.bytesSent)}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Data Received:</span>
          <span class="metric-value">{formatBytes(metrics.websocket.bytesReceived)}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Reconnects:</span>
          <span class="metric-value">{metrics.websocket.reconnectCount}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .network-metrics {
    position: fixed;
    top: 20px;
    right: 20px;
    background: rgba(0, 0, 0, 0.9);
    border: 1px solid #4a9eff;
    border-radius: 12px;
    color: white;
    font-family: 'Segoe UI', system-ui, sans-serif;
    z-index: 1000;
    backdrop-filter: blur(10px);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    min-width: 280px;
    max-width: 400px;
    opacity: 0;
    transform: translateX(100%);
    transition: all 0.3s ease;
  }

  .network-metrics.visible {
    opacity: 1;
    transform: translateX(0);
  }

  .metrics-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: rgba(74, 158, 255, 0.1);
    border-bottom: 1px solid #4a9eff;
    border-radius: 12px 12px 0 0;
  }

  .metrics-header h3 {
    margin: 0;
    color: #4a9eff;
    font-size: 1rem;
    font-weight: 600;
  }

  .toggle-btn {
    background: #4a9eff;
    color: white;
    border: none;
    padding: 0.5rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;
  }

  .toggle-btn:hover {
    background: #3a8eef;
  }

  .metrics-content {
    padding: 1rem;
    max-height: 60vh;
    overflow-y: auto;
  }

  .metrics-section {
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid rgba(74, 158, 255, 0.2);
  }

  .metrics-section:last-child {
    border-bottom: none;
    margin-bottom: 0;
    padding-bottom: 0;
  }

  .metrics-section h4 {
    margin: 0 0 0.75rem 0;
    color: #4a9eff;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .metric-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
    font-size: 0.8rem;
  }

  .metric-label {
    color: #a0a0a0;
  }

  .metric-value {
    color: #ffffff;
    font-weight: 600;
    font-family: 'Courier New', monospace;
  }

  /* Scrollbar styling */
  .metrics-content::-webkit-scrollbar {
    width: 4px;
  }

  .metrics-content::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.1);
  }

  .metrics-content::-webkit-scrollbar-thumb {
    background: #4a9eff;
    border-radius: 2px;
  }

  @media (max-width: 768px) {
    .network-metrics {
      top: 10px;
      right: 10px;
      left: 10px;
      max-width: none;
    }
  }
</style>
