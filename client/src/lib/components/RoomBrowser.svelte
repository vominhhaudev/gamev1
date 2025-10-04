<script lang="ts">
  import { onMount } from 'svelte';
  import { roomList, isLoadingRooms, roomError, roomActions } from '$lib/stores/room';
  import { currentPlayer } from '$lib/stores/game';
    // Types are imported but not used with type annotations in Svelte script
  import { roomUtils } from '$lib/stores/room';

  let rooms = [];
  let filteredRooms = [];
  let loading = false;
  let error = null;
  let showCreateRoom = false;
  let currentPlayerId = null;

  // Filter state
  let filter = {
    gameMode: undefined,
    hasPassword: undefined,
    minPlayers: undefined,
    maxPlayers: undefined,
    state: 'waiting',
  };

  // Create room form state
  let newRoomName = '';
  let newRoomGameMode = 'deathmatch';
  let newRoomMaxPlayers = 8;
  let newRoomHasPassword = false;
  let newRoomPassword = '';

  // Subscribe to stores
  roomList.subscribe(value => {
    rooms = value;
    applyFilter();
  });

  isLoadingRooms.subscribe(value => loading = value);
  roomError.subscribe(value => error = value);
  currentPlayer.subscribe(value => currentPlayerId = value);

  onMount(() => {
    loadRooms();
  });

  function loadRooms() {
    roomActions.listRooms(filter);
  }

  function applyFilter() {
    filteredRooms = rooms.filter(room => {
      if (filter.gameMode && room.gameMode !== filter.gameMode) return false;
      if (filter.hasPassword !== undefined && room.hasPassword !== filter.hasPassword) return false;
      if (filter.minPlayers && room.playerCount < filter.minPlayers) return false;
      if (filter.maxPlayers && room.playerCount > filter.maxPlayers) return false;
      if (filter.state && room.state !== filter.state) return false;
      return true;
    });
  }

  function handleFilterChange() {
    applyFilter();
  }

  function resetFilter() {
    filter = {
      gameMode: undefined,
      hasPassword: undefined,
      minPlayers: undefined,
      maxPlayers: undefined,
      state: 'waiting',
    };
    applyFilter();
  }

  async function handleJoinRoom(room) {
    if (!currentPlayerId) {
      error = 'Please set your player name first';
      return;
    }

    const result = await roomActions.joinRoom({
      roomId: room.id,
      playerId: currentPlayerId,
      playerName: `Player_${currentPlayerId.slice(0, 8)}`,
    });

    if (result.success) {
      // Room joined successfully, switch to game view
      // This would typically navigate to a game page
      console.log('Joined room:', room.name);
    } else {
      error = result.error || 'Failed to join room';
    }
  }

  async function handleJoinRoomAsSpectator(room) {
    if (!currentPlayerId) {
      error = 'Please set your player name first';
      return;
    }

    const result = await roomActions.joinRoomAsSpectator({
      roomId: room.id,
      spectatorId: currentPlayerId,
      spectatorName: `Spectator_${currentPlayerId.slice(0, 8)}`,
    });

    if (result.success) {
      // Spectator joined successfully, navigate to spectator view
      window.location.href = `/spectator?spectatorId=${encodeURIComponent(currentPlayerId)}&roomId=${encodeURIComponent(room.id)}`;
      console.log('Joined room as spectator:', room.name);
    } else {
      error = result.error || 'Failed to join as spectator';
    }
  }

  async function handleCreateRoom() {
    if (!currentPlayerId) {
      error = 'Please set your player name first';
      return;
    }

    const roomNameError = roomUtils.validateRoomName(newRoomName);
    if (roomNameError) {
      error = roomNameError;
      return;
    }

    const result = await roomActions.createRoom({
      roomName: newRoomName,
      hostId: currentPlayerId,
      hostName: `Host_${currentPlayerId.slice(0, 8)}`,
      settings: {
        maxPlayers: newRoomMaxPlayers,
        gameMode: newRoomGameMode,
        mapName: 'default_map',
        timeLimit: 300,
        hasPassword: newRoomHasPassword,
        isPrivate: false,
        allowSpectators: true,
        autoStart: true,
        minPlayersToStart: 2,
      },
    });

    if (result.success) {
      showCreateRoom = false;
      newRoomName = '';
      newRoomGameMode = GameMode.Deathmatch;
      newRoomMaxPlayers = 8;
      newRoomHasPassword = false;
      newRoomPassword = '';
      loadRooms(); // Refresh room list
    } else {
      error = result.error || 'Failed to create room';
    }
  }

  function formatTimeAgo(seconds) {
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    return `${Math.floor(seconds / 3600)}h ago`;
  }
</script>

<div class="room-browser">
  <div class="header">
    <h2>🎮 Game Rooms</h2>
    <button class="create-btn" on:click={() => showCreateRoom = !showCreateRoom}>
      {showCreateRoom ? 'Cancel' : 'Create Room'}
    </button>
  </div>

  {#if error}
    <div class="error-message">
      {error}
      <button on:click={() => error = null}>×</button>
    </div>
  {/if}

  <!-- Filter Controls -->
  <div class="filters">
    <div class="filter-group">
      <label>Game Mode:</label>
      <select bind:value={filter.gameMode} on:change={handleFilterChange}>
        <option value={undefined}>All</option>
        <option value="deathmatch">Deathmatch</option>
        <option value="team_deathmatch">Team Deathmatch</option>
        <option value="capture_the_flag">Capture the Flag</option>
        <option value="king_of_the_hill">King of the Hill</option>
      </select>
    </div>

    <div class="filter-group">
      <label>Password:</label>
      <select bind:value={filter.hasPassword} on:change={handleFilterChange}>
        <option value={undefined}>All</option>
        <option value={true}>Protected</option>
        <option value={false}>Open</option>
      </select>
    </div>

    <div class="filter-group">
      <label>Min Players:</label>
      <input
        type="number"
        min="1"
        max="16"
        bind:value={filter.minPlayers}
        on:input={handleFilterChange}
        placeholder="0"
      />
    </div>

    <div class="filter-group">
      <label>Max Players:</label>
      <input
        type="number"
        min="2"
        max="16"
        bind:value={filter.maxPlayers}
        on:input={handleFilterChange}
        placeholder="16"
      />
    </div>

    <button class="reset-btn" on:click={resetFilter}>Reset</button>
  </div>

  <!-- Create Room Form -->
  {#if showCreateRoom}
    <div class="create-room-form">
      <h3>Create New Room</h3>
      <div class="form-grid">
        <div class="form-group">
          <label>Room Name:</label>
          <input
            type="text"
            bind:value={newRoomName}
            placeholder="Enter room name"
            maxlength="50"
          />
        </div>

        <div class="form-group">
          <label>Game Mode:</label>
          <select bind:value={newRoomGameMode}>
            <option value="deathmatch">Deathmatch</option>
            <option value="team_deathmatch">Team Deathmatch</option>
            <option value="capture_the_flag">Capture the Flag</option>
            <option value="king_of_the_hill">King of the Hill</option>
          </select>
        </div>

        <div class="form-group">
          <label>Max Players:</label>
          <input
            type="number"
            min="2"
            max="16"
            bind:value={newRoomMaxPlayers}
          />
        </div>

        <div class="form-group">
          <label>
            <input type="checkbox" bind:checked={newRoomHasPassword} />
            Password Protected
          </label>
        </div>
      </div>

      <div class="form-actions">
        <button class="create-btn" on:click={handleCreateRoom} disabled={loading}>
          {loading ? 'Creating...' : 'Create Room'}
        </button>
        <button class="cancel-btn" on:click={() => showCreateRoom = false}>Cancel</button>
      </div>
    </div>
  {/if}

  <!-- Room List -->
  <div class="room-list">
    {#if loading}
      <div class="loading">Loading rooms...</div>
    {:else if filteredRooms.length === 0}
      <div class="empty-state">
        <p>No rooms found matching your criteria.</p>
        <button on:click={loadRooms}>Refresh</button>
      </div>
    {:else}
      <div class="rooms-grid">
        {#each filteredRooms as room (room.id)}
          <div class="room-card">
            <div class="room-header">
              <h3>{room.name}</h3>
              <span class="room-state {room.state}">{roomUtils.formatRoomState(room.state)}</span>
            </div>

            <div class="room-info">
              <div class="info-row">
                <span>Mode:</span>
                <span>{roomUtils.formatGameMode(room.gameMode)}</span>
              </div>
              <div class="info-row">
                <span>Players:</span>
                <span>{room.playerCount}/{room.maxPlayers}</span>
              </div>
              <div class="info-row">
                <span>Created:</span>
                <span>{formatTimeAgo(room.createdAt)}</span>
              </div>
              {#if room.hasPassword}
                <div class="info-row password-indicator">
                  🔒 Password Protected
                </div>
              {/if}
            </div>

            <div class="room-actions">
              <button
                class="join-btn"
                on:click={() => handleJoinRoom(room)}
                disabled={room.state !== 'waiting' || room.playerCount >= room.maxPlayers}
              >
                {room.state !== 'waiting' ? 'Game Started' :
                 room.playerCount >= room.maxPlayers ? 'Full' : 'Join as Player'}
              </button>
              {#if room.allowSpectators}
                <button
                  class="spectator-btn"
                  on:click={() => handleJoinRoomAsSpectator(room)}
                  disabled={room.state === 'finished'}
                >
                  👁️ Spectate
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .room-browser {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
    background: #0a0e1a;
    color: white;
    min-height: 100vh;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
  }

  .header h2 {
    margin: 0;
    color: #4a9eff;
  }

  .create-btn, .reset-btn {
    background: #4a9eff;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }

  .create-btn:hover:not(:disabled) {
    background: #3a8eef;
  }

  .create-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-message {
    background: #ff4757;
    color: white;
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .error-message button {
    background: none;
    border: none;
    color: white;
    font-size: 1.2rem;
    cursor: pointer;
  }

  .filters {
    display: flex;
    gap: 1rem;
    margin-bottom: 2rem;
    flex-wrap: wrap;
    align-items: center;
    padding: 1rem;
    background: #1a1f2e;
    border-radius: 8px;
  }

  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .filter-group label {
    font-size: 0.9rem;
    color: #a0a0a0;
  }

  .filter-group select,
  .filter-group input {
    padding: 0.5rem;
    border: 1px solid #3a3f4b;
    border-radius: 4px;
    background: #2a2f3e;
    color: white;
    font-size: 0.9rem;
  }

  .reset-btn {
    background: #666;
    margin-left: auto;
  }

  .create-room-form {
    background: #1a1f2e;
    padding: 2rem;
    border-radius: 12px;
    margin-bottom: 2rem;
    border: 1px solid #3a3f4b;
  }

  .create-room-form h3 {
    margin-top: 0;
    color: #4a9eff;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group label {
    font-size: 0.9rem;
    color: #a0a0a0;
  }

  .form-group input,
  .form-group select {
    padding: 0.75rem;
    border: 1px solid #3a3f4b;
    border-radius: 6px;
    background: #2a2f3e;
    color: white;
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
  }

  .cancel-btn {
    background: #666;
  }

  .loading {
    text-align: center;
    padding: 3rem;
    color: #a0a0a0;
  }

  .empty-state {
    text-align: center;
    padding: 3rem;
    color: #a0a0a0;
  }

  .empty-state button {
    background: #4a9eff;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    margin-top: 1rem;
  }

  .rooms-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1.5rem;
  }

  .room-card {
    background: #1a1f2e;
    border: 1px solid #3a3f4b;
    border-radius: 12px;
    padding: 1.5rem;
    transition: transform 0.2s, box-shadow 0.2s;
  }

  .room-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(74, 158, 255, 0.1);
  }

  .room-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .room-header h3 {
    margin: 0;
    color: #4a9eff;
  }

  .room-state {
    padding: 0.25rem 0.75rem;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .room-state.waiting {
    background: #2ecc71;
    color: white;
  }

  .room-state.playing {
    background: #e74c3c;
    color: white;
  }

  .room-state.finished {
    background: #95a5a6;
    color: white;
  }

  .room-info {
    margin-bottom: 1.5rem;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .info-row.password-indicator {
    color: #f39c12;
    font-weight: 600;
  }

  .room-actions {
    display: flex;
    justify-content: flex-end;
  }

  .join-btn {
    background: #2ecc71;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }

  .join-btn:hover:not(:disabled) {
    background: #27ae60;
  }

  .join-btn:disabled {
    background: #7f8c8d;
    cursor: not-allowed;
  }

  .spectator-btn {
    background: #9b59b6;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.9rem;
    transition: background 0.2s;
  }

  .spectator-btn:hover:not(:disabled) {
    background: #8e44ad;
  }

  .spectator-btn:disabled {
    background: #7f8c8d;
    cursor: not-allowed;
  }

  @media (max-width: 768px) {
    .room-browser {
      padding: 1rem;
    }

    .header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .filters {
      flex-direction: column;
      align-items: stretch;
    }

    .rooms-grid {
      grid-template-columns: 1fr;
    }

    .form-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
