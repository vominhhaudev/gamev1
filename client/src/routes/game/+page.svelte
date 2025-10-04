<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { gameState, currentPlayer, isConnected, connectionError, gameStats, gameActions } from '$lib/stores/game';
    import { roomService } from '$lib/stores/room';
    import { transportActions } from '$lib/stores/transport';
    import NetworkMetrics from '$lib/components/NetworkMetrics.svelte';

    // SvelteKit route props
    export let data;

    // Game state
    let canvas;
    let ctx;
    let animationId;
    let isRunning = false;

    // Game world properties
    const CANVAS_WIDTH = 800;
    const CANVAS_HEIGHT = 600;
    const WORLD_SIZE = 1000;

    // Player state
    let playerX = 400;
    let playerY = 300;
    let playerScore = 0;
    let playerHealth = 100;

    // Input state
    let keys = {};
    let mouseX = 0;
    let mouseY = 0;

    // Game entities (from game state store)
    let entities = [];

    // Subscribe to game state changes
    gameState.subscribe(state => {
        if (state && state.entities) {
            // Convert game state entities to renderable format
            entities = state.entities.map(entity => {
                let color = '#666666'; // default
                let size = 10;
                let type = 'obstacle';

                if (entity.player) {
                    color = '#4a9eff';
                    size = PLAYER_SIZE;
                    type = 'player';
                } else if (entity.pickup) {
                    color = '#ffd700';
                    size = 8;
                    type = 'pickup';
                } else if (entity.enemy) {
                    color = '#ff4444';
                    size = 15;
                    type = 'enemy';
                } else if (entity.obstacle) {
                    color = '#666666';
                    size = 20;
                    type = 'obstacle';
                }

                return {
                    id: entity.id,
                    x: entity.transform.position[0],
                    y: entity.transform.position[1],
                    type,
                    color,
                    size
                };
            });
        }
    });

    // Camera/viewport
    let cameraX = 0;
    let cameraY = 0;

    // Game settings
    const PLAYER_SPEED = 200; // pixels per second
    const PLAYER_SIZE = 20;

    onMount(() => {
        initializeCanvas();
        initializeGame();
        setupEventListeners();
        startGameLoop();
    });

    onDestroy(() => {
        if (animationId) {
            cancelAnimationFrame(animationId);
        }
    });

    function initializeCanvas() {
        if (!canvas) return;

        ctx = canvas.getContext('2d');
        if (!ctx) {
            console.error('Failed to get 2D context');
            return;
        }

        // Set canvas size
        canvas.width = CANVAS_WIDTH;
        canvas.height = CANVAS_HEIGHT;

        // Configure context
        ctx.imageSmoothingEnabled = true;
        ctx.font = '16px monospace';
    }

    function initializeGame() {
        // Initialize mock entities for testing
        entities = [
            // Player (center)
            { id: 1, x: 400, y: 300, type: 'player', color: '#4a9eff', size: PLAYER_SIZE },

            // Pickups (yellow circles)
            { id: 2, x: 200, y: 200, type: 'pickup', color: '#ffd700', size: 8 },
            { id: 3, x: 600, y: 150, type: 'pickup', color: '#ffd700', size: 8 },
            { id: 4, x: 350, y: 500, type: 'pickup', color: '#ffd700', size: 8 },

            // Enemies (red triangles)
            { id: 5, x: 100, y: 100, type: 'enemy', color: '#ff4444', size: 15 },
            { id: 6, x: 700, y: 450, type: 'enemy', color: '#ff4444', size: 15 },

            // Obstacles (gray squares)
            { id: 7, x: 250, y: 350, type: 'obstacle', color: '#666666', size: 30 },
            { id: 8, x: 550, y: 250, type: 'obstacle', color: '#666666', size: 30 },
        ];

        // Try to connect to game service if not connected
        if (!$isConnected) {
            connectToGame();
        }
    }

    async function connectToGame() {
        try {
            if ($isConnected) {
                console.log('Already connected to game');
                return;
            }

            // Set player ID if not set
            if (!$currentPlayer) {
                currentPlayer.set('player_' + Date.now());
            }

            // Initialize transport if needed
            await transportActions.initialize();

            // Try to join a room or create one if none exists
            const currentRoomId = roomService.getCurrentRoomId();
            if (!currentRoomId) {
                // For now, create a simple room for testing
                // In real implementation, this would come from room browser
                console.log('No current room, would need to join/create room');
                return;
            }

            // Join game room and start synchronization
            await gameActions.joinGame(currentRoomId, $currentPlayer);

            console.log('Game connection established');

        } catch (error) {
            console.error('Failed to connect to game:', error);
            connectionError.set('Failed to connect to game server');
        }
    }

    function setupEventListeners() {
        // Keyboard input
        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', handleKeyUp);

        // Mouse input
        canvas.addEventListener('mousemove', handleMouseMove);
        canvas.addEventListener('click', handleMouseClick);

        // Prevent context menu on right click
        canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    }

    function handleKeyDown(event) {
        keys[event.code] = true;

        // Handle special keys
        if (event.code === 'Space') {
            event.preventDefault();
            // Jump action
            sendPlayerInput({ jump: true });
        }

        if (event.code === 'ShiftLeft' || event.code === 'ShiftRight') {
            // Sprint mode
            sendPlayerInput({ sprint: true });
        }
    }

    function handleKeyUp(event) {
        keys[event.code] = false;

        if (event.code === 'ShiftLeft' || event.code === 'ShiftRight') {
            sendPlayerInput({ sprint: false });
        }
    }

    function handleMouseMove(event) {
        const rect = canvas.getBoundingClientRect();
        mouseX = event.clientX - rect.left;
        mouseY = event.clientY - rect.top;

        // Convert to world coordinates
        const worldMouseX = mouseX + cameraX;
        const worldMouseY = mouseY + cameraY;

        // Send mouse position for aiming/looking
        sendPlayerInput({
            mouseX: worldMouseX,
            mouseY: worldMouseY
        });
    }

    function handleMouseClick(event) {
        // Handle click actions (e.g., shooting, interaction)
        const rect = canvas.getBoundingClientRect();
        const worldX = event.clientX - rect.left + cameraX;
        const worldY = event.clientY - rect.top + cameraY;

        sendPlayerInput({
            click: true,
            clickX: worldX,
            clickY: worldY
        });
    }

    async function sendPlayerInput(input) {
        // Send input to game service
        try {
            if (!$isConnected) {
                console.warn('Not connected to game server');
                return;
            }

            // Create player input object
            const playerInput = {
                player_id: $currentPlayer || 'anonymous',
                input_sequence: Date.now(), // Use timestamp as sequence for now
                movement: input.movement || [0, 0, 0],
                timestamp: Date.now()
            };

            // Add special inputs if present
            if (input.jump) playerInput.jump = true;
            if (input.sprint !== undefined) playerInput.sprint = input.sprint;
            if (input.mouseX !== undefined) playerInput.mouse_x = input.mouseX;
            if (input.mouseY !== undefined) playerInput.mouse_y = input.mouseY;
            if (input.click) playerInput.click = true;

            // Send via game actions (which will handle gRPC communication)
            await gameActions.sendInput(playerInput);

            console.log('Player input sent:', playerInput);
        } catch (error) {
            console.error('Failed to send player input:', error);
        }
    }

    function startGameLoop() {
        if (isRunning) return;

        isRunning = true;
        let lastTime = performance.now();

        function gameLoop(currentTime) {
            const deltaTime = (currentTime - lastTime) / 1000; // Convert to seconds
            lastTime = currentTime;

            update(deltaTime);
            render();

            if (isRunning) {
                animationId = requestAnimationFrame(gameLoop);
            }
        }

        animationId = requestAnimationFrame(gameLoop);
    }

    function update(deltaTime) {
        // Update camera to follow player
        cameraX = playerX - CANVAS_WIDTH / 2;
        cameraY = playerY - CANVAS_HEIGHT / 2;

        // Handle player movement input
        let moveX = 0;
        let moveY = 0;

        if (keys['KeyW'] || keys['ArrowUp']) moveY -= 1;
        if (keys['KeyS'] || keys['ArrowDown']) moveY += 1;
        if (keys['KeyA'] || keys['ArrowLeft']) moveX -= 1;
        if (keys['KeyD'] || keys['ArrowRight']) moveX += 1;

        // Normalize diagonal movement
        if (moveX !== 0 && moveY !== 0) {
            const length = Math.sqrt(moveX * moveX + moveY * moveY);
            moveX /= length;
            moveY /= length;
        }

        // Apply sprint multiplier
        const speedMultiplier = keys['ShiftLeft'] || keys['ShiftRight'] ? 1.5 : 1.0;

        // Update player position
        const speed = PLAYER_SPEED * deltaTime * speedMultiplier;
        playerX += moveX * speed;
        playerY += moveY * speed;

        // Keep player in bounds
        playerX = Math.max(PLAYER_SIZE, Math.min(WORLD_SIZE - PLAYER_SIZE, playerX));
        playerY = Math.max(PLAYER_SIZE, Math.min(WORLD_SIZE - PLAYER_SIZE, playerY));

        // Send movement input to server
        if (moveX !== 0 || moveY !== 0) {
            sendPlayerInput({
                movement: [moveX, moveY, 0],
                timestamp: Date.now()
            });
        }

        // Check collisions with entities
        checkCollisions();
    }

    function checkCollisions() {
        entities.forEach(entity => {
            if (entity.type === 'pickup') {
                const distance = Math.sqrt(
                    Math.pow(playerX - entity.x, 2) +
                    Math.pow(playerY - entity.y, 2)
                );

                if (distance < PLAYER_SIZE + entity.size) {
                    // Collect pickup
                    playerScore += 10;
                    entity.x = Math.random() * WORLD_SIZE;
                    entity.y = Math.random() * WORLD_SIZE;

                    console.log('Pickup collected! Score:', playerScore);
                }
            }
        });
    }

    function render() {
        if (!ctx) return;

        // Clear canvas
        ctx.fillStyle = '#1a1f2e';
        ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

        // Save context for camera transform
        ctx.save();
        ctx.translate(-cameraX, -cameraY);

        // Draw world bounds
        ctx.strokeStyle = '#333333';
        ctx.lineWidth = 2;
        ctx.strokeRect(0, 0, WORLD_SIZE, WORLD_SIZE);

        // Draw grid for reference
        ctx.strokeStyle = '#2a2f3e';
        ctx.lineWidth = 1;
        for (let x = 0; x <= WORLD_SIZE; x += 100) {
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, WORLD_SIZE);
            ctx.stroke();
        }
        for (let y = 0; y <= WORLD_SIZE; y += 100) {
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(WORLD_SIZE, y);
            ctx.stroke();
        }

        // Draw entities
        entities.forEach(entity => {
            ctx.fillStyle = entity.color;

            switch (entity.type) {
                case 'player':
                    // Draw player as circle
                    ctx.beginPath();
                    ctx.arc(entity.x, entity.y, entity.size, 0, Math.PI * 2);
                    ctx.fill();

                    // Draw player name/health
                    ctx.fillStyle = '#ffffff';
                    ctx.font = '12px monospace';
                    ctx.textAlign = 'center';
                    ctx.fillText('Player', entity.x, entity.y - entity.size - 20);
                    break;

                case 'pickup':
                    // Draw pickup as small circle
                    ctx.beginPath();
                    ctx.arc(entity.x, entity.y, entity.size, 0, Math.PI * 2);
                    ctx.fill();

                    // Glow effect
                    ctx.shadowColor = entity.color;
                    ctx.shadowBlur = 10;
                    ctx.beginPath();
                    ctx.arc(entity.x, entity.y, entity.size, 0, Math.PI * 2);
                    ctx.fill();
                    ctx.shadowBlur = 0;
                    break;

                case 'enemy':
                    // Draw enemy as triangle
                    ctx.beginPath();
                    ctx.moveTo(entity.x, entity.y - entity.size);
                    ctx.lineTo(entity.x - entity.size, entity.y + entity.size);
                    ctx.lineTo(entity.x + entity.size, entity.y + entity.size);
                    ctx.closePath();
                    ctx.fill();
                    break;

                case 'obstacle':
                    // Draw obstacle as square
                    ctx.fillRect(
                        entity.x - entity.size / 2,
                        entity.y - entity.size / 2,
                        entity.size,
                        entity.size
                    );
                    break;
            }
        });

        // Draw current player (blue square for now)
        ctx.fillStyle = '#4a9eff';
        ctx.fillRect(playerX - PLAYER_SIZE / 2, playerY - PLAYER_SIZE / 2, PLAYER_SIZE, PLAYER_SIZE);

        // Draw player direction indicator
        const directionX = mouseX + cameraX - playerX;
        const directionY = mouseY + cameraY - playerY;
        const directionLength = Math.sqrt(directionX * directionX + directionY * directionY);

        if (directionLength > 0) {
            const normalizedX = directionX / directionLength;
            const normalizedY = directionY / directionLength;

            ctx.strokeStyle = '#ffffff';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(playerX, playerY);
            ctx.lineTo(
                playerX + normalizedX * 30,
                playerY + normalizedY * 30
            );
            ctx.stroke();
        }

        // Restore context
        ctx.restore();

        // Draw UI overlay (HUD)
        drawHUD();
    }

    function drawHUD() {
        if (!ctx) return;

        // Semi-transparent overlay for UI
        ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
        ctx.fillRect(0, 0, CANVAS_WIDTH, 60);

        // Player stats
        ctx.fillStyle = '#ffffff';
        ctx.font = '16px monospace';
        ctx.textAlign = 'left';

        // Display actual player score if available
        const currentPlayerEntity = entities.find(e => e.type === 'player');
        const score = currentPlayerEntity ? 'Score: Connected' : `Score: ${playerScore}`;
        ctx.fillText(score, 20, 30);
        ctx.fillText(`Health: ${playerHealth}`, 20, 50);

        // Connection status
        ctx.textAlign = 'center';
        const connectionText = $isConnected ? '🟢 Connected' : '🔴 Disconnected';
        ctx.fillText(connectionText, CANVAS_WIDTH / 2, 30);

        if ($gameStats) {
            ctx.fillText(`Entities: ${$gameStats.totalEntities} | Players: ${$gameStats.players}`, CANVAS_WIDTH / 2, 50);
        }

        // Controls help
        ctx.textAlign = 'right';
        ctx.font = '14px monospace';
        ctx.fillStyle = '#cccccc';
        ctx.fillText('WASD: Move | Mouse: Aim | Space: Jump | Shift: Sprint', CANVAS_WIDTH - 20, 30);
        ctx.fillText(`Pos: (${Math.round(playerX)}, ${Math.round(playerY)})`, CANVAS_WIDTH - 20, 50);
    }

    function togglePause() {
        isRunning = !isRunning;
        if (isRunning) {
            startGameLoop();
        } else {
            if (animationId) {
                cancelAnimationFrame(animationId);
            }
        }
    }

    function resetGame() {
        playerX = 400;
        playerY = 300;
        playerScore = 0;
        playerHealth = 100;
    }

    async function joinCurrentRoom() {
        try {
            const currentRoomId = roomService.getCurrentRoomId();
            if (!currentRoomId) {
                error = 'No room selected. Please browse and join a room first.';
                return;
            }

            if (!$currentPlayer) {
                currentPlayer.set('player_' + Date.now());
            }

            // Join game room and start synchronization
            await gameActions.joinGame(currentRoomId, $currentPlayer);

            console.log('Joined room and started game synchronization');

        } catch (error) {
            console.error('Failed to join room:', error);
            connectionError.set('Failed to join game room');
        }
    }
</script>

<svelte:head>
    <title>GameV1 - Multiplayer Game</title>
</svelte:head>

<div class="game-container">
    <div class="game-header">
        <h1>🎮 GameV1 - Multiplayer Game</h1>
        <div class="game-controls">
            <button on:click={togglePause} class="control-btn">
                {isRunning ? '⏸️ Pause' : '▶️ Resume'}
            </button>
            <button on:click={resetGame} class="control-btn">🔄 Reset</button>
            <button on:click={connectToGame} class="control-btn" disabled={$isConnected}>
                {#if $isConnected}
                    ✅ Connected
                {:else}
                    🔌 Connect
                {/if}
            </button>
            <button on:click={joinCurrentRoom} class="control-btn" disabled={!roomService.getCurrentRoomId() || $isConnected}>
                🎮 Join Room
            </button>
        </div>
    </div>

    <div class="game-content">
        <div class="game-canvas-container">
            <canvas
                bind:this={canvas}
                class="game-canvas"
                width={CANVAS_WIDTH}
                height={CANVAS_HEIGHT}
            ></canvas>

            {#if $connectionError}
                <div class="connection-error">
                    <p>{$connectionError}</p>
                    <button on:click={() => connectionError.set(null)}>×</button>
                </div>
            {/if}
        </div>

        <div class="game-sidebar">
            <div class="game-info">
                <h3>🎯 Game Info</h3>
                <div class="info-item">
                    <span class="label">Status:</span>
                    <span class="value">{$isConnected ? 'Connected' : 'Disconnected'}</span>
                </div>
                <div class="info-item">
                    <span class="label">Player:</span>
                    <span class="value">{$currentPlayer || 'Not set'}</span>
                </div>
                {#if $gameStats}
                    <div class="info-item">
                        <span class="label">Tick:</span>
                        <span class="value">{$gameStats.tick}</span>
                    </div>
                <div class="info-item">
                    <span class="label">Entities:</span>
                    <span class="value">{$gameStats ? $gameStats.totalEntities : entities.length}</span>
                </div>
                <div class="info-item">
                    <span class="label">Players:</span>
                    <span class="value">{$gameStats ? $gameStats.players : entities.filter(e => e.type === 'player').length}</span>
                </div>
                <div class="info-item">
                    <span class="label">Pickups:</span>
                    <span class="value">{entities.filter(e => e.type === 'pickup').length}</span>
                </div>
                <div class="info-item">
                    <span class="label">Enemies:</span>
                    <span class="value">{entities.filter(e => e.type === 'enemy').length}</span>
                </div>
                {/if}
            </div>

            <div class="debug-info">
                <h3>🔧 Debug</h3>
                <div class="debug-item">
                    <span>Mouse: ({Math.round(mouseX)}, {Math.round(mouseY)})</span>
                </div>
                <div class="debug-item">
                    <span>Camera: ({Math.round(cameraX)}, {Math.round(cameraY)})</span>
                </div>
                <div class="debug-item">
                    <span>Keys: {Object.keys(keys).filter(k => keys[k]).join(', ') || 'None'}</span>
                </div>
            </div>
        </div>
    </div>

    <div class="game-footer">
        <a href="/" class="back-link">← Back to Home</a>
        <a href="/rooms" class="rooms-link">🏠 Browse Rooms</a>
    </div>

    <!-- Network Metrics Overlay -->
    <NetworkMetrics />
</div>

<style>
    .game-container {
        min-height: 100vh;
        background: #0a0e1a;
        color: white;
        font-family: 'Segoe UI', system-ui, sans-serif;
    }

    .game-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem 2rem;
        background: rgba(0, 0, 0, 0.3);
        border-bottom: 1px solid #1a1f2e;
    }

    .game-header h1 {
        margin: 0;
        color: #4a9eff;
        font-size: 1.5rem;
    }

    .game-controls {
        display: flex;
        gap: 1rem;
    }

    .control-btn {
        background: #4a9eff;
        color: white;
        border: none;
        padding: 0.5rem 1rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
        transition: background 0.2s;
    }

    .control-btn:hover:not(:disabled) {
        background: #3a8eef;
    }

    .control-btn:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .game-content {
        display: flex;
        padding: 1rem;
        gap: 1rem;
        min-height: calc(100vh - 120px);
    }

    .game-canvas-container {
        flex: 1;
        position: relative;
        background: #000;
        border-radius: 8px;
        overflow: hidden;
        border: 2px solid #1a1f2e;
    }

    .game-canvas {
        display: block;
        background: #0a0e1a;
        cursor: crosshair;
    }

    .connection-error {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background: rgba(255, 71, 87, 0.9);
        color: white;
        padding: 1rem 2rem;
        border-radius: 8px;
        text-align: center;
        z-index: 1000;
    }

    .connection-error button {
        background: white;
        color: #ff4757;
        border: none;
        padding: 0.5rem 1rem;
        border-radius: 4px;
        cursor: pointer;
        margin-top: 0.5rem;
    }

    .game-sidebar {
        width: 250px;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .game-info, .debug-info {
        background: rgba(255, 255, 255, 0.05);
        border-radius: 8px;
        padding: 1rem;
        border: 1px solid #1a1f2e;
    }

    .game-info h3, .debug-info h3 {
        margin: 0 0 1rem 0;
        color: #4a9eff;
        font-size: 1rem;
    }

    .info-item {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.5rem;
        font-size: 0.9rem;
    }

    .info-item .label {
        color: #a0a0a0;
    }

    .info-item .value {
        color: #ffffff;
        font-weight: 600;
    }

    .debug-item {
        font-family: monospace;
        font-size: 0.8rem;
        color: #888;
        margin-bottom: 0.25rem;
        word-break: break-all;
    }

    .game-footer {
        display: flex;
        justify-content: center;
        gap: 2rem;
        padding: 1rem;
        background: rgba(0, 0, 0, 0.3);
        border-top: 1px solid #1a1f2e;
    }

    .back-link, .rooms-link {
        color: #4a9eff;
        text-decoration: none;
        padding: 0.5rem 1rem;
        border-radius: 6px;
        border: 1px solid #4a9eff;
        transition: all 0.2s;
    }

    .back-link:hover, .rooms-link:hover {
        background: #4a9eff;
        color: white;
    }

    /* Mobile responsiveness */
    @media (max-width: 768px) {
        .game-header {
            flex-direction: column;
            gap: 1rem;
            text-align: center;
        }

        .game-content {
            flex-direction: column;
        }

        .game-sidebar {
            width: 100%;
        }

        .game-footer {
            flex-direction: column;
            gap: 1rem;
            text-align: center;
        }
    }

    /* Canvas focus styles */
    .game-canvas:focus {
        outline: none;
        border-color: #4a9eff;
    }

    /* Scrollbar styling for debug info */
    .debug-info {
        max-height: 300px;
        overflow-y: auto;
    }

    .debug-info::-webkit-scrollbar {
        width: 4px;
    }

    .debug-info::-webkit-scrollbar-track {
        background: rgba(255, 255, 255, 0.1);
    }

    .debug-info::-webkit-scrollbar-thumb {
        background: #4a9eff;
        border-radius: 2px;
    }
</style>

