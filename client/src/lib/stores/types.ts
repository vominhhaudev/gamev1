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

export interface GameSnapshot {
    tick: number;
    entities: EntitySnapshot[];
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
