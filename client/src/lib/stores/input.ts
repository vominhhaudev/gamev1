import { writable, derived } from 'svelte/store';
import { gameService } from './game';
import type { InputState } from './types';

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

        // Send input to game service nếu có kết nối
        if (gameService.getCurrentPlayerId()) {
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
