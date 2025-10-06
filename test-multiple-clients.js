// Test script for multiple clients with room management
const GATEWAY_URL = 'http://localhost:8080';

class MultiClientTester {
    constructor() {
        this.clients = [];
        this.gatewayUrl = GATEWAY_URL;
    }

    async runTests() {
        console.log('🚀 Testing Multiple Clients with Room Management...\n');

        try {
            // Test 1: Create multiple clients and find rooms
            console.log('1️⃣ Creating multiple clients and testing room assignment...');
            await this.testRoomAssignment();

            // Test 2: Test heartbeat functionality
            console.log('\n2️⃣ Testing heartbeat functionality...');
            await this.testHeartbeat();

            // Test 3: Test sticky token reconnection
            console.log('\n3️⃣ Testing sticky token reconnection...');
            await this.testStickyTokenReconnection();

            console.log('\n🎉 Multiple Clients Test Complete!');

        } catch (error) {
            console.error('❌ Multiple clients test failed:', error.message);
        }
    }

    async testRoomAssignment() {
        const clients = [];

        // Create 3 clients
        for (let i = 0; i < 3; i++) {
            const client = {
                id: `client-${i + 1}`,
                name: `Player ${i + 1}`,
                stickyToken: null,
                roomId: null
            };
            clients.push(client);
        }

        // Each client finds and joins a room
        for (const client of clients) {
            console.log(`\n📱 Client ${client.id} finding room...`);

            const response = await fetch(`${this.gatewayUrl}/api/rooms/find`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    player_id: client.id,
                    player_name: client.name,
                    game_mode: 'deathmatch',
                    max_players: 4
                })
            });

            if (response.ok) {
                const result = await response.json();
                if (result.success) {
                    client.roomId = result.room_id;
                    client.stickyToken = result.sticky_token;
                    console.log(`✅ Client ${client.id} joined room ${result.room_id} (${result.room_name})`);
                } else {
                    console.log(`❌ Client ${client.id} failed to find room: ${result.error}`);
                }
            } else {
                console.log(`❌ Client ${client.id} HTTP error: ${response.status}`);
            }
        }

        // Check room list
        const roomsResponse = await fetch(`${this.gatewayUrl}/api/rooms/list`);
        if (roomsResponse.ok) {
            const rooms = await roomsResponse.json();
            console.log(`📊 Total rooms: ${rooms.rooms?.length || 0}`);
            rooms.rooms?.forEach(room => {
                console.log(`   - ${room.name} (${room.id}): ${room.player_count}/${room.max_players} players`);
            });
        }

        this.clients = clients;
    }

    async testHeartbeat() {
        if (this.clients.length === 0) {
            console.log('⚠️ No clients available for heartbeat test');
            return;
        }

        // Send heartbeat for each client
        for (const client of this.clients) {
            if (client.roomId && client.id) {
                console.log(`💓 Sending heartbeat for client ${client.id} in room ${client.roomId}`);

                const response = await fetch(`${this.gatewayUrl}/api/rooms/heartbeat`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        room_id: client.roomId,
                        player_id: client.id
                    })
                });

                if (response.ok) {
                    const result = await response.json();
                    console.log(`✅ Heartbeat for ${client.id}: ${result.message}`);
                } else {
                    console.log(`❌ Heartbeat for ${client.id} failed: ${response.status}`);
                }
            }
        }
    }

    async testStickyTokenReconnection() {
        if (this.clients.length === 0) {
            console.log('⚠️ No clients available for reconnection test');
            return;
        }

        // Test reconnection with sticky token
        for (const client of this.clients) {
            if (client.roomId && client.stickyToken) {
                console.log(`🔄 Testing reconnection for client ${client.id}`);

                const response = await fetch(`${this.gatewayUrl}/api/rooms/reconnect`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        sticky_token: client.stickyToken,
                        player_id: client.id
                    })
                });

                if (response.ok) {
                    const result = await response.json();
                    if (result.success) {
                        console.log(`✅ Client ${client.id} reconnected successfully to room ${result.room_id}`);
                    } else {
                        console.log(`❌ Client ${client.id} reconnection failed: ${result.error}`);
                    }
                } else {
                    console.log(`❌ Client ${client.id} reconnection HTTP error: ${response.status}`);
                }
            }
        }
    }

    async cleanup() {
        console.log('\n🧹 Cleaning up test clients...');

        for (const client of this.clients) {
            if (client.roomId && client.id) {
                console.log(`👋 Removing client ${client.id} from room ${client.roomId}`);

                // Leave room
                await fetch(`${this.gatewayUrl}/game/leave`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        room_id: client.roomId,
                        player_id: client.id
                    })
                }).catch(error => {
                    console.log(`   (Leave request failed: ${error.message})`);
                });
            }
        }

        this.clients = [];
    }
}

// Helper function for making HTTP requests
async function makeRequest(method, path, data = null) {
    const options = {
        method,
        headers: {
            'Content-Type': 'application/json',
            'User-Agent': 'MultiClient-Test/1.0'
        }
    };

    if (data) {
        options.body = JSON.stringify(data);
    }

    const response = await fetch(`${GATEWAY_URL}${path}`, options);
    return await response.json();
}

// Run the test
const tester = new MultiClientTester();

// Run test and cleanup
tester.runTests()
    .then(() => tester.cleanup())
    .catch(error => {
        console.error('❌ Test failed:', error.message);
        tester.cleanup();
    });
