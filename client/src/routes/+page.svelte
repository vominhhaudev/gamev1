<script lang="ts">
  // Simple home page for debugging
  import Login from '$lib/components/Login.svelte';
  import ValidationErrors from '$lib/components/ValidationErrors.svelte';
  import { inputValidationErrors } from '$lib/stores/input';

  // SvelteKit route props

  let validationErrors = [];

  // Subscribe to input validation errors
  inputValidationErrors.subscribe(errors => {
    validationErrors = errors;
  });

  // Test function to simulate validation errors
  function testValidation() {
    // Simulate some validation errors
    validationErrors = [
      { field: 'movement[0]', message: 'Movement value cannot be NaN' },
      { field: 'player_id', message: 'Player ID contains invalid characters' }
    ];
    inputValidationErrors.set(validationErrors);
  }

  function clearValidation() {
    validationErrors = [];
    inputValidationErrors.set([]);
  }
</script>

<svelte:head>
  <title>Home - gamev1 client</title>
</svelte:head>

<section class="hero">
  <div class="hero-content">
    <h1 class="hero-title">🎮 GameV1 Client</h1>
    <p class="hero-subtitle">Multiplayer Gaming Platform</p>
    <p class="hero-description">Experience the next generation of web-based multiplayer gaming with real-time 3D graphics, endless runner gameplay, and advanced networking.</p>

    <!-- Main Game Links -->
    <div class="main-links">
      <a href="/game" class="primary-btn">🎮 Play 3D Endless Runner</a>
      <a href="/game-test" class="secondary-btn">🧪 Test Game</a>
      <a href="/rooms" class="secondary-btn">🎯 Browse Rooms</a>
    </div>

    <!-- Quick Access Info -->
    <div class="access-info">
      <p><strong>🎮 Game URL:</strong> <code>http://localhost:5173/game</code></p>
      <p><strong>🎯 Controls:</strong> A/D or ←/→ to move lanes, SPACE to jump!</p>
      <p><strong>📊 Status:</strong> ✅ Full 3D Endless Runner working!</p>
      <p><strong>🎨 Graphics:</strong> Real-time 3D rendering with Three.js</p>
      <p><strong>🎪 Features:</strong> Procedurally generated track, obstacles, collectibles</p>
      <p><strong>⚠️ Pro Tip:</strong> Use <code>/game</code> for the complete 3D experience!</p>
    </div>

    <!-- Additional Links -->
    <div class="secondary-links">
      <a href="/net-test" class="tertiary-btn">🔧 Network Test</a>
      <a href="/spectator" class="tertiary-btn">👀 Spectator Mode</a>
    </div>

    <!-- Game Information -->
    <div class="game-info">
      <h3>🎮 Game Features</h3>
      <div class="feature-list">
        <p>✅ <strong>3D Graphics:</strong> Real-time rendering with Three.js</p>
        <p>✅ <strong>Endless Runner:</strong> Procedurally generated track</p>
        <p>✅ <strong>Multiple Lanes:</strong> Move left/right to avoid obstacles</p>
        <p>✅ <strong>Jump Mechanics:</strong> Spacebar to jump over barriers</p>
        <p>✅ <strong>Collectibles:</strong> Gather coins and power-ups</p>
        <p>✅ <strong>Dynamic Difficulty:</strong> Speed increases over time</p>
      </div>
    </div>
  </div>

  <!-- Login Section -->
  <div class="login-section">
    <Login />
  </div>

  <!-- Development Tools (Collapsible) -->
  <details class="dev-tools">
    <summary>🛠️ Development Tools</summary>
    <div class="dev-tools-content">
      <h3>🛡️ Input Validation System</h3>
      <p>Testing the new input validation system:</p>
      <div class="validation-controls">
        <button on:click={testValidation} class="danger-btn">Test Validation Errors</button>
        <button on:click={clearValidation} class="success-btn">Clear Errors</button>
      </div>
      <ValidationErrors errors={validationErrors} title="Demo Validation Errors" />
    </div>
  </details>
</section>

<style>
  .hero {
    min-height: 100vh;
    background: linear-gradient(135deg, #0a0e1a 0%, #1a1f2e 50%, #2a3441 100%);
    color: white;
    font-family: 'Segoe UI', system-ui, sans-serif;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 2rem;
  }

  .hero-content {
    max-width: 800px;
    margin-bottom: 3rem;
  }

  .hero-title {
    font-size: 3.5rem;
    margin: 0 0 1rem 0;
    background: linear-gradient(135deg, #4a9eff, #00d4ff);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    text-shadow: 0 0 30px rgba(74, 158, 255, 0.5);
    animation: glow 3s ease-in-out infinite alternate;
  }

  .hero-subtitle {
    font-size: 1.5rem;
    margin: 0 0 1.5rem 0;
    color: #a0a0a0;
    font-weight: 300;
  }

  .hero-description {
    font-size: 1.1rem;
    line-height: 1.6;
    margin: 0 0 3rem 0;
    color: #cccccc;
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
  }

  .main-links {
    margin-bottom: 2rem;
  }

  .primary-btn {
    display: inline-block;
    padding: 1.2rem 3rem;
    background: linear-gradient(135deg, #4a9eff, #3a8eef);
    color: white;
    text-decoration: none;
    border-radius: 12px;
    font-size: 1.2rem;
    font-weight: 600;
    margin: 0 1rem;
    transition: all 0.3s ease;
    box-shadow: 0 8px 25px rgba(74, 158, 255, 0.3);
    border: 2px solid transparent;
  }

  .primary-btn:hover {
    transform: translateY(-3px);
    box-shadow: 0 12px 35px rgba(74, 158, 255, 0.4);
    border-color: rgba(74, 158, 255, 0.5);
  }

  .secondary-btn {
    display: inline-block;
    padding: 1rem 2.5rem;
    background: linear-gradient(135deg, #ff6b6b, #ee5a52);
    color: white;
    text-decoration: none;
    border-radius: 12px;
    font-size: 1.1rem;
    font-weight: 500;
    margin: 0 0.5rem;
    transition: all 0.3s ease;
    box-shadow: 0 6px 20px rgba(255, 107, 107, 0.3);
  }

  .secondary-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(255, 107, 107, 0.4);
  }

  .secondary-links {
    margin-top: 1rem;
  }

  .tertiary-btn {
    display: inline-block;
    padding: 0.8rem 1.5rem;
    background: rgba(255, 255, 255, 0.1);
    color: white;
    text-decoration: none;
    border-radius: 8px;
    font-size: 0.9rem;
    margin: 0 0.5rem;
    transition: all 0.3s ease;
    border: 1px solid rgba(255, 255, 255, 0.2);
    backdrop-filter: blur(10px);
  }

  .tertiary-btn:hover {
    background: rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
  }

  .login-section {
    margin: 3rem 0;
    width: 100%;
    max-width: 400px;
  }

  .access-info {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.5rem;
    margin: 2rem 0;
    border: 1px solid rgba(74, 158, 255, 0.3);
    backdrop-filter: blur(10px);
  }

  .access-info p {
    margin: 0.5rem 0;
    font-size: 0.9rem;
    color: #cccccc;
  }

  .access-info code {
    background: rgba(0, 0, 0, 0.3);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    color: #4a9eff;
    border: 1px solid rgba(74, 158, 255, 0.3);
  }


  .game-info {
    background: rgba(74, 158, 255, 0.1);
    border-radius: 12px;
    padding: 1.5rem;
    margin: 2rem 0;
    border: 1px solid rgba(74, 158, 255, 0.3);
    backdrop-filter: blur(10px);
  }

  .game-info h3 {
    margin: 0 0 1rem 0;
    color: #4a9eff;
    font-size: 1.2rem;
  }

  .feature-list p {
    margin: 0.5rem 0;
    font-size: 0.9rem;
    color: #cccccc;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .feature-list strong {
    color: #4a9eff;
  }

  .dev-tools {
    margin-top: 3rem;
    width: 100%;
    max-width: 600px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    backdrop-filter: blur(10px);
  }

  .dev-tools summary {
    padding: 1rem 1.5rem;
    cursor: pointer;
    font-weight: 600;
    color: #4a9eff;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .dev-tools summary:hover {
    background: rgba(74, 158, 255, 0.1);
  }

  .dev-tools-content {
    padding: 1.5rem;
    text-align: left;
  }

  .dev-tools-content h3 {
    margin: 0 0 1rem 0;
    color: #4a9eff;
  }

  .validation-controls {
    margin: 1rem 0;
  }

  .danger-btn {
    background: #ff6b6b;
    color: white;
    border: none;
    padding: 0.6rem 1.2rem;
    border-radius: 6px;
    cursor: pointer;
    margin-right: 0.5rem;
    transition: background 0.3s;
  }

  .danger-btn:hover {
    background: #ee5a52;
  }

  .success-btn {
    background: #51cf66;
    color: white;
    border: none;
    padding: 0.6rem 1.2rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.3s;
  }

  .success-btn:hover {
    background: #40c057;
  }

  @keyframes glow {
    from {
      filter: brightness(1);
    }
    to {
      filter: brightness(1.2);
    }
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .hero-title {
      font-size: 2.5rem;
    }

    .hero-description {
      font-size: 1rem;
    }

    .main-links {
      flex-direction: column;
      align-items: center;
    }

    .primary-btn, .secondary-btn {
      margin: 0.5rem 0;
      width: 250px;
      text-align: center;
    }

    .secondary-links {
      flex-direction: column;
      align-items: center;
    }

    .tertiary-btn {
      margin: 0.3rem 0;
      width: 200px;
      text-align: center;
    }
  }
</style>
