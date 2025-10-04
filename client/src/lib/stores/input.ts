import { writable, derived } from 'svelte/store';
import { gameService } from './game';
import type { InputState } from './types';

// Input validation errors
export interface InputValidationError {
    field: string;
    message: string;
}

// Input validation store
export const inputValidationErrors = writable<InputValidationError[]>([]);

// Client-side input validation
export class InputValidator {
    private static readonly MAX_MOVEMENT_MAGNITUDE = 10.0;

    static validateMovement(movement: [number, number, number]): InputValidationError | null {
        for (let i = 0; i < movement.length; i++) {
            const val = movement[i];

            if (isNaN(val)) {
                return { field: `movement[${i}]`, message: 'Movement value cannot be NaN' };
            }

            if (!isFinite(val)) {
                return { field: `movement[${i}]`, message: 'Movement value must be finite' };
            }

            if (Math.abs(val) > this.MAX_MOVEMENT_MAGNITUDE) {
                return {
                    field: `movement[${i}]`,
                    message: `Movement magnitude too large: ${Math.abs(val)} > ${this.MAX_MOVEMENT_MAGNITUDE}`
                };
            }
        }

        return null;
    }

    static validatePlayerId(playerId: string): InputValidationError | null {
        if (!playerId || playerId.trim().length === 0) {
            return { field: 'player_id', message: 'Player ID is required' };
        }

        if (playerId.length > 50) {
            return { field: 'player_id', message: 'Player ID too long (max 50 characters)' };
        }

        // Allow only alphanumeric, underscore, and hyphen
        if (!/^[a-zA-Z0-9_-]+$/.test(playerId)) {
            return { field: 'player_id', message: 'Player ID contains invalid characters' };
        }

        return null;
    }

    static validateSequence(sequence: number): InputValidationError | null {
        if (!Number.isInteger(sequence) || sequence < 0) {
            return { field: 'sequence', message: 'Sequence must be a non-negative integer' };
        }

        if (sequence > Number.MAX_SAFE_INTEGER) {
            return { field: 'sequence', message: 'Sequence number too large' };
        }

        return null;
    }

    static validateTimestamp(timestamp: number): InputValidationError | null {
        const now = Date.now();
        const maxDiff = 10000; // 10 seconds

        if (Math.abs(now - timestamp) > maxDiff) {
            return { field: 'timestamp', message: 'Timestamp is too far from current time' };
        }

        return null;
    }

    static validateInput(input: any): InputValidationError[] {
        const errors: InputValidationError[] = [];

        if (input.player_id !== undefined) {
            const playerIdError = this.validatePlayerId(input.player_id);
            if (playerIdError) errors.push(playerIdError);
        }

        if (input.input_sequence !== undefined) {
            const sequenceError = this.validateSequence(input.input_sequence);
            if (sequenceError) errors.push(sequenceError);
        }

        if (input.movement !== undefined) {
            const movementError = this.validateMovement(input.movement);
            if (movementError) errors.push(movementError);
        }

        if (input.timestamp !== undefined) {
            const timestampError = this.validateTimestamp(input.timestamp);
            if (timestampError) errors.push(timestampError);
        }

        return errors;
    }
}

// Input state store
export const inputState = writable<InputState>({
    forward: false,
    backward: false,
    left: false,
    right: false,
    jump: false,
    sprint: false
});

// Movement vector derived từ input state
export const movementVector = derived(inputState, ($inputState) => {
    let x = 0;
    let z = 0;

    if ($inputState.left) x -= 1;
    if ($inputState.right) x += 1;
    if ($inputState.forward) z -= 1;
    if ($inputState.backward) z += 1;

    // Normalize diagonal movement
    if (x !== 0 && z !== 0) {
        const length = Math.sqrt(x * x + z * z);
        x /= length;
        z /= length;
    }

    // Apply sprint multiplier
    if ($inputState.sprint) {
        x *= 2;
        z *= 2;
    }

    return [x, 0, z] as [number, number, number];
});

// Game loop store để quản lý việc gửi input liên tục
export class GameLoop {
    private intervalId: number | null = null;
    private isRunning = false;
    private lastUpdate = 0;
    private readonly targetFPS = 60;
    private readonly frameTime = 1000 / this.targetFPS;

    start() {
        if (this.isRunning) return;

        this.isRunning = true;
        this.lastUpdate = performance.now();

        this.intervalId = window.setInterval(() => {
            this.update();
        }, this.frameTime);
    }

    stop() {
        if (this.intervalId) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }
        this.isRunning = false;
    }

    private async update() {
        const now = performance.now();
        const deltaTime = now - this.lastUpdate;

        if (deltaTime >= this.frameTime) {
            await this.sendInput();
            this.lastUpdate = now;
        }
    }

    private async sendInput() {
        // Get current movement vector
        let movement: [number, number, number] = [0, 0, 0];
        movementVector.subscribe(m => movement = m)();

        // Validate input trước khi gửi
        const validationErrors = InputValidator.validateMovement(movement);
        if (validationErrors) {
            inputValidationErrors.set([validationErrors]);
            return; // Don't send invalid input
        }

        // Clear validation errors if input is valid
        inputValidationErrors.set([]);

        // Send input to game service nếu có kết nối
        if (gameService.getCurrentPlayerId()) {
            // Ensure game service is initialized before sending input
            await gameService.initializeGrpc();
            await gameService.sendInput(movement);
        }
    }

    isGameLoopRunning(): boolean {
        return this.isRunning;
    }
}

// Export singleton instance
export const gameLoop = new GameLoop();

// Keyboard event handlers
export function handleKeyDown(event: KeyboardEvent) {
    switch (event.code) {
        case 'KeyW':
        case 'ArrowUp':
            inputState.update(state => ({ ...state, forward: true }));
            break;
        case 'KeyS':
        case 'ArrowDown':
            inputState.update(state => ({ ...state, backward: true }));
            break;
        case 'KeyA':
        case 'ArrowLeft':
            inputState.update(state => ({ ...state, left: true }));
            break;
        case 'KeyD':
        case 'ArrowRight':
            inputState.update(state => ({ ...state, right: true }));
            break;
        case 'Space':
            event.preventDefault();
            inputState.update(state => ({ ...state, jump: true }));
            break;
        case 'ShiftLeft':
        case 'ShiftRight':
            inputState.update(state => ({ ...state, sprint: true }));
            break;
    }
}

export function handleKeyUp(event: KeyboardEvent) {
    switch (event.code) {
        case 'KeyW':
        case 'ArrowUp':
            inputState.update(state => ({ ...state, forward: false }));
            break;
        case 'KeyS':
        case 'ArrowDown':
            inputState.update(state => ({ ...state, backward: false }));
            break;
        case 'KeyA':
        case 'ArrowLeft':
            inputState.update(state => ({ ...state, left: false }));
            break;
        case 'KeyD':
        case 'ArrowRight':
            inputState.update(state => ({ ...state, right: false }));
            break;
        case 'Space':
            inputState.update(state => ({ ...state, jump: false }));
            break;
        case 'ShiftLeft':
        case 'ShiftRight':
            inputState.update(state => ({ ...state, sprint: false }));
            break;
    }
}

// Mouse handling for camera (nếu cần sau này)
export function handleMouseMove(event: MouseEvent) {
    // Có thể implement camera rotation sau
}

// Initialize input handlers
export function initializeInputHandlers() {
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('mousemove', handleMouseMove);

    // Cleanup function
    return () => {
        window.removeEventListener('keydown', handleKeyDown);
        window.removeEventListener('keyup', handleKeyUp);
        window.removeEventListener('mousemove', handleMouseMove);
    };
}

