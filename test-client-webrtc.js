// Test script for client-side WebRTC integration
const GATEWAY_URL = 'http://localhost:8080';

class ClientWebRTCTest {
    constructor() {
        this.gatewayUrl = GATEWAY_URL;
        this.testRoomId = `test-room-${Date.now()}`;
        this.testPeerId = `test-peer-${Math.random().toString(36).substr(2, 9)}`;
    }

    async runTests() {
        console.log('🚀 Testing Client-Side WebRTC Integration...\n');

        try {
            // Test 1: Check browser support
            console.log('1️⃣ Checking browser WebRTC support...');
            if (!('RTCPeerConnection' in window)) {
                throw new Error('WebRTC not supported in this browser');
            }
            console.log('✅ WebRTC is supported');

            // Test 2: Transport negotiation
            console.log('\n2️⃣ Testing transport negotiation...');
            const capabilities = await this.testTransportNegotiation();
            console.log('✅ Server capabilities:', capabilities);

            if (capabilities.webrtc) {
                // Test 3: WebRTC signaling flow
                console.log('\n3️⃣ Testing WebRTC signaling flow...');
                await this.testWebRTCSignaling();
                console.log('✅ WebRTC signaling completed');
            } else {
                console.log('⚠️ WebRTC not available on server');
            }

            // Test 4: Transport selection
            console.log('\n4️⃣ Testing transport selection...');
            const bestTransport = await this.testTransportSelection(capabilities);
            console.log('🏆 Best transport:', bestTransport);

            console.log('\n🎉 Client WebRTC Integration Test Complete!');

        } catch (error) {
            console.error('❌ Client WebRTC test failed:', error.message);
        }
    }

    async testTransportNegotiation() {
        const response = await fetch(`${this.gatewayUrl}/api/transport/negotiate`, {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
            },
        });

        if (!response.ok) {
            throw new Error(`Server returned ${response.status}`);
        }

        return await response.json();
    }

    async testWebRTCSignaling() {
        // Create peer connection
        const pc = new RTCPeerConnection({
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' },
                { urls: 'stun:stun1.l.google.com:19302' }
            ]
        });

        // Create DataChannel for testing
        const dc = pc.createDataChannel('test', {
            ordered: true,
            maxRetransmits: 0
        });

        return new Promise(async (resolve, reject) => {
            let signalingComplete = false;

            // Handle ICE candidates
            pc.onicecandidate = async (event) => {
                if (event.candidate && !signalingComplete) {
                    try {
                        await fetch(`${this.gatewayUrl}/rtc/ice`, {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({
                                room_id: this.testRoomId,
                                peer_id: this.testPeerId,
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

            // Handle DataChannel events
            dc.onopen = () => {
                console.log('✅ DataChannel opened');
                pc.close();
                signalingComplete = true;
                resolve();
            };

            dc.onerror = (error) => {
                console.error('❌ DataChannel error:', error);
                pc.close();
                reject(error);
            };

            try {
                // Create and send offer
                const offer = await pc.createOffer();
                await pc.setLocalDescription(offer);

                const response = await fetch(`${this.gatewayUrl}/rtc/offer`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        room_id: this.testRoomId,
                        peer_id: this.testPeerId,
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
                    if (!signalingComplete) {
                        pc.close();
                        reject(new Error('WebRTC signaling timeout'));
                    }
                }, 10000);

            } catch (error) {
                pc.close();
                reject(error);
            }
        });
    }

    async testTransportSelection(capabilities) {
        // Simulate transport selection logic
        const transports = [];

        if (capabilities.quic) transports.push({ type: 'quic', latency: 20, available: true });
        if (capabilities.webrtc) transports.push({ type: 'webrtc', latency: 15, available: true });
        if (capabilities.websocket) transports.push({ type: 'websocket', latency: 50, available: true });

        // Sort by latency (lower is better)
        transports.sort((a, b) => a.latency - b.latency);

        return transports[0]?.type || 'websocket';
    }
}

// Run the test
const test = new ClientWebRTCTest();
test.runTests().catch(console.error);
