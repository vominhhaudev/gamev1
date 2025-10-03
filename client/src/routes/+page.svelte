<script lang="ts">
  // Simple home page for debugging
  import Login from '$lib/components/Login.svelte';
</script>

<svelte:head>
  <title>Home - gamev1 client</title>
</svelte:head>

<section class="container">
  <h1>Welcome to gamev1 client</h1>
  <p>Client is running successfully!</p>

  <!-- Test Login Component -->
  <div style="margin: 2rem 0;">
    <Login />
  </div>

  <div class="links">
    <a href="/game">🎮 Play Game</a>
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
