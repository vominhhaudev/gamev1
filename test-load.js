// Load Testing Script for GameV1
// Tests concurrent clients and performance under load

const GATEWAY_URL = 'http://localhost:8080';
const NUM_CLIENTS = 50; // Start with 50 clients, can increase to 100+
const CONCURRENT_BATCHES = 10; // Process clients in batches
const BATCH_SIZE = Math.ceil(NUM_CLIENTS / CONCURRENT_BATCHES);

class LoadTester {
    constructor() {
        this.gatewayUrl = GATEWAY_URL;
        this.clients = [];
        this.results = {
            totalClients: NUM_CLIENTS,
            successfulConnections: 0,
            failedConnections: 0,
            averageResponseTime: 0,
            totalHeartbeats: 0,
            successfulHeartbeats: 0,
            errors: []
        };
    }

    async runLoadTest() {
        console.log(`🚀 Starting Load Test with ${NUM_CLIENTS} concurrent clients...`);
        console.log(`📊 Batch size: ${BATCH_SIZE}, Batches: ${CONCURRENT_BATCHES}`);

        const startTime = Date.now();

        try {
            // Phase 1: Create and join rooms
            console.log('\n📱 Phase 1: Creating clients and joining rooms...');
            await this.createClientsInBatches();

            // Phase 2: Send heartbeats
            console.log('\n💓 Phase 2: Sending heartbeats...');
            await this.sendHeartbeats();

            // Phase 3: Test reconnection
            console.log('\n🔄 Phase 3: Testing reconnection...');
            await this.testReconnection();

            // Phase 4: Cleanup
            console.log('\n🧹 Phase 4: Cleaning up...');
            await this.cleanup();

        } catch (error) {
            console.error('❌ Load test failed:', error.message);
        }

        const totalTime = Date.now() - startTime;
        this.printResults(totalTime);
    }

    async createClientsInBatches() {
        for (let batch = 0; batch < CONCURRENT_BATCHES; batch++) {
            console.log(`\n🔄 Processing batch ${batch + 1}/${CONCURRENT_BATCHES}...`);

            const batchPromises = [];
            const startIndex = batch * BATCH_SIZE;
            const endIndex = Math.min(startIndex + BATCH_SIZE, NUM_CLIENTS);

            for (let i = startIndex; i < endIndex; i++) {
                batchPromises.push(this.createClient(i));
            }

            const batchResults = await Promise.allSettled(batchPromises);

            batchResults.forEach((result, index) => {
                const clientIndex = startIndex + index;
                if (result.status === 'fulfilled') {
                    this.results.successfulConnections++;
                    console.log(`✅ Client ${clientIndex}: Joined room successfully`);
                } else {
                    this.results.failedConnections++;
                    this.results.errors.push(`Client ${clientIndex}: ${result.reason.message}`);
                    console.log(`❌ Client ${clientIndex}: ${result.reason.message}`);
                }
            });

            // Small delay between batches to avoid overwhelming the server
            if (batch < CONCURRENT_BATCHES - 1) {
                await new Promise(resolve => setTimeout(resolve, 100));
            }
        }

        console.log(`\n📊 Connection Results:`);
        console.log(`   ✅ Successful: ${this.results.successfulConnections}`);
        console.log(`   ❌ Failed: ${this.results.failedConnections}`);
        console.log(`   📈 Success Rate: ${((this.results.successfulConnections / NUM_CLIENTS) * 100).toFixed(1)}%`);
    }

    async createClient(clientIndex) {
        const client = {
            id: `load-client-${clientIndex}`,
            name: `Load Client ${clientIndex}`,
            roomId: null,
            stickyToken: null
        };

        const startTime = Date.now();

        try {
            const response = await postWithTimeout(`${this.gatewayUrl}/api/rooms/find`, {
                player_id: client.id,
                player_name: client.name,
                game_mode: 'deathmatch',
                max_players: 8
            }, 5000);

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();

            if (result.success) {
                client.roomId = result.room_id;
                client.stickyToken = result.sticky_token;
                this.clients.push(client);
                return client;
            } else {
                throw new Error(result.error || 'Unknown error');
            }

        } catch (error) {
            throw new Error(`Failed to create client: ${error.message}`);
        }
    }

    async sendHeartbeats() {
        if (this.clients.length === 0) {
            console.log('⚠️ No clients available for heartbeat test');
            return;
        }

        console.log(`Sending heartbeats for ${this.clients.length} clients...`);

        const heartbeatPromises = this.clients.map(async (client) => {
            try {
                const response = await postWithTimeout(`${this.gatewayUrl}/api/rooms/heartbeat`, {
                    room_id: client.roomId,
                    player_id: client.id
                }, 3000);

                if (response.ok) {
                    this.results.successfulHeartbeats++;
                }

                return response.ok;
            } catch (error) {
                this.results.errors.push(`Heartbeat failed for ${client.id}: ${error.message}`);
                return false;
            }
        });

        await Promise.allSettled(heartbeatPromises);
        this.results.totalHeartbeats = this.clients.length;

        console.log(`💓 Heartbeat Results:`);
        console.log(`   ✅ Successful: ${this.results.successfulHeartbeats}`);
        console.log(`   ❌ Failed: ${this.results.totalHeartbeats - this.results.successfulHeartbeats}`);
        console.log(`   📈 Success Rate: ${((this.results.successfulHeartbeats / this.results.totalHeartbeats) * 100).toFixed(1)}%`);
    }

    async testReconnection() {
        if (this.clients.length === 0) {
            console.log('⚠️ No clients available for reconnection test');
            return;
        }

        console.log(`Testing reconnection for ${this.clients.length} clients...`);

        const reconnectPromises = this.clients.map(async (client) => {
            try {
                const response = await postWithTimeout(`${this.gatewayUrl}/api/rooms/reconnect`, {
                    sticky_token: client.stickyToken,
                    player_id: client.id
                }, 3000);

                if (response.ok) {
                    const result = await response.json();
                    return result.success === true;
                }
                return false;
            } catch (error) {
                return false;
            }
        });

        const reconnectResults = await Promise.allSettled(reconnectPromises);
        const successfulReconnects = reconnectResults.filter(r => r.status === 'fulfilled' && r.value === true).length;

        console.log(`🔄 Reconnection Results:`);
        console.log(`   ✅ Successful: ${successfulReconnects}`);
        console.log(`   ❌ Failed: ${this.clients.length - successfulReconnects}`);
        console.log(`   📈 Success Rate: ${((successfulReconnects / this.clients.length) * 100).toFixed(1)}%`);
    }

    async cleanup() {
        console.log('\n🧹 Cleaning up test clients...');

        const cleanupPromises = this.clients.map(async (client) => {
            try {
                await postWithTimeout(`${this.gatewayUrl}/game/leave`, {
                    room_id: client.roomId,
                    player_id: client.id
                }, 2000).catch(() => {}); // Ignore cleanup errors
            } catch (error) {
                // Ignore cleanup errors
            }
        });

        await Promise.allSettled(cleanupPromises);
        this.clients = [];
        console.log('✅ Cleanup completed');
    }

    printResults(totalTime) {
        console.log('\n' + '='.repeat(60));
        console.log('📊 LOAD TEST RESULTS');
        console.log('='.repeat(60));

        console.log(`⏱️  Total Time: ${totalTime}ms`);
        console.log(`👥 Total Clients: ${this.results.totalClients}`);
        console.log(`🔗 Successful Connections: ${this.results.successfulConnections}`);
        console.log(`❌ Failed Connections: ${this.results.failedConnections}`);
        console.log(`💓 Total Heartbeats: ${this.results.totalHeartbeats}`);
        console.log(`💚 Successful Heartbeats: ${this.results.successfulHeartbeats}`);

        const connectionRate = (this.results.successfulConnections / this.results.totalClients) * 100;
        const heartbeatRate = this.results.totalHeartbeats > 0 ?
            (this.results.successfulHeartbeats / this.results.totalHeartbeats) * 100 : 0;

        console.log(`📈 Connection Success Rate: ${connectionRate.toFixed(1)}%`);
        console.log(`📈 Heartbeat Success Rate: ${heartbeatRate.toFixed(1)}%`);

        if (this.results.errors.length > 0) {
            console.log(`\n❌ Errors (${this.results.errors.length}):`);
            this.results.errors.slice(0, 5).forEach(error => {
                console.log(`   • ${error}`);
            });
            if (this.results.errors.length > 5) {
                console.log(`   ... and ${this.results.errors.length - 5} more errors`);
            }
        }

        console.log('\n🏆 Performance Assessment:');
        if (connectionRate >= 90 && heartbeatRate >= 90) {
            console.log('   ✅ EXCELLENT - Ready for production');
        } else if (connectionRate >= 80 && heartbeatRate >= 80) {
            console.log('   ⚠️  GOOD - Minor optimizations needed');
        } else if (connectionRate >= 70 && heartbeatRate >= 70) {
            console.log('   🔶 FAIR - Performance tuning required');
        } else {
            console.log('   ❌ POOR - Major issues need fixing');
        }

        console.log('='.repeat(60));
    }
}

// Helper function for fetch with timeout
async function fetchWithTimeout(url, options, timeout = 5000) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
        const response = await fetch(url, {
            ...options,
            signal: controller.signal
        });
        clearTimeout(timeoutId);
        return response;
    } catch (error) {
        clearTimeout(timeoutId);
        if (error.name === 'AbortError') {
            throw new Error(`Request timeout after ${timeout}ms`);
        }
        throw error;
    }
}

// Helper function for fetch with timeout
async function postWithTimeout(url, data, timeout = 5000) {
    return fetchWithTimeout(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
    }, timeout);
}

// Helper function for fetch with timeout
async function getWithTimeout(url, timeout = 5000) {
    return fetchWithTimeout(url, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' }
    }, timeout);
}

// Run the load test
const tester = new LoadTester();
tester.runLoadTest().catch(console.error);
