<script lang="ts">
  // Simple home page for debugging
  import Login from '$lib/components/Login.svelte';
  import ValidationErrors from '$lib/components/ValidationErrors.svelte';
  import { inputValidationErrors } from '$lib/stores/input';

  // SvelteKit route props
  export let data;

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

<section class="container">
  <h1>Welcome to gamev1 client</h1>
  <p>Client is running successfully!</p>

  <!-- Validation Errors Demo -->
  <div style="margin: 2rem 0;">
    <h2>🛡️ Input Validation System</h2>
    <p>Testing the new input validation system:</p>
    <div style="margin: 1rem 0;">
      <button on:click={testValidation} style="margin-right: 1rem; background: #ff6b6b; color: white; border: none; padding: 0.5rem 1rem; border-radius: 4px; cursor: pointer;">
        Test Validation Errors
      </button>
      <button on:click={clearValidation} style="background: #51cf66; color: white; border: none; padding: 0.5rem 1rem; border-radius: 4px; cursor: pointer;">
        Clear Errors
      </button>
    </div>

    <ValidationErrors errors={validationErrors} title="Demo Validation Errors" />
  </div>

  <!-- Test Login Component -->
  <div style="margin: 2rem 0;">
    <Login />
  </div>

  <div class="links">
    <a href="/rooms">🎮 Browse Rooms</a>
    <a href="/game">🎯 Quick Game</a>
    <a href="/net-test">Network Test</a>
  </div>
</section>

<style>
  .container {
    max-width: 800px;
    margin: 2rem auto;
    padding: 2rem;
    text-align: center;
  }

  .links {
    margin-top: 2rem;
  }

  .links a {
    display: inline-block;
    padding: 1rem 2rem;
    background: #446bff;
    color: white;
    text-decoration: none;
    border-radius: 8px;
    margin: 0 1rem;
  }

  .links a:hover {
    background: #3359e0;
  }
</style>
