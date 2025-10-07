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

  // Gaming state for demo
  let playerLevel = 42;
  let experience = 7850;
  let nextLevelExp = 10000;
  let achievements = ['First Steps', 'Speed Demon', 'Marathon Runner'];

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
  <title>Home - ENEEGY</title>
</svelte:head>

<section class="hero">
  <div class="hero-content">
    <h1 class="hero-title">ENEEGY</h1>
    <p class="hero-subtitle">Dark Gaming Platform</p>
   <p class="hero-description"></p>

    <!-- Main Game Links -->
    <div class="main-links">
      <a href="/game" class="primary-btn">
        <span>GAME</span>
        <span>Play 3D Endless Runner</span>
      </a>
      <a href="/game-test" class="secondary-btn">
        <span>TEST</span>
        <span>Test Game</span>
      </a>
      <a href="/rooms" class="secondary-btn">
        <span>TARGET</span>
        <span>Browse Rooms</span>
      </a>
    </div>

    <!-- Quick Access Info - Có thể xóa nếu muốn giao diện sạch hơn -->
    <!-- <div class="access-info">
      <p><strong>Game URL:</strong> <code>http://localhost:5173/game</code></p>
      <p><strong>Controls:</strong> A/D or ←/→ to move lanes, SPACE to jump!</p>
      <p><strong>Status:</strong> Full 3D Endless Runner working!</p>
      <p><strong>Graphics:</strong> Real-time 3D rendering with Three.js</p>
      <p><strong>Features:</strong> Procedurally generated track, obstacles, collectibles</p>
      <p><strong>Pro Tip:</strong> Use <code>/game</code> for the complete 3D experience!</p>
    </div> -->

    <!-- Additional Links -->
    <div class="secondary-links">
      <a href="/net-test" class="tertiary-btn">
        <span>Network Test</span>
      </a>
      <a href="/spectator" class="tertiary-btn">
        <span>Spectator Mode</span>
      </a>
    </div>

    <!-- Game Information - Có thể xóa nếu muốn giao diện đơn giản hơn -->
    <!-- <div class="game-info">
      <h3>Game Features</h3>
      <div class="feature-list">
        <p><strong>3D Graphics:</strong> Real-time rendering with Three.js</p>
        <p><strong>Endless Runner:</strong> Procedurally generated track</p>
        <p><strong>Multiple Lanes:</strong> Move left/right to avoid obstacles</p>
        <p><strong>Jump Mechanics:</strong> Spacebar to jump over barriers</p>
        <p><strong>Collectibles:</strong> Gather coins and power-ups</p>
        <p><strong>Dynamic Difficulty:</strong> Speed increases over time</p>
      </div>
    </div> -->
  </div>

  <!-- Gaming Stats Section -->
  <div class="gaming-stats">
    <div class="stats-container">
      <div class="stat-card">
        <div class="stat-icon">LVL</div>
        <div class="stat-info">
          <div class="stat-label">Level</div>
          <div class="stat-value">{playerLevel}</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon">EXP</div>
        <div class="stat-info">
          <div class="stat-label">Experience</div>
          <div class="stat-value">{experience.toLocaleString()}</div>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" style="width: {(experience / nextLevelExp) * 100}%"></div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon">ACH</div>
        <div class="stat-info">
          <div class="stat-label">Achievements</div>
          <div class="stat-value">{achievements.length}</div>
        </div>
      </div>
    </div>
  </div>

  <!-- Login Section -->
  <div class="login-section">
    <Login />
  </div>

  <!-- Development Tools (Collapsible) -->
  <details class="dev-tools">
    <summary>Development Tools</summary>
    <div class="dev-tools-content">
      <h3>Input Validation System</h3>
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
    background: linear-gradient(135deg, #0a0a0f 0%, #12121a 30%, #1a1a2a 70%, #0f0f14 100%);
    color: #ffffff;
    font-family: 'Inter', 'Segoe UI', system-ui, sans-serif;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 2rem;
    position: relative;
    overflow: hidden;
  }

  .hero-content {
    max-width: 900px;
    margin-bottom: 3rem;
    position: relative;
    background: rgba(15, 15, 20, 0.6);
    padding: 3rem;
    border-radius: 16px;
    border: 1px solid rgba(64, 224, 208, 0.2);
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.3),
      inset 0 1px 0 rgba(64, 224, 208, 0.1);
    backdrop-filter: blur(20px);
  }

  .hero-title {
    font-size: 4.5rem;
    font-weight: 800;
    margin: 0 0 2rem 0;
    background: linear-gradient(135deg, #40e0d0 0%, #a855f7 50%, #06b6d4 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    letter-spacing: -0.02em;
    line-height: 1.1;
    position: relative;
  }

  .hero-subtitle {
    font-size: 1.8rem;
    margin: 0 0 2.5rem 0;
    color: #40e0d0;
    font-weight: 500;
    letter-spacing: 0.02em;
  }

  .hero-description {
    font-size: 1.2rem;
    line-height: 1.7;
    margin: 0 0 4rem 0;
    color: rgba(255, 255, 255, 0.85);
    max-width: 650px;
    margin-left: auto;
    margin-right: auto;
    font-weight: 400;
  }

  .main-links {
    margin-bottom: 4rem;
    display: flex;
    gap: 2rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  .primary-btn {
    display: inline-flex;
    align-items: center;
    gap: 1rem;
    padding: 1.5rem 3rem;
    background: linear-gradient(135deg, #40e0d0 0%, #06b6d4 100%);
    color: #0a0a0f;
    text-decoration: none;
    border-radius: 12px;
    font-size: 1.3rem;
    font-weight: 700;
    margin: 0;
    transition: all 0.3s ease;
    box-shadow:
      0 8px 25px rgba(64, 224, 208, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
    border: 2px solid rgba(64, 224, 208, 0.6);
    position: relative;
    overflow: hidden;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .primary-btn:hover {
    transform: translateY(-3px);
    box-shadow:
      0 12px 35px rgba(64, 224, 208, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.3);
    border-color: #40e0d0;
  }

  .secondary-btn {
    display: inline-flex;
    align-items: center;
    gap: 1rem;
    padding: 1.2rem 2.5rem;
    background: linear-gradient(135deg, #a855f7 0%, #7c3aed 100%);
    color: #fff;
    text-decoration: none;
    border-radius: 12px;
    font-size: 1.2rem;
    font-weight: 600;
    margin: 0;
    transition: all 0.3s ease;
    box-shadow:
      0 8px 25px rgba(168, 85, 247, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    border: 2px solid rgba(168, 85, 247, 0.6);
    position: relative;
    overflow: hidden;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .secondary-btn:hover {
    transform: translateY(-3px);
    box-shadow:
      0 12px 35px rgba(168, 85, 247, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
    border-color: #a855f7;
  }

  .secondary-links {
    margin-top: 2.5rem;
    display: flex;
    gap: 1.5rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  .tertiary-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.8rem;
    padding: 1rem 2rem;
    background: rgba(15, 15, 20, 0.8);
    color: #40e0d0;
    text-decoration: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 500;
    margin: 0;
    transition: all 0.3s ease;
    border: 1px solid rgba(64, 224, 208, 0.3);
    backdrop-filter: blur(10px);
    position: relative;
    overflow: hidden;
  }

  .tertiary-btn:hover {
    background: rgba(64, 224, 208, 0.1);
    transform: translateY(-2px);
    border-color: #40e0d0;
    box-shadow: 0 4px 15px rgba(64, 224, 208, 0.2);
  }

  .gaming-stats {
    margin: 4rem 0;
    position: relative;
  }

  .stats-container {
    display: flex;
    gap: 2rem;
    justify-content: center;
    flex-wrap: wrap;
    max-width: 800px;
    margin: 0 auto;
  }

  .stat-card {
    background: rgba(15, 15, 20, 0.7);
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid rgba(64, 224, 208, 0.2);
    backdrop-filter: blur(10px);
    text-align: center;
    min-width: 180px;
    position: relative;
    transition: all 0.3s ease;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  }

  .stat-card:hover {
    transform: translateY(-3px);
    border-color: #40e0d0;
    box-shadow: 0 8px 30px rgba(64, 224, 208, 0.2);
  }

  .stat-icon {
    font-size: 2.5rem;
    margin-bottom: 0.8rem;
  }

  .stat-info {
    margin-bottom: 0.8rem;
  }

  .stat-label {
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.3rem;
  }

  .stat-value {
    color: #40e0d0;
    font-size: 2rem;
    font-weight: 800;
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    overflow: hidden;
    margin-top: 0.8rem;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #40e0d0 0%, #06b6d4 100%);
    border-radius: 3px;
    transition: width 0.8s ease;
  }

  .login-section {
    margin: 5rem 0;
    width: 100%;
    max-width: 450px;
    position: relative;
    background: rgba(15, 15, 20, 0.7);
    padding: 2.5rem;
    border-radius: 16px;
    border: 1px solid rgba(168, 85, 247, 0.2);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(20px);
  }

  .dev-tools {
    margin-top: 5rem;
    width: 100%;
    max-width: 700px;
    background: rgba(15, 15, 20, 0.7);
    border-radius: 16px;
    border: 1px solid rgba(255, 193, 7, 0.2);
    backdrop-filter: blur(20px);
    position: relative;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  .dev-tools summary {
    padding: 1.5rem 2rem;
    cursor: pointer;
    font-weight: 600;
    color: #ffc107;
    border-bottom: 1px solid rgba(255, 193, 7, 0.2);
    border-radius: 16px 16px 0 0;
    transition: all 0.3s ease;
  }

  .dev-tools summary:hover {
    background: rgba(255, 193, 7, 0.08);
    color: #ffda6a;
  }

  .dev-tools-content {
    padding: 2rem;
    text-align: left;
  }

  .dev-tools-content h3 {
    margin: 0 0 1.5rem 0;
    color: #ffc107;
    font-size: 1.3rem;
  }

  .validation-controls {
    margin: 1.5rem 0;
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .danger-btn {
    background: linear-gradient(135deg, #dc3545, #c82333);
    color: white;
    border: none;
    padding: 0.8rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    font-size: 1rem;
    transition: all 0.3s ease;
    box-shadow: 0 4px 15px rgba(220, 53, 69, 0.3);
  }

  .danger-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(220, 53, 69, 0.4);
  }

  .success-btn {
    background: linear-gradient(135deg, #28a745, #218838);
    color: white;
    border: none;
    padding: 0.8rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    font-size: 1rem;
    transition: all 0.3s ease;
    box-shadow: 0 4px 15px rgba(40, 167, 69, 0.3);
  }

  .success-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(40, 167, 69, 0.4);
  }


  /* Responsive design */
  @media (max-width: 768px) {
    .hero-title {
      font-size: 3.5rem;
    }

    .hero-subtitle {
      font-size: 1.6rem;
    }

    .hero-description {
      font-size: 1.1rem;
      padding: 0 1rem;
    }

    .main-links {
      flex-direction: column;
      align-items: center;
      gap: 1.5rem;
    }

    .primary-btn, .secondary-btn {
      width: 300px;
      justify-content: center;
      padding: 1.2rem 2rem;
    }

    .secondary-links {
      flex-direction: column;
      align-items: center;
      gap: 1rem;
    }

    .tertiary-btn {
      width: 220px;
      justify-content: center;
    }

    .validation-controls {
      flex-direction: column;
    }

    .danger-btn, .success-btn {
      width: 100%;
      text-align: center;
    }

    .hero-content {
      padding: 2rem;
      margin: 1rem;
    }

    .stats-container {
      flex-direction: column;
      align-items: center;
      gap: 1.5rem;
    }

    .stat-card {
      width: 100%;
      max-width: 280px;
      min-width: auto;
    }
  }

  @media (max-width: 480px) {
    .hero-title {
      font-size: 2.8rem;
    }

    .hero-subtitle {
      font-size: 1.3rem;
    }

    .hero-description {
      font-size: 1rem;
    }

    .primary-btn, .secondary-btn {
      padding: 1rem 1.5rem;
      font-size: 1rem;
      width: 260px;
    }

    .hero-content {
      padding: 1.5rem;
    }

    .stat-card {
      padding: 1.2rem;
    }

    .login-section {
      padding: 2rem;
    }
  }

  /* Import clean fonts */
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap');
</style>
