<script lang="ts">
  import { onMount } from 'svelte';
  import SpectatorView from '$lib/components/SpectatorView.svelte';
  import { currentPlayer, isConnected } from '$lib/stores/game';

  // SvelteKit route props

  // Spectator state
  let spectatorId = null;
  let roomId = null;

  onMount(() => {
    // Get parameters from URL or navigation state
    const urlParams = new URLSearchParams(window.location.search);
    spectatorId = urlParams.get('spectatorId');
    roomId = urlParams.get('roomId');

    if (!spectatorId || !roomId) {
      // Redirect to rooms if no spectator parameters
      window.location.href = '/rooms';
      return;
    }

    // Set current player as spectator
    currentPlayer.set(spectatorId);

    console.log('Entering spectator mode for room:', roomId);
  });
</script>

<svelte:head>
  <title>GameV1 - Spectator Mode</title>
</svelte:head>

{#if spectatorId && roomId}
  <SpectatorView />
{:else}
  <div class="loading-container">
    <div class="loading">
      <p>Loading spectator mode...</p>
    </div>
  </div>
{/if}

<style>
  .loading-container {
    min-height: 100vh;
    background: #0a0e1a;
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .loading {
    text-align: center;
    padding: 2rem;
  }

  .loading p {
    color: #9b59b6;
    font-size: 1.2rem;
  }
</style>
