// Simple WebSocket test script
const WebSocket = require('ws');

console.log('Testing WebSocket connection to ws://localhost:8080/ws...');

const ws = new WebSocket('ws://localhost:8080/ws');

ws.on('open', () => {
  console.log('✅ WebSocket connected successfully');

  // Send a simple text message
  ws.send('Hello from test script');

  // Close after 2 seconds
  setTimeout(() => {
    ws.close();
  }, 2000);
});

ws.on('message', (data) => {
  console.log('📨 Received:', data.toString());
});

ws.on('error', (error) => {
  console.error('❌ WebSocket error:', error.message);
});

ws.on('close', (code, reason) => {
  console.log('🔌 WebSocket closed:', code, reason.toString());
});
