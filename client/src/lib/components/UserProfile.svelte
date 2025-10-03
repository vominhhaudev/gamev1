<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore, authActions } from '$lib/stores/auth';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  let isAuthenticated = false;
  let currentUser = null;
  let isLoading = false;
  let error = '';
  let isEditing = false;
  let editForm = {
    name: '',
    email: '',
    avatar: ''
  };

  // Subscribe to auth store
  onMount(() => {
    const unsubscribe = authStore.subscribe(state => {
      isAuthenticated = state.isAuthenticated;
      currentUser = state.user;
      isLoading = state.isLoading;

      // Initialize edit form with current user data
      if (currentUser && !editForm.email) {
        editForm = {
          name: currentUser.name || '',
          email: currentUser.email || '',
          avatar: currentUser.avatar || ''
        };
      }
    });

    return unsubscribe;
  });

  function startEditing() {
    isEditing = true;
    editForm = {
      name: currentUser?.name || '',
      email: currentUser?.email || '',
      avatar: currentUser?.avatar || ''
    };
  }

  function cancelEditing() {
    isEditing = false;
    error = '';
  }

  async function saveProfile(event: Event) {
    event.preventDefault();
    error = '';
    isLoading = true;

    if (!editForm.name || !editForm.email) {
      error = 'Name and email are required';
      isLoading = false;
      return;
    }

    try {
      const response = await fetch('http://127.0.0.1:8080/auth/profile', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${authActions.getAuthHeaders()['Authorization']?.replace('Bearer ', '')}`,
        },
        body: JSON.stringify(editForm),
      });

      if (response.ok) {
        const updatedUser = await response.json();
        // Update local user data
        authStore.update(state => ({
          ...state,
          user: { ...state.user, ...updatedUser }
        }));
        isEditing = false;
        dispatch('profileUpdated');
      } else {
        const errorData = await response.json().catch(() => ({ message: 'Failed to update profile' }));
        error = errorData.message || 'Failed to update profile';
      }
    } catch (err) {
      error = 'Network error. Please try again later.';
    }

    isLoading = false;
  }

  function handleLogout() {
    authActions.logout();
    dispatch('logout');
  }
</script>

<div class="profile-container">
  <div class="profile-header">
    <h2>User Profile</h2>
    <button class="logout-btn" on:click={handleLogout} disabled={isLoading}>
      Logout
    </button>
  </div>

  {#if currentUser}
    {#if isEditing}
      <!-- Edit Profile Form -->
      <form class="profile-form" on:submit={saveProfile}>
        <div class="form-group">
          <label for="name">Name</label>
          <input
            id="name"
            type="text"
            bind:value={editForm.name}
            placeholder="Enter your name"
            required
            disabled={isLoading}
          />
        </div>

        <div class="form-group">
          <label for="email">Email</label>
          <input
            id="email"
            type="email"
            bind:value={editForm.email}
            placeholder="Enter your email"
            required
            disabled={isLoading}
          />
        </div>

        <div class="form-group">
          <label for="avatar">Avatar URL</label>
          <input
            id="avatar"
            type="url"
            bind:value={editForm.avatar}
            placeholder="https://example.com/avatar.jpg"
            disabled={isLoading}
          />
        </div>

        {#if error}
          <div class="error-message">
            {error}
          </div>
        {/if}

        <div class="form-actions">
          <button type="submit" class="save-btn" disabled={isLoading}>
            {#if isLoading}
              Saving...
            {:else}
              Save Changes
            {/if}
          </button>
          <button type="button" class="cancel-btn" on:click={cancelEditing} disabled={isLoading}>
            Cancel
          </button>
        </div>
      </form>
    {:else}
      <!-- View Profile -->
      <div class="profile-view">
        <div class="avatar-section">
          <div class="avatar">
            {#if currentUser.avatar}
              <img src={currentUser.avatar} alt="Avatar" />
            {:else}
              {currentUser.email?.charAt(0).toUpperCase() || '?'}
            {/if}
          </div>
        </div>

        <div class="profile-info">
          <div class="info-group">
            <label>Name:</label>
            <span>{currentUser.name || 'Not set'}</span>
          </div>

          <div class="info-group">
            <label>Email:</label>
            <span>{currentUser.email}</span>
          </div>

          <div class="info-group">
            <label>User ID:</label>
            <span>{currentUser.id}</span>
          </div>

          <div class="info-group">
            <label>Joined:</label>
            <span>{currentUser.created ? new Date(currentUser.created).toLocaleDateString() : 'Unknown'}</span>
          </div>
        </div>

        <div class="profile-actions">
          <button class="edit-btn" on:click={startEditing} disabled={isLoading}>
            Edit Profile
          </button>
        </div>
      </div>
    {/if}
  {:else}
    <div class="no-user">
      <p>Please log in to view your profile.</p>
    </div>
  {/if}
</div>

<style>
  .profile-container {
    max-width: 600px;
    margin: 2rem auto;
    padding: 2rem;
    background: #0b0f1a;
    border-radius: 12px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.25);
    border: 1px solid #253157;
  }

  .profile-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #253157;
  }

  .profile-header h2 {
    margin: 0;
    color: #f6f8ff;
    font-size: 1.5rem;
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

  .logout-btn:hover:not(:disabled) {
    background: #a02f31;
  }

  .logout-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .avatar-section {
    display: flex;
    justify-content: center;
    margin-bottom: 2rem;
  }

  .avatar {
    width: 100px;
    height: 100px;
    border-radius: 50%;
    background: linear-gradient(135deg, #446bff, #3359e0);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 2.5rem;
    color: white;
    overflow: hidden;
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .profile-info {
    margin-bottom: 2rem;
  }

  .info-group {
    display: flex;
    margin-bottom: 1rem;
    align-items: center;
  }

  .info-group label {
    min-width: 80px;
    color: #90a0d0;
    font-weight: 600;
    margin-right: 1rem;
  }

  .info-group span {
    color: #f6f8ff;
    flex: 1;
  }

  .profile-actions {
    display: flex;
    justify-content: center;
  }

  .edit-btn {
    padding: 0.75rem 1.5rem;
    background: #446bff;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }

  .edit-btn:hover:not(:disabled) {
    background: #3359e0;
  }

  .edit-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .profile-form {
    margin-top: 1rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    color: #c3ccec;
    font-weight: 600;
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

  .form-actions {
    display: flex;
    gap: 1rem;
    margin-top: 1.5rem;
  }

  .save-btn {
    flex: 1;
    padding: 0.75rem;
    background: #446bff;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .save-btn:hover:not(:disabled) {
    background: #3359e0;
  }

  .save-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .cancel-btn {
    flex: 1;
    padding: 0.75rem;
    background: transparent;
    color: #90a0d0;
    border: 1px solid #253157;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .cancel-btn:hover:not(:disabled) {
    background: #253157;
    color: #c3ccec;
  }

  .cancel-btn:disabled {
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

  .no-user {
    text-align: center;
    padding: 2rem;
    color: #90a0d0;
  }
</style>
