import { writable, derived } from 'svelte/store';
import type { GameSnapshot, PlayerInput, EntitySnapshot } from './types';

// Game state store
export const gameState = writable<GameSnapshot | null>(null);

// Current player store
export const currentPlayer = writable<string | null>(null);

// Connection status
export const isConnected = writable<boolean>(false);
export const connectionError = writable<string | null>(null);

// Game statistics
export const gameStats = derived(gameState, ($gameState) => {
    if (!$gameState) return null;

    const players = $gameState.entities.filter(e => e.player).length;
    const pickups = $gameState.entities.filter(e => e.pickup).length;
    const enemies = $gameState.entities.filter(e => e.enemy).length;
    const obstacles = $gameState.entities.filter(e => e.obstacle).length;

    return {
        tick: $gameState.tick,
        totalEntities: $gameState.entities.length,
        players,
        pickups,
        enemies,
        obstacles
    };
});

// Game service class để quản lý kết nối và giao tiếp với worker
export class GameService {
    private grpc: any = null;
    private client: any = null;
    private roomId: string = 'default_room';
    private playerId: string = '';
    private inputSequence: number = 0;
    private initialized: boolean = false;
    private snapshotInterval: number | null = null;
    private lastSnapshotTick: number = 0;

    constructor() {
        // Don't initialize immediately - will be called when needed
    }

    async initializeGrpc() {
        if (this.initialized) return;
        this.initialized = true;
        try {
            // For now, we'll use HTTP API instead of direct gRPC connection
            // This avoids CORS issues and works better in browser environment
            console.log('✅ Game service initialized (using HTTP API)');

            // Test connection to gateway
            const response = await fetch('http://localhost:8080/healthz');
            if (response.ok) {
                isConnected.set(true);
                connectionError.set(null);
                console.log('✅ Connected to game gateway');
            } else {
                throw new Error('Gateway not responding');
            }
        } catch (error) {
            console.error('❌ Failed to connect to game gateway:', error);
            isConnected.set(false);
            connectionError.set(error.message);
        }
    }

    async joinRoom(playerId: string): Promise<boolean> {
        if (!this.initialized) {
            await this.initializeGrpc();
        }

        this.playerId = playerId;

        try {
            // Use HTTP API instead of gRPC
            const response = await fetch(`http://localhost:8080/api/rooms/${this.roomId}/join`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    player_id: playerId,
                    player_name: `Player_${playerId.slice(0, 8)}`
                })
            });

            const data = await response.json();

            if (response.ok && data.success) {
                currentPlayer.set(playerId);
                console.log(`✅ Player ${playerId} joined room ${this.roomId}`);
                return true;
            } else {
                throw new Error(data.error || 'Failed to join room');
            }
        } catch (error) {
            console.error('❌ Failed to join room:', error);
            connectionError.set(error.message);
            return false;
        }
    }


    // Start receiving game snapshots for real-time sync
    async startSnapshotSync() {
        if (!this.playerId) {
            console.warn('Player not joined any room');
            return;
        }

        try {
            // Start receiving snapshots from HTTP API
            console.log('Starting snapshot sync...');

            // Use polling for now (in production, this would use WebSocket)
            this.snapshotInterval = window.setInterval(() => {
                this.requestSnapshot();
            }, 1000 / 60); // 60 FPS

        } catch (error) {
            console.error('Failed to start snapshot sync:', error);
        }
    }

    // Stop snapshot sync
    stopSnapshotSync() {
        if (this.snapshotInterval) {
            clearInterval(this.snapshotInterval);
            this.snapshotInterval = null;
        }
    }

    // Request a snapshot from server
    async requestSnapshot() {
        if (!this.roomId || !this.playerId) {
            return;
        }

        try {
            // Use HTTP API to get snapshot
            const response = await fetch(`http://localhost:8080/api/rooms/${this.roomId}/snapshot?player_id=${this.playerId}`);

            if (response.ok) {
                const snapshot = await response.json();
                this.handleSnapshot(snapshot);
            } else {
                console.warn('Failed to get snapshot:', response.status);
            }

        } catch (error) {
            console.error('Failed to request snapshot:', error);
        }
    }

    // Handle received snapshot
    handleSnapshot(snapshot: GameSnapshot) {
        // Update game state store with received snapshot
        gameState.set(snapshot);
        this.lastSnapshotTick = snapshot.tick;

        console.log('Received snapshot:', snapshot.tick, 'entities:', snapshot.entities.length);
    }

    // Join game room and start synchronization
    async joinGame(roomId: string, playerId: string) {
        this.roomId = roomId;
        this.playerId = playerId;

        try {
            // Initialize gRPC connection if needed
            await this.initializeGrpc();

            // Join room via gRPC
            // In real implementation, this would call worker's join_room RPC

            // Start receiving snapshots
            await this.startSnapshotSync();

            isConnected.set(true);
            console.log('Joined game room:', roomId);

        } catch (error) {
            console.error('Failed to join game:', error);
            connectionError.set('Failed to join game room');
        }
    }

    // Leave game room
    async leaveGame() {
        this.stopSnapshotSync();

        try {
            // Leave room via gRPC if connected
            if (this.client) {
                // Call worker's leave_room RPC
            }

            // Reset state
            gameState.set(null);
            isConnected.set(false);
            this.roomId = 'default_room';
            this.playerId = '';

            console.log('Left game room');

        } catch (error) {
            console.error('Failed to leave game:', error);
        }
    }

    // Send input to server
    async sendInput(input: PlayerInput) {
        if (!this.roomId || !this.playerId) {
            console.warn('Not connected to game');
            return;
        }

        try {
            // Send input via HTTP API
            const response = await fetch(`http://localhost:8080/api/rooms/${this.roomId}/input`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    player_id: this.playerId,
                    input_sequence: this.inputSequence++,
                    movement: input.movement,
                    timestamp: input.timestamp
                })
            });

            if (!response.ok) {
                console.warn('Failed to send input:', response.status);
            }

        } catch (error) {
            console.error('Failed to send input:', error);
        }
    }

    disconnect() {
        this.stopSnapshotSync();
        this.client = null;
        this.grpc = null;
        this.initialized = false;
        isConnected.set(false);
        currentPlayer.set(null);
        gameState.set(null);
        console.log('🔌 Disconnected from game service');
    }

    getCurrentPlayerId(): string | null {
        let playerId: string | null = null;
        currentPlayer.subscribe(p => playerId = p)();
        return playerId;
    }

    getCurrentGameState(): GameSnapshot | null {
        let state: GameSnapshot | null = null;
        gameState.subscribe(s => state = s)();
        return state;
    }
}

// Export singleton instance
export const gameService = new GameService();

// Export actions for easy use in components
export const gameActions = {
    async joinGame(roomId: string, playerId: string) {
        return await gameService.joinGame(roomId, playerId);
    },

    async leaveGame() {
        return await gameService.leaveGame();
    },


    startSnapshotSync() {
        return gameService.startSnapshotSync();
    },

    stopSnapshotSync() {
        return gameService.stopSnapshotSync();
    },

    handleSnapshot(snapshot: GameSnapshot) {
        return gameService.handleSnapshot(snapshot);
    },

    getCurrentPlayerId(): string | null {
        return gameService.getCurrentPlayerId();
    },

    getCurrentGameState(): GameSnapshot | null {
        return gameService.getCurrentGameState();
    },

    isConnected(): boolean {
        let connected = false;
        isConnected.subscribe(c => connected = c)();
        return connected;
    }
};
