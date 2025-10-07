<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { gameState, currentPlayer, isConnected, connectionError, gameActions } from '$lib/stores/game';
  import NetworkMetrics from '$lib/components/NetworkMetrics.svelte';

  // Spectator state
  let canvas;
  let ctx;
  let animationId;
  let isRunning = false;

  // Spectator camera settings
  let cameraMode = 'free'; // 'free', 'follow', 'overview', 'fixed'
  let targetPlayerId = null;
  let cameraX = 0;
  let cameraY = 0;
  let cameraZ = 10; // Spectator camera height
  let cameraYaw = 0;
  let cameraPitch = -0.5; // Looking down slightly
  let cameraDistance = 15;

  // Game world properties
  const CANVAS_WIDTH = 800;
  const CANVAS_HEIGHT = 600;
  const WORLD_SIZE = 1000;

  // Game entities (from game state store)
  let entities = [];
  let players = [];

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
          size = 20;
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
          z: entity.transform.position[2],
          type,
          color,
          size
        };
      });

      // Extract players for follow mode
      players = entities.filter(e => e.type === 'player');
    }
  });

  onMount(() => {
    initializeCanvas();
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

  function setupEventListeners() {
    // Keyboard controls for spectator camera
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    // Mouse controls for camera rotation
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('wheel', handleMouseWheel);
  }

  function handleKeyDown(event) {
    const speed = 2.0;

    switch (event.code) {
      case 'KeyW':
        if (cameraMode === 'free') cameraZ += speed;
        break;
      case 'KeyS':
        if (cameraMode === 'free') cameraZ -= speed;
        break;
      case 'KeyA':
        if (cameraMode === 'free') cameraX -= speed;
        break;
      case 'KeyD':
        if (cameraMode === 'free') cameraX += speed;
        break;
      case 'ArrowLeft':
        cameraYaw -= 0.1;
        break;
      case 'ArrowRight':
        cameraYaw += 0.1;
        break;
      case 'ArrowUp':
        cameraPitch = Math.max(-Math.PI/2 + 0.1, cameraPitch - 0.1);
        break;
      case 'ArrowDown':
        cameraPitch = Math.min(Math.PI/2 - 0.1, cameraPitch + 0.1);
        break;
      case 'Digit1':
        cameraMode = 'free';
        break;
      case 'Digit2':
        cameraMode = 'follow';
        if (players.length > 0) {
          targetPlayerId = players[0].id;
        }
        break;
      case 'Digit3':
        cameraMode = 'overview';
        cameraX = 0;
        cameraY = 0;
        cameraZ = 50;
        cameraYaw = 0;
        cameraPitch = -Math.PI/3;
        break;
      case 'Digit4':
        cameraMode = 'fixed';
        break;
    }
  }

  function handleKeyUp(event) {
    // Handle key releases if needed
  }

  function handleMouseMove(event) {
    if (event.buttons === 1) { // Left mouse button
      const sensitivity = 0.01;
      cameraYaw -= event.movementX * sensitivity;
      cameraPitch = Math.max(-Math.PI/2 + 0.1, Math.min(Math.PI/2 - 0.1, cameraPitch - event.movementY * sensitivity));
    }
  }

  function handleMouseWheel(event) {
    event.preventDefault();
    if (cameraMode === 'free' || cameraMode === 'follow') {
      cameraDistance = Math.max(5, Math.min(50, cameraDistance + event.deltaY * 0.1));
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
    // Update camera based on mode
    switch (cameraMode) {
      case 'follow':
        if (targetPlayerId) {
          const targetPlayer = entities.find(e => e.id === targetPlayerId);
          if (targetPlayer) {
            // Calculate camera position behind and above the target
            const targetX = targetPlayer.x;
            const targetY = targetPlayer.y;

            cameraX = targetX - Math.sin(cameraYaw) * cameraDistance;
            cameraY = targetY - Math.cos(cameraYaw) * cameraDistance;
            cameraZ = targetY + cameraDistance * 0.5; // Look down at target
          }
        }
        break;

      case 'overview':
        // Camera is already positioned, no updates needed
        break;

      case 'fixed':
        // Fixed camera position, no updates needed
        break;

      case 'free':
      default:
        // Free camera - already handled in keyboard input
        break;
    }
  }

  function render() {
    if (!ctx) return;

    // Clear canvas
    ctx.fillStyle = '#0a0e1a';
    ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    // Save context for camera transform
    ctx.save();

    // Apply camera transformation (isometric projection for spectator view)
    const screenX = CANVAS_WIDTH / 2;
    const screenY = CANVAS_HEIGHT / 2;

    // Simple isometric projection
    ctx.translate(screenX, screenY);
    ctx.scale(1, 0.5); // Flatten Y axis for isometric effect

    // Draw world bounds
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 2;
    ctx.strokeRect(-WORLD_SIZE/2, -WORLD_SIZE/2, WORLD_SIZE, WORLD_SIZE);

    // Draw grid for reference
    ctx.strokeStyle = '#2a2f3e';
    ctx.lineWidth = 1;
    for (let x = -WORLD_SIZE/2; x <= WORLD_SIZE/2; x += 100) {
      ctx.beginPath();
      ctx.moveTo(x, -WORLD_SIZE/2);
      ctx.lineTo(x, WORLD_SIZE/2);
      ctx.stroke();
    }
    for (let y = -WORLD_SIZE/2; y <= WORLD_SIZE/2; y += 100) {
      ctx.beginPath();
      ctx.moveTo(-WORLD_SIZE/2, y);
      ctx.lineTo(WORLD_SIZE/2, y);
      ctx.stroke();
    }

    // Draw entities with isometric projection
    entities.forEach(entity => {
      ctx.fillStyle = entity.color;

      // Convert 3D world coordinates to 2D screen coordinates (simple isometric)
      const isoX = entity.x - entity.z * 0.5;
      const isoY = entity.y + (entity.x + entity.z) * 0.25;

      switch (entity.type) {
        case 'player':
          // Draw player as circle
          ctx.beginPath();
          ctx.arc(isoX, isoY, entity.size * 0.8, 0, Math.PI * 2);
          ctx.fill();

          // Draw player indicator
          ctx.fillStyle = '#ffffff';
          ctx.font = '10px monospace';
          ctx.textAlign = 'center';
          ctx.fillText('P', isoX, isoY - entity.size - 15);
          break;

        case 'pickup':
          // Draw pickup as small glowing circle
          ctx.shadowColor = entity.color;
          ctx.shadowBlur = 8;
          ctx.beginPath();
          ctx.arc(isoX, isoY, entity.size, 0, Math.PI * 2);
          ctx.fill();
          ctx.shadowBlur = 0;
          break;

        case 'enemy':
          // Draw enemy as diamond
          ctx.beginPath();
          ctx.moveTo(isoX, isoY - entity.size);
          ctx.lineTo(isoX + entity.size, isoY);
          ctx.lineTo(isoX, isoY + entity.size);
          ctx.lineTo(isoX - entity.size, isoY);
          ctx.closePath();
          ctx.fill();
          break;

        case 'obstacle':
          // Draw obstacle as cube (represented as hexagon in isometric)
          ctx.beginPath();
          ctx.moveTo(isoX - entity.size * 0.5, isoY - entity.size * 0.25);
          ctx.lineTo(isoX + entity.size * 0.5, isoY - entity.size * 0.25);
          ctx.lineTo(isoX + entity.size, isoY);
          ctx.lineTo(isoX + entity.size * 0.5, isoY + entity.size * 0.25);
          ctx.lineTo(isoX - entity.size * 0.5, isoY + entity.size * 0.25);
          ctx.lineTo(isoX - entity.size, isoY);
          ctx.closePath();
          ctx.fill();
          break;
      }
    });

    // Restore context
    ctx.restore();

    // Draw UI overlay (HUD)
    drawHUD();
  }

  function drawHUD() {
    if (!ctx) return;

    // Semi-transparent overlay for UI
    ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
    ctx.fillRect(0, 0, CANVAS_WIDTH, 80);

    // Camera mode indicator
    ctx.fillStyle = '#ffffff';
    ctx.font = '16px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`Camera Mode: ${cameraMode.toUpperCase()}`, 20, 30);

    // Controls help
    ctx.font = '14px monospace';
    ctx.fillStyle = '#cccccc';
    ctx.fillText('1:Free 2:Follow 3:Overview 4:Fixed | Mouse:Rotate | WASD:Move (Free mode)', 20, 50);

    // Target player indicator (for follow mode)
    if (cameraMode === 'follow' && targetPlayerId) {
      ctx.fillStyle = '#4a9eff';
      ctx.fillText(`Following: Player ${targetPlayerId}`, 20, 70);
    }

    // Spectator info
    ctx.textAlign = 'right';
    ctx.fillStyle = '#9b59b6';
    ctx.fillText('SPECTATOR MODE', CANVAS_WIDTH - 20, 30);

    // Connection status
    ctx.fillStyle = $isConnected ? '#2ecc71' : '#e74c3c';
    ctx.fillText($isConnected ? 'Connected' : 'Disconnected', CANVAS_WIDTH - 20, 50);

    // Entity count
    ctx.fillStyle = '#ffffff';
    ctx.fillText(`Entities: ${entities.length}`, CANVAS_WIDTH - 20, 70);
  }

  function exitSpectatorMode() {
    // Leave spectator mode and return to room browser
    window.location.href = '/rooms';
  }
</script>

<svelte:head>
  <title>GameV1 - Spectator Mode</title>
</svelte:head>

<div class="spectator-container">
  <div class="spectator-header">
    <h1>Spectator Mode</h1>
    <div class="spectator-controls">
      <button on:click={exitSpectatorMode} class="exit-btn">
        Exit Spectator Mode
      </button>
    </div>
  </div>

  <div class="spectator-content">
    <div class="spectator-canvas-container">
      <canvas
        bind:this={canvas}
        class="spectator-canvas"
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
      ></canvas>

      {#if $connectionError}
        <div class="connection-error">
          <p>{$connectionError}</p>
          <button on:click={() => connectionError.set(null)}>Close</button>
        </div>
      {/if}
    </div>

    <div class="spectator-sidebar">
      <div class="camera-controls">
        <h3>📷 Camera Controls</h3>
        <div class="camera-mode-selector">
          <button
            class="mode-btn {cameraMode === 'free' ? 'active' : ''}"
            on:click={() => cameraMode = 'free'}
          >
            🌍 Free Camera
          </button>
          <button
            class="mode-btn {cameraMode === 'follow' ? 'active' : ''}"
            on:click={() => cameraMode = 'follow'}
          >
            Follow Player
          </button>
          <button
            class="mode-btn {cameraMode === 'overview' ? 'active' : ''}"
            on:click={() => cameraMode = 'overview'}
          >
            Overview
          </button>
          <button
            class="mode-btn {cameraMode === 'fixed' ? 'active' : ''}"
            on:click={() => cameraMode = 'fixed'}
          >
            📍 Fixed Position
          </button>
        </div>

        {#if cameraMode === 'follow'}
          <div class="player-selector">
            <label>Target Player:</label>
            <select bind:value={targetPlayerId}>
              <option value={null}>Select Player</option>
              {#each players as player}
                <option value={player.id}>Player {player.id}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>

      <div class="game-info">
        <h3>Game Info</h3>
        <div class="info-item">
          <span class="label">Status:</span>
          <span class="value">{$isConnected ? 'Connected' : 'Disconnected'}</span>
        </div>
        <div class="info-item">
          <span class="label">Mode:</span>
          <span class="value">{cameraMode}</span>
        </div>
        <div class="info-item">
          <span class="label">Entities:</span>
          <span class="value">{entities.length}</span>
        </div>
        <div class="info-item">
          <span class="label">Players:</span>
          <span class="value">{players.length}</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Network Metrics Overlay -->
  <NetworkMetrics />
</div>

<style>
  .spectator-container {
    min-height: 100vh;
    background: #0a0e1a;
    color: white;
    font-family: 'Segoe UI', system-ui, sans-serif;
  }

  .spectator-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 2rem;
    background: rgba(0, 0, 0, 0.3);
    border-bottom: 1px solid #1a1f2e;
  }

  .spectator-header h1 {
    margin: 0;
    color: #9b59b6;
    font-size: 1.5rem;
  }

  .exit-btn {
    background: #e74c3c;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }

  .exit-btn:hover {
    background: #c0392b;
  }

  .spectator-content {
    display: flex;
    padding: 1rem;
    gap: 1rem;
    min-height: calc(100vh - 80px);
  }

  .spectator-canvas-container {
    flex: 1;
    position: relative;
    background: #000;
    border-radius: 8px;
    overflow: hidden;
    border: 2px solid #1a1f2e;
  }

  .spectator-canvas {
    display: block;
    background: #0a0e1a;
    cursor: grab;
  }

  .spectator-canvas:active {
    cursor: grabbing;
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

  .spectator-sidebar {
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .camera-controls, .game-info {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1rem;
    border: 1px solid #1a1f2e;
  }

  .camera-controls h3, .game-info h3 {
    margin: 0 0 1rem 0;
    color: #9b59b6;
    font-size: 1rem;
  }

  .camera-mode-selector {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .mode-btn {
    background: #2a2f3e;
    color: white;
    border: 1px solid #3a3f4b;
    padding: 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    text-align: center;
    transition: all 0.2s;
  }

  .mode-btn:hover {
    background: #3a3f4b;
  }

  .mode-btn.active {
    background: #9b59b6;
    border-color: #9b59b6;
  }

  .player-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .player-selector label {
    font-size: 0.9rem;
    color: #a0a0a0;
  }

  .player-selector select {
    padding: 0.5rem;
    border: 1px solid #3a3f4b;
    border-radius: 4px;
    background: #2a2f3e;
    color: white;
    font-size: 0.9rem;
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

  @media (max-width: 768px) {
    .spectator-header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .spectator-content {
      flex-direction: column;
    }

    .spectator-sidebar {
      width: 100%;
    }

    .camera-mode-selector {
      grid-template-columns: 1fr;
    }
  }
</style>
