<script>
  // Authentication component with real backend connection
  import { authStore, authActions } from '$lib/stores/auth';

  let email = '';
  let password = '';
  let error = '';
  let isLoading = false;
  let isAuthenticated = false;
  let currentUser = null;

  // Subscribe to auth store
  authStore.subscribe(state => {
    isLoading = state.isLoading;
    isAuthenticated = state.isAuthenticated;
    currentUser = state.user;
  });

  async function handleSubmit(event) {
    event.preventDefault();
    error = '';
    isLoading = true;

    if (!email || !password) {
      error = 'Please fill in all fields';
      isLoading = false;
      return;
    }

    try {
      const result = await authActions.login(email, password, false);
      if (result.success) {
        // Success - store will update
      } else {
        error = result.error || 'Login failed';
      }
    } catch (err) {
      error = 'Network error. Please try again later.';
    }
  }
</script>

<div class="login-container">
  {#if isAuthenticated && currentUser}
    <!-- Logged in state -->
    <div class="user-info">
      <div class="avatar">
        {currentUser.email.charAt(0).toUpperCase()}
      </div>
      <div class="user-details">
        <div class="email">{currentUser.email}</div>
        <div class="user-id">ID: {currentUser.id}</div>
      </div>
    </div>
    <button class="logout-btn" on:click={() => authActions.logout()}>
      Logout
    </button>
  {:else}
    <!-- Login form -->
    <h2>Login to GameV1</h2>

    {#if error}
      <div class="error-message">
        {error}
      </div>
    {/if}

    <form on:submit={handleSubmit}>
      <div class="form-group">
        <input
          type="email"
          bind:value={email}
          placeholder="admin@pocketbase.local"
          required
          disabled={isLoading}
        />
      </div>

      <div class="form-group">
        <input
          type="password"
          bind:value={password}
          placeholder="123456789"
          required
          disabled={isLoading}
        />
      </div>

      <button type="submit" disabled={isLoading}>
        {#if isLoading}
          Logging in...
        {:else}
          Login
        {/if}
      </button>
    </form>

    <div class="demo-credentials">
      <small>Demo: admin@pocketbase.local / 123456789</small>
    </div>
  {/if}
</div>

<style>
  .login-container {
    max-width: 400px;
    margin: 2rem auto;
    padding: 2rem;
    background: #0b0f1a;
    border-radius: 12px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.25);
    border: 1px solid #253157;
  }


  h2 {
    margin: 0 0 1.5rem 0;
    color: #f6f8ff;
    text-align: center;
    font-size: 1.5rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #253157;
    border-radius: 8px;
    background: #121a2b;
    color: #f6f8ff;
    font-size: 1rem;
    transition: border-color 0.2s;
  }

  .form-group input:focus {
    outline: none;
    border-color: #446bff;
  }

  .form-group input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }


  button[type="submit"] {
    width: 100%;
    padding: 0.75rem;
    background: #446bff;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
    margin-top: 0.5rem;
  }

  button[type="submit"]:hover:not(:disabled) {
    background: #3359e0;
  }

  button[type="submit"]:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-message {
    background: #b9383a;
    color: white;
    padding: 0.75rem;
    border-radius: 8px;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  .demo-credentials {
    text-align: center;
    margin-top: 1rem;
    color: #90a0d0;
  }

  .demo-credentials small {
    font-size: 0.8rem;
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .avatar {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: linear-gradient(135deg, #446bff, #3359e0);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 1.2rem;
    color: white;
  }

  .user-details {
    flex: 1;
  }

  .email {
    font-weight: 600;
    color: #f6f8ff;
    margin-bottom: 0.25rem;
  }

  .user-id {
    font-size: 0.8rem;
    color: #90a0d0;
  }

  .logout-btn {
    padding: 0.5rem 1rem;
    background: #b9383a;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }

  .logout-btn:hover {
    background: #a02f31;
  }
</style>
