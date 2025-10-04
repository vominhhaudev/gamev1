// Game data types matching the Rust backend
export interface TransformQ {
    position: [number, number, number];
    rotation: [number, number, number, number];
}

export interface VelocityQ {
    velocity: [number, number, number];
    angular_velocity: [number, number, number];
}

export interface Player {
    id: string;
    score: number;
}

export interface Pickup {
    value: number;
}

export interface Obstacle {
    obstacle_type: string;
}

export interface PowerUp {
    power_type: string;
    duration: number; // in seconds
    value: number;
}

export interface Enemy {
    enemy_type: string;
    damage: number;
    speed: number;
    last_attack: number;
    attack_cooldown: number;
}

export interface EntitySnapshot {
    id: number;
    transform: TransformQ;
    velocity?: VelocityQ;
    player?: Player;
    pickup?: Pickup;
    obstacle?: Obstacle;
    power_up?: PowerUp;
    enemy?: Enemy;
}

export interface ChatMessage {
    id: string;
    player_id: string;
    player_name: string;
    message: string;
    timestamp: number;
    message_type: ChatMessageType;
}

export interface SpectatorSnapshot {
    id: string;
    transform: TransformQ;
    camera_mode: string;
    target_player_id?: string;
    view_distance: number;
}

export interface GameSnapshot {
    tick: number;
    entities: EntitySnapshot[];
    chat_messages?: ChatMessage[];
    spectators?: SpectatorSnapshot[];
}

export enum ChatMessageType {
    Global = 'global',
    Team = 'team',
    Whisper = 'whisper',
    System = 'system'
}

export enum SpectatorCameraMode {
    Free = 'free',
    Follow = 'follow',
    Overview = 'overview',
    Fixed = 'fixed'
}

export interface PlayerInput {
    player_id: string;
    input_sequence: number;
    movement: [number, number, number];
    timestamp: number;
}

// UI-related types
export interface GameStats {
    tick: number;
    totalEntities: number;
    players: number;
    pickups: number;
    enemies: number;
    obstacles: number;
}

// Input state for keyboard/mouse handling
export interface InputState {
    forward: boolean;
    backward: boolean;
    left: boolean;
    right: boolean;
    jump: boolean;
    sprint: boolean;
}

// Room management types
export interface RoomSettings {
    maxPlayers: number;
    gameMode: GameMode;
    mapName: string;
    timeLimit?: number; // seconds
    hasPassword: boolean;
    isPrivate: boolean;
    allowSpectators: boolean;
    autoStart: boolean;
    minPlayersToStart: number;
}

export interface RoomPlayer {
    id: string;
    name: string;
    joinedAt: number;
    isReady: boolean;
    isHost: boolean;
    team?: string;
    score: number;
    ping: number;
    lastSeen: number;
}

export interface RoomSpectator {
    id: string;
    name: string;
    joinedAt: number;
}

export interface Room {
    id: string;
    name: string;
    settings: RoomSettings;
    state: RoomState;
    players: RoomPlayer[];
    spectators: RoomSpectator[];
    hostId: string;
    createdAt: number;
    startedAt?: number;
    endedAt?: number;
    gameWorldId?: string;
}

export interface RoomInfo {
    id: string;
    name: string;
    settings: RoomSettings;
    state: RoomState;
    playerCount: number;
    spectatorCount: number;
    maxPlayers: number;
    hasPassword: boolean;
    gameMode: GameMode;
    createdAt: number; // seconds ago
}

export interface RoomListFilter {
    gameMode?: GameMode;
    hasPassword?: boolean;
    minPlayers?: number;
    maxPlayers?: number;
    state?: RoomState;
}

export interface CreateRoomRequest {
    roomName: string;
    hostId: string;
    hostName: string;
    settings?: RoomSettings;
}

export interface JoinRoomRequest {
    roomId: string;
    playerId: string;
    playerName: string;
}

export interface RoomOperationResponse {
    success: boolean;
    roomId?: string;
    error?: string;
    data?: any;
}

export enum RoomState {
    Waiting = 'waiting',
    Starting = 'starting',
    Playing = 'playing',
    Finished = 'finished',
    Closed = 'closed'
}

export enum GameMode {
    Deathmatch = 'deathmatch',
    TeamDeathmatch = 'team_deathmatch',
    CaptureTheFlag = 'capture_the_flag',
    KingOfTheHill = 'king_of_the_hill'
}

