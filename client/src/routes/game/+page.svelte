<script lang="ts">
    // 3D Endless Runner Game - The complete experience
    import EndlessRunner3D from '$lib/components/EndlessRunner3D.svelte';
    import { gameService, gameActions, isConnected, connectionError, currentPlayer } from '$lib/stores/game';
    import { onMount, onDestroy } from 'svelte';

    let connected = false;
    let error = null;
    let playerId = null;

    // Subscribe to stores
    isConnected.subscribe(value => connected = value);
    connectionError.subscribe(value => error = value);
    currentPlayer.subscribe(value => playerId = value);

    // Initialize connection when component mounts
    onMount(async () => {
        console.log('🎮 Game page loaded - running in single-player mode for now');
        // TODO: Re-enable multiplayer when connection issues are resolved
        /*
        // Generate a random player ID for this session
        const newPlayerId = `player_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

        try {
            console.log('🔄 Initializing multiplayer connection...');
            // Initialize connection to gateway
            await gameService.initializeGrpc();

            // Try to join the default room
            const success = await gameService.joinGame('default_room', newPlayerId);

            if (success) {
                console.log('✅ Successfully joined multiplayer game');
            } else {
                console.warn('⚠️ Could not join multiplayer game, running in single-player mode');
            }
        } catch (err) {
            console.error('❌ Failed to initialize multiplayer:', err);
            console.log('🔄 Running in single-player mode');
        }
        */
    });

    // Listen for game errors from EndlessRunner3D component
    onMount(() => {
        const handleGameError = (event) => {
            console.error('🎮 Game Error:', event.detail.message);
            error = event.detail.message;
        };

        window.addEventListener('gameError', handleGameError);

        return () => {
            window.removeEventListener('gameError', handleGameError);
        };
    });

    // Cleanup when component unmounts
    onDestroy(() => {
        // gameService.leaveGame(); // Commented out for single-player mode
    });
</script>

<svelte:head>
    <title>🎮 3D Endless Runner - GameV1</title>
</svelte:head>

<div class="game-container">
    <!-- Multiplayer status temporarily disabled for single-player mode -->
    <!-- {#if error}
        <div class="connection-error">
            <p>⚠️ {error}</p>
            <button on:click={() => gameService.initializeGrpc()}>
                Retry Connection
            </button>
        </div>
    {/if}

    {#if connected}
        <div class="connection-status connected">
            🟢 Connected to multiplayer server
        </div>
    {:else}
        <div class="connection-status disconnected">
            🟡 Running in single-player mode
        </div>
    {/if} -->

    <EndlessRunner3D />
</div>

<style>
    .game-container {
        position: relative;
        width: 100%;
        height: 100vh;
        background: #000;
    }
</style>