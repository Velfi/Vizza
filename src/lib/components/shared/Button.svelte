<button
  class="ui-button"
  class:primary={variant === 'primary'}
  class:success={variant === 'success'}
  class:danger={variant === 'danger'}
  class:warning={variant === 'warning'}
  class:small={size === 'small'}
  class:large={size === 'large'}
  {disabled}
  {type}
  on:click
  {...$$restProps}
>
  <slot />
</button>

<script lang="ts">
  export let variant: 'default' | 'primary' | 'success' | 'danger' | 'warning' = 'default';
  export let size: 'small' | 'medium' | 'large' = 'medium';
  export let disabled: boolean = false;
  export let type: 'button' | 'submit' | 'reset' = 'button';
</script>

<style>
  .ui-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--ui-border);
    border-radius: var(--radius-sm);
    background: var(--ui-bg);
    color: var(--ui-text);
    cursor: pointer;
    font-family: inherit;
    font-size: inherit;
    font-weight: inherit;
    transition: var(--transition-fast);
    user-select: none;
    white-space: nowrap;
    box-sizing: border-box;
    padding: var(--spacing-sm) var(--spacing-md);
    height: var(--button-height);
  }
  .ui-button.small {
    padding: var(--spacing-xs) var(--spacing-sm);
    font-size: 0.875rem;
    height: 28px;
  }
  .ui-button.large {
    padding: var(--spacing-md) var(--spacing-lg);
    font-size: 1.125rem;
    height: 44px;
  }
  .ui-button:hover:not(:disabled) {
    background: var(--ui-bg-hover);
    border-color: var(--ui-border-hover);
  }
  .ui-button:focus {
    outline: none;
    border-color: var(--ui-border-focus);
  }
  .ui-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ui-button.primary {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }
  .ui-button.primary:hover:not(:disabled) {
    background: var(--accent-primary-hover);
    border-color: var(--accent-primary-hover);
  }
  .ui-button.success {
    background: var(--accent-success);
    border-color: var(--accent-success);
    color: white;
  }
  .ui-button.danger {
    background: var(--accent-danger);
    border-color: var(--accent-danger);
    color: white;
  }
  .ui-button.warning {
    background: var(--accent-warning);
    border-color: var(--accent-warning);
    color: #1a1a1a;
  }
</style>
