import { writable, derived } from 'svelte/store';
import type { Room, RoomInfo, RoomSettings, RoomListFilter, CreateRoomRequest, JoinRoomRequest, RoomOperationResponse, RoomState, GameMode } from './types';

// Room state store
export const currentRoom = writable<Room | null>(null);

// Room list store
export const roomList = writable<RoomInfo[]>([]);

// Room loading states
export const isLoadingRooms = writable<boolean>(false);
export const isCreatingRoom = writable<boolean>(false);
export const isJoiningRoom = writable<boolean>(false);

// Room error store
export const roomError = writable<string | null>(null);

// Gateway base URL
const GATEWAY_BASE_URL = 'http://localhost:8080';

// Room service class để quản lý room operations
export class RoomService {
    private gatewayUrl: string;

    constructor(gatewayUrl: string = GATEWAY_BASE_URL) {
        this.gatewayUrl = gatewayUrl;
    }

    async createRoom(request: CreateRoomRequest): Promise<RoomOperationResponse> {
        isCreatingRoom.set(true);
        roomError.set(null);

        try {
            const response = await fetch(`${this.gatewayUrl}/api/rooms/create`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(request),
            });

            const data = await response.json();

            if (data.success) {
                return {
                    success: true,
                    roomId: data.room_id,
                    data: data,
                };
            } else {
                roomError.set(data.error || 'Failed to create room');
                return {
                    success: false,
                    error: data.error || 'Failed to create room',
                };
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Network error';
            roomError.set(errorMessage);
            return {
                success: false,
                error: errorMessage,
            };
        } finally {
            isCreatingRoom.set(false);
        }
    }

    async listRooms(filter?: RoomListFilter): Promise<RoomOperationResponse> {
        isLoadingRooms.set(true);
        roomError.set(null);

        try {
            const params = new URLSearchParams();
            if (filter) {
                if (filter.gameMode) params.append('gameMode', filter.gameMode);
                if (filter.hasPassword !== undefined) params.append('hasPassword', filter.hasPassword.toString());
                if (filter.minPlayers) params.append('minPlayers', filter.minPlayers.toString());
                if (filter.maxPlayers) params.append('maxPlayers', filter.maxPlayers.toString());
                if (filter.state) params.append('state', filter.state);
            }

            const url = `${this.gatewayUrl}/api/rooms/list${params.toString() ? `?${params.toString()}` : ''}`;
            const response = await fetch(url);

            const data = await response.json();

            if (data.success) {
                // Transform server room data to client format
                const transformedRooms = (data.rooms || []).map(room => transformServerRoom(room));
                roomList.set(transformedRooms);
                return {
                    success: true,
                    data: transformedRooms,
                };
            } else {
                roomError.set(data.error || 'Failed to list rooms');
                return {
                    success: false,
                    error: data.error || 'Failed to list rooms',
                };
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Network error';
            roomError.set(errorMessage);
            return {
                success: false,
                error: errorMessage,
            };
        } finally {
            isLoadingRooms.set(false);
        }
    }

    async getRoomInfo(roomId: string): Promise<RoomOperationResponse> {
        roomError.set(null);

        try {
            const response = await fetch(`${this.gatewayUrl}/api/rooms/${roomId}`);

            const data = await response.json();

            if (data.success) {
                return {
                    success: true,
                    data: data.room,
                };
            } else {
                roomError.set(data.error || 'Failed to get room info');
                return {
                    success: false,
                    error: data.error || 'Failed to get room info',
                };
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Network error';
            roomError.set(errorMessage);
            return {
                success: false,
                error: errorMessage,
            };
        }
    }

    async joinRoom(request: JoinRoomRequest): Promise<RoomOperationResponse> {
        isJoiningRoom.set(true);
        roomError.set(null);

        try {
            const response = await fetch(`${this.gatewayUrl}/api/rooms/join-player`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(request),
            });

            const data = await response.json();

            if (data.success) {
                // Refresh room list after joining
                await this.listRooms();
                return {
                    success: true,
                    roomId: request.roomId,
                    data: data,
                };
            } else {
                roomError.set(data.error || 'Failed to join room');
                return {
                    success: false,
                    error: data.error || 'Failed to join room',
                };
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Network error';
            roomError.set(errorMessage);
            return {
                success: false,
                error: errorMessage,
            };
        } finally {
            isJoiningRoom.set(false);
        }
    }

    async startGame(roomId: string, playerId: string): Promise<RoomOperationResponse> {
        roomError.set(null);

        try {
            const response = await fetch(`${this.gatewayUrl}/api/rooms/start-game`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ roomId, playerId }),
            });

            const data = await response.json();

            if (data.success) {
                // Refresh room info after starting game
                await this.getRoomInfo(roomId);
                return {
                    success: true,
                    roomId,
                    data: data,
                };
            } else {
                roomError.set(data.error || 'Failed to start game');
                return {
                    success: false,
                    error: data.error || 'Failed to start game',
                };
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Network error';
            roomError.set(errorMessage);
            return {
                success: false,
                error: errorMessage,
            };
        }
    }

    // Get current room ID
    getCurrentRoomId(): string | null {
        let room: Room | null = null;
        currentRoom.subscribe(r => room = r)();
        return room?.id || null;
    }

    // Leave current room
    async leaveRoom(): Promise<RoomOperationResponse> {
        const roomId = this.getCurrentRoomId();
        if (!roomId) {
            return { success: false, error: 'Not in a room' };
        }

        roomError.set(null);

        try {
            // For now, just clear local state
            // In a real implementation, you might want to call a leave endpoint
            currentRoom.set(null);
            await this.listRooms(); // Refresh room list

            return {
                success: true,
                roomId,
            };
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Network error';
            roomError.set(errorMessage);
            return {
                success: false,
                error: errorMessage,
            };
        }
    }

    // Get room by ID from current list
    getRoomById(roomId: string): RoomInfo | null {
        let rooms: RoomInfo[] = [];
        roomList.subscribe(r => rooms = r)();
        return rooms.find(room => room.id === roomId) || null;
    }

    // Check if player is in a room
    isPlayerInRoom(playerId: string): boolean {
        let room: Room | null = null;
        currentRoom.subscribe(r => room = r)();
        if (!room) return false;

        return room.players.some(player => player.id === playerId);
    }

    // Get room players
    getRoomPlayers(): RoomPlayer[] {
        let room: Room | null = null;
        currentRoom.subscribe(r => room = r)();
        return room?.players || [];
    }

    // Get room spectators
    getRoomSpectators(): RoomSpectator[] {
        let room: Room | null = null;
        currentRoom.subscribe(r => room = r)();
        return room?.spectators || [];
    }

    // Check if current player is host
    isCurrentPlayerHost(playerId: string): boolean {
        let room: Room | null = null;
        currentRoom.subscribe(r => room = r)();
        return room?.hostId === playerId;
    }

    // Check if room can be started
    canStartGame(playerId: string): boolean {
        let room: Room | null = null;
        currentRoom.subscribe(r => room = r)();

        if (!room || room.state !== RoomState.Waiting) return false;

        return room.hostId === playerId && room.players.length >= 2;
    }
}

// Utility functions for room operations
export const roomUtils = {
    // Format room state for display
    formatRoomState(state: RoomState): string {
        switch (state) {
            case RoomState.Waiting: return 'Waiting for players';
            case RoomState.Starting: return 'Starting...';
            case RoomState.Playing: return 'In progress';
            case RoomState.Finished: return 'Finished';
            case RoomState.Closed: return 'Closed';
            default: return 'Unknown';
        }
    },

    // Format game mode for display
    formatGameMode(mode: GameMode): string {
        switch (mode) {
            case GameMode.Deathmatch: return 'Deathmatch';
            case GameMode.TeamDeathmatch: return 'Team Deathmatch';
            case GameMode.CaptureTheFlag: return 'Capture the Flag';
            case GameMode.KingOfTheHill: return 'King of the Hill';
            default: return 'Unknown';
        }
    },

    // Get default room settings
    getDefaultRoomSettings(): RoomSettings {
        return {
            maxPlayers: 8,
            gameMode: GameMode.Deathmatch,
            mapName: 'default_map',
            timeLimit: 300, // 5 minutes
            hasPassword: false,
            isPrivate: false,
            allowSpectators: true,
            autoStart: true,
            minPlayersToStart: 2,
        };
    },

    // Validate room name
    validateRoomName(name: string): string | null {
        if (!name || name.trim().length === 0) {
            return 'Room name is required';
        }
        if (name.length > 50) {
            return 'Room name too long (max 50 characters)';
        }
        if (name.length < 3) {
            return 'Room name too short (min 3 characters)';
        }
        return null;
    },

    // Validate player name
    validatePlayerName(name: string): string | null {
        if (!name || name.trim().length === 0) {
            return 'Player name is required';
        }
        if (name.length > 20) {
            return 'Player name too long (max 20 characters)';
        }
        if (name.length < 2) {
            return 'Player name too short (min 2 characters)';
        }
        return null;
    },
};

// Export singleton instance
export const roomService = new RoomService();

// Transform server room data to client format
function transformServerRoom(serverRoom: any): RoomInfo {
    // Convert game_mode number to string enum
    let gameMode: GameMode = GameMode.Deathmatch; // default
    switch (serverRoom.game_mode) {
        case 0:
            gameMode = GameMode.Deathmatch;
            break;
        case 1:
            gameMode = GameMode.TeamDeathmatch;
            break;
        case 2:
            gameMode = GameMode.CaptureTheFlag;
            break;
        case 3:
            gameMode = GameMode.KingOfTheHill;
            break;
    }

    // Convert room state number to string enum
    let state: RoomState = RoomState.Waiting; // default
    switch (serverRoom.state) {
        case 0:
            state = RoomState.Waiting;
            break;
        case 1:
            state = RoomState.Starting;
            break;
        case 2:
            state = RoomState.Playing;
            break;
        case 3:
            state = RoomState.Finished;
            break;
        case 4:
            state = RoomState.Closed;
            break;
    }

    return {
        id: serverRoom.id,
        name: serverRoom.name,
        settings: {
            maxPlayers: serverRoom.max_players || 8,
            gameMode: gameMode,
            mapName: serverRoom.settings?.map_name || 'default_map',
            timeLimit: serverRoom.settings?.time_limit || 300,
            hasPassword: serverRoom.has_password || false,
            isPrivate: serverRoom.settings?.is_private || false,
            allowSpectators: serverRoom.settings?.allow_spectators || true,
            autoStart: serverRoom.settings?.auto_start || true,
            minPlayersToStart: serverRoom.settings?.min_players_to_start || 2,
        },
        state: state,
        playerCount: serverRoom.player_count || 0,
        spectatorCount: serverRoom.spectator_count || 0,
        maxPlayers: serverRoom.max_players || 8,
        hasPassword: serverRoom.has_password || false,
        gameMode: gameMode,
        createdAt: serverRoom.created_at_seconds_ago || 0,
    };
}

// Export actions for easy use
export const roomActions = {
    async createRoom(request: CreateRoomRequest): Promise<RoomOperationResponse> {
        return await roomService.createRoom(request);
    },

    async listRooms(filter?: RoomListFilter): Promise<RoomOperationResponse> {
        return await roomService.listRooms(filter);
    },

    async getRoomInfo(roomId: string): Promise<RoomOperationResponse> {
        return await roomService.getRoomInfo(roomId);
    },

    async joinRoom(request: JoinRoomRequest): Promise<RoomOperationResponse> {
        return await roomService.joinRoom(request);
    },

    async startGame(roomId: string, playerId: string): Promise<RoomOperationResponse> {
        return await roomService.startGame(roomId, playerId);
    },

    async leaveRoom(): Promise<RoomOperationResponse> {
        return await roomService.leaveRoom();
    },

    getCurrentRoomId(): string | null {
        return roomService.getCurrentRoomId();
    },

    isPlayerInRoom(playerId: string): boolean {
        return roomService.isPlayerInRoom(playerId);
    },

    isCurrentPlayerHost(playerId: string): boolean {
        return roomService.isCurrentPlayerHost(playerId);
    },

    canStartGame(playerId: string): boolean {
        return roomService.canStartGame(playerId);
    },

    getRoomPlayers(): RoomPlayer[] {
        return roomService.getRoomPlayers();
    },

    getRoomSpectators(): RoomSpectator[] {
        return roomService.getRoomSpectators();
    },

    getRoomById(roomId: string): RoomInfo | null {
        return roomService.getRoomById(roomId);
    },
};
