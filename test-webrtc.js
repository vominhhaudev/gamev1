#!/usr/bin/env node

/**
 * Test script for WebRTC Signaling
 * Tests the /rtc/offer, /rtc/answer, /rtc/ice endpoints
 */

const http = require('http');

const GATEWAY_URL = 'http://localhost:8080';

async function testWebRTCEndpoints() {
  console.log('🧪 Testing WebRTC Signaling Endpoints...\n');

  // Test 1: Offer endpoint
  console.log('1️⃣ Testing /rtc/offer endpoint...');
  try {
    const offerPayload = {
      room_id: 'test-room-123',
      peer_id: 'test-peer-456',
      offer: {
        type: 'offer',
        sdp: 'v=0\r\no=- 123456789 987654321 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic: WMS\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=candidate:1 1 UDP 2130706431 127.0.0.1 9000 typ host\r\na=setup:active\r\na=mid:0\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n'
      }
    };

    const offerResponse = await makeRequest('POST', '/rtc/offer', offerPayload);
    console.log('✅ Offer response:', JSON.stringify(offerResponse, null, 2));

    if (offerResponse.success && offerResponse.answer) {
      console.log('✅ Offer handled successfully, received answer');

      // Test 2: Answer endpoint (using the answer from offer response)
      console.log('\n2️⃣ Testing /rtc/answer endpoint...');
      const answerPayload = {
        room_id: 'test-room-123',
        peer_id: 'test-peer-456',
        answer: offerResponse.answer
      };

      const answerResponse = await makeRequest('POST', '/rtc/answer', answerPayload);
      console.log('✅ Answer response:', JSON.stringify(answerResponse, null, 2));

      if (answerResponse.success) {
        console.log('✅ Answer handled successfully');

        // Test 3: ICE candidate endpoint
        console.log('\n3️⃣ Testing /rtc/ice endpoint...');
        const icePayload = {
          room_id: 'test-room-123',
          peer_id: 'test-peer-456',
          candidate: {
            candidate: 'candidate:1 1 UDP 2130706431 127.0.0.1 9000 typ host',
            sdpMid: '0',
            sdpMLineIndex: 0,
            usernameFragment: 'abcd1234'
          }
        };

        const iceResponse = await makeRequest('POST', '/rtc/ice', icePayload);
        console.log('✅ ICE response:', JSON.stringify(iceResponse, null, 2));

        // Test 4: List sessions endpoint
        console.log('\n4️⃣ Testing /rtc/sessions endpoint...');
        const sessionsResponse = await makeRequest('GET', '/rtc/sessions');
        console.log('✅ Sessions response:', JSON.stringify(sessionsResponse, null, 2));

      } else {
        console.log('❌ Answer failed');
      }
    } else {
      console.log('❌ Offer failed');
    }

  } catch (error) {
    console.error('❌ WebRTC endpoint test failed:', error.message);
  }

  console.log('\n🎯 WebRTC Signaling Test Complete!');
}

function makeRequest(method, path, data = null) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'localhost',
      port: 8080,
      path: path,
      method: method,
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'WebRTC-Test/1.0'
      }
    };

    const req = http.request(options, (res) => {
      let body = '';

      res.on('data', (chunk) => {
        body += chunk;
      });

      res.on('end', () => {
        try {
          const response = body ? JSON.parse(body) : {};
          resolve(response);
        } catch (error) {
          reject(new Error(`Failed to parse response: ${error.message}`));
        }
      });
    });

    req.on('error', (error) => {
      reject(error);
    });

    if (data) {
      req.write(JSON.stringify(data));
    }

    req.end();
  });
}

// Run the test
testWebRTCEndpoints().catch(console.error);
