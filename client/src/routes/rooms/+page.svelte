<script lang="ts">
  import RoomBrowser from '$lib/components/RoomBrowser.svelte';
  import { currentPlayer } from '$lib/stores/game';

  // SvelteKit route props
  export let data;

  let currentPlayerId = null;

  // Subscribe to current player
  currentPlayer.subscribe(value => {
    currentPlayerId = value;
  });

  function setPlayerName() {
    const name = prompt('Enter your player name:');
    if (name && name.trim()) {
      currentPlayer.set(name.trim());
    }
  }
</script>

<svelte:head>
  <title>GameV1 - Room Browser</title>
</svelte:head>

<div class="rooms-page">
  <div class="header">
    <h1>🎮 GameV1 Room Browser</h1>

    <div class="player-info">
      {#if currentPlayerId}
        <span class="player-name">Player: {currentPlayerId}</span>
        <button class="change-name-btn" on:click={setPlayerName}>Change Name</button>
      {:else}
        <button class="set-name-btn" on:click={setPlayerName}>Set Player Name</button>
      {/if}
    </div>
  </div>

  {#if currentPlayerId}
    <RoomBrowser />
  {:else}
    <div class="no-player-state">
      <h2>Welcome to GameV1!</h2>
      <p>Please set your player name to browse and join game rooms.</p>
      <button class="primary-btn" on:click={setPlayerName}>Set Player Name</button>
    </div>
  {/if}

  <div class="footer">
    <a href="/">← Back to Home</a>
  </div>
</div>

<style>
  .rooms-page {
    min-height: 100vh;
    background: linear-gradient(135deg, #0a0e1a 0%, #1a1f2e 100%);
    color: white;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 2rem 2rem 1rem 2rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .header h1 {
    margin: 0;
    color: #4a9eff;
  }

  .player-info {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .player-name {
    background: #2a2f3e;
    padding: 0.5rem 1rem;
    border-radius: 20px;
    border: 1px solid #4a9eff;
    color: #4a9eff;
  }

  .change-name-btn, .set-name-btn, .primary-btn {
    background: #4a9eff;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }

  .change-name-btn:hover, .set-name-btn:hover, .primary-btn:hover {
    background: #3a8eef;
  }

  .no-player-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 60vh;
    text-align: center;
    padding: 2rem;
  }

  .no-player-state h2 {
    color: #4a9eff;
    margin-bottom: 1rem;
  }

  .no-player-state p {
    color: #a0a0a0;
    margin-bottom: 2rem;
    max-width: 400px;
  }

  .footer {
    text-align: center;
    padding: 2rem;
    border-top: 1px solid #3a3f4b;
    margin-top: 2rem;
  }

  .footer a {
    color: #4a9eff;
    text-decoration: none;
    font-weight: 600;
  }

  .footer a:hover {
    color: #6bb6ff;
  }

  @media (max-width: 768px) {
    .header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .player-info {
      flex-direction: column;
      gap: 0.5rem;
    }
  }
</style>
