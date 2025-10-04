<script>
  export let errors = [];
  export let title = 'Input Validation Errors';

  $: hasErrors = errors.length > 0;
</script>

{#if hasErrors}
  <div class="validation-errors">
    <div class="error-header">
      <span class="error-icon">⚠️</span>
      <strong>{title}</strong>
    </div>
    <ul class="error-list">
      {#each errors as error (error.field)}
        <li class="error-item">
          <span class="error-field">{error.field}:</span>
          <span class="error-message">{error.message}</span>
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .validation-errors {
    background: rgba(185, 56, 58, 0.1);
    border: 1px solid #b9383a;
    border-radius: 8px;
    padding: 1rem;
    margin: 1rem 0;
    color: #ffcccc;
  }

  .error-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .error-icon {
    font-size: 1rem;
  }

  .error-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .error-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
    padding: 0.25rem 0;
  }

  .error-field {
    font-weight: 600;
    color: #ffb3b3;
  }

  .error-message {
    color: #ffcccc;
  }

  @media (min-width: 768px) {
    .error-item {
      flex-direction: row;
      gap: 0.5rem;
    }

    .error-field::after {
      content: ' ';
    }
  }
</style>
