<script lang="ts">
  import '../app.css';
  import { authStore } from '$lib/stores/auth';
  import { browser } from '$app/environment';

  // Only import Login component on client-side to avoid SSR issues
  let LoginComponent = null;
  if (browser) {
    import('$lib/components/Login.svelte').then(module => {
      LoginComponent = module.default;
    });
  }
</script>

<svelte:head>
  <title>gamev1 client</title>
</svelte:head>

<div class="app">
  <header class="app-header">
    <div class="header-left">
      <h1>gamev1 client</h1>
      <span class="status">Week 3: Authentication System</span>
    </div>

    <div class="header-right">
      {#if LoginComponent}
        <svelte:component this={LoginComponent} />
      {:else}
        <div class="login-placeholder">Loading...</div>
      {/if}
    </div>
  </header>

  <main class="app-main">
    <slot />
  </main>
</div>

<style>
  .app {
    min-height: 100vh;
    background: #0a0e1a;
    color: #f6f8ff;
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 2rem;
    background: #0f1629;
    border-bottom: 1px solid #253157;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .header-right {
    display: flex;
    align-items: center;
  }

  .header-left h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: #f6f8ff;
  }

  .status {
    padding: 0.25rem 0.75rem;
    background: #446bff;
    color: white;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 600;
  }


  .app-main {
    padding: 2rem;
  }

  .login-placeholder {
    color: #666;
    font-size: 0.9rem;
  }
</style>
