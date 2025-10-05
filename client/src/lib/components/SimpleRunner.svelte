<script>
    import { onMount, onDestroy } from 'svelte';

    // Simple variable declarations - let Svelte infer types
    let canvas = null;
    let ctx = null;
    let animationId = null;

    // Simple game state
    let isRunning = false;
    let playerX = 400;
    let playerY = 300;
    let score = 0;
    let speed = 2;

    onMount(() => {
        // Canvas will be bound in the template, so we need to wait for it
        setTimeout(() => {
            if (canvas) {
                ctx = canvas.getContext('2d');
                if (ctx) {
                    startGame();
                }
            }
        }, 100);
    });

    onDestroy(() => {
        if (animationId) {
            cancelAnimationFrame(animationId);
        }
    });

    function startGame() {
        isRunning = true;

        function gameLoop() {
            if (!isRunning || !ctx || !canvas) return;

            // Clear canvas
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Update game state
            playerX += speed;
            score = Math.floor(playerX / 10);

            // Draw background
            ctx.fillStyle = '#87CEEB';
            ctx.fillRect(0, 0, canvas.width, canvas.height);

            // Draw player (simple circle)
            ctx.fillStyle = '#4a9eff';
            ctx.beginPath();
            ctx.arc(playerX % canvas.width, playerY, 20, 0, Math.PI * 2);
            ctx.fill();

            // Draw score
            ctx.fillStyle = '#333';
            ctx.font = '24px Arial';
            ctx.fillText(`Score: ${score}`, 10, 30);

            animationId = requestAnimationFrame(gameLoop);
        }

        gameLoop();
    }

    function handleKeyDown(event) {
        if (event.code === 'Space') {
            playerY = Math.max(50, playerY - 50);
            setTimeout(() => {
                playerY = Math.min(550, playerY + 50);
            }, 300);
        }
    }

    // Event listeners
    onMount(() => {
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    });
</script>

<svelte:head>
    <title>Simple Endless Runner</title>
</svelte:head>

<div class="game-container">
    <div class="game-header">
        <h1>🏃 Simple Endless Runner (Test)</h1>
        <p>Press SPACE to jump!</p>
    </div>

    <div class="game-canvas-container">
        <canvas
            bind:this={canvas}
            width="800"
            height="600"
            class="game-canvas"
        ></canvas>
    </div>
</div>

<style>
    .game-container {
        min-height: 100vh;
        background: #0a0e1a;
        color: white;
        font-family: 'Segoe UI', system-ui, sans-serif;
    }

    .game-header {
        text-align: center;
        padding: 2rem;
        background: rgba(0, 0, 0, 0.6);
        border-bottom: 3px solid #4a9eff;
    }

    .game-header h1 {
        margin: 0 0 1rem 0;
        color: #4a9eff;
        font-size: 2.5rem;
    }

    .game-canvas-container {
        display: flex;
        justify-content: center;
        align-items: center;
        padding: 2rem;
    }

    .game-canvas {
        border: 3px solid #4a9eff;
        border-radius: 8px;
        background: #fff;
        box-shadow: 0 0 20px rgba(74, 158, 255, 0.3);
    }
</style>
