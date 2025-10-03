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

    constructor() {
        this.initializeGrpc();
    }

    private async initializeGrpc() {
        try {
            // Import gRPC modules dynamically
            const grpc = await import('@grpc/grpc-js');
            const protoLoader = await import('@grpc/proto-loader');

            // Load proto file
            const packageDefinition = protoLoader.loadSync(
                '../../proto/worker.proto',
                {
                    keepCase: true,
                    longs: String,
                    enums: String,
                    defaults: true,
                    oneofs: true
                }
            );

            const proto = grpc.loadPackageDefinition(packageDefinition) as any;

            // Create gRPC client
            this.client = new proto.worker.v1.Worker(
                'localhost:50051',
                grpc.credentials.createInsecure()
            );

            this.grpc = grpc;
            isConnected.set(true);
            connectionError.set(null);

            console.log('✅ Connected to game worker');
        } catch (error) {
            console.error('❌ Failed to connect to game worker:', error);
            isConnected.set(false);
            connectionError.set(error.message);
        }
    }

    async joinRoom(playerId: string): Promise<boolean> {
        if (!this.client) {
            await this.initializeGrpc();
        }

        this.playerId = playerId;

        try {
            const response = await new Promise((resolve, reject) => {
                this.client.JoinRoom({
                    room_id: this.roomId,
                    player_id: playerId
                }, (error: any, response: any) => {
                    if (error) {
                        reject(error);
                    } else {
                        resolve(response);
                    }
                });
            });

            if (response.ok) {
                currentPlayer.set(playerId);
                console.log(`✅ Player ${playerId} joined room ${this.roomId}`);
                return true;
            } else {
                throw new Error(response.error || 'Failed to join room');
            }
        } catch (error) {
            console.error('❌ Failed to join room:', error);
            connectionError.set(error.message);
            return false;
        }
    }

    async sendInput(movement: [number, number, number]): Promise<GameSnapshot | null> {
        if (!this.client || !this.playerId) {
            console.warn('⚠️ Not connected or no player ID');
            return null;
        }

        this.inputSequence++;

        const input: PlayerInput = {
            player_id: this.playerId,
            input_sequence: this.inputSequence,
            movement,
            timestamp: Date.now()
        };

        try {
            const response = await new Promise((resolve, reject) => {
                this.client.PushInput({
                    room_id: this.roomId,
                    sequence: this.inputSequence,
                    payload_json: JSON.stringify(input)
                }, (error: any, response: any) => {
                    if (error) {
                        reject(error);
                    } else {
                        resolve(response);
                    }
                });
            });

            if (response.ok && response.snapshot) {
                const snapshot: GameSnapshot = JSON.parse(response.snapshot.payload_json);
                gameState.set(snapshot);
                return snapshot;
            }

            return null;
        } catch (error) {
            console.error('❌ Failed to send input:', error);
            return null;
        }
    }

    disconnect() {
        if (this.client) {
            this.client.close();
            this.client = null;
            this.grpc = null;
        }
        isConnected.set(false);
        currentPlayer.set(null);
        gameState.set(null);
        console.log('🔌 Disconnected from game worker');
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
// export const gameService = new GameService();
