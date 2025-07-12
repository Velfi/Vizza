<div class="input-wrapper" class:error>
  {#if label}
    <label for={id} class="input-label">{label}</label>
  {/if}
  <input
    {id}
    {type}
    {value}
    {placeholder}
    {min}
    {max}
    {step}
    {disabled}
    {readonly}
    class="ui-input"
    on:input
    on:change
    on:focus
    on:blur
    on:keydown
    {...$$restProps}
  />
  {#if error}
    <div class="error-message">{error}</div>
  {/if}
</div>

<script lang="ts">
  export let id: string = '';
  export let type: string = 'text';
  export let value: string | number = '';
  export let placeholder: string = '';
  export let label: string = '';
  export let error: string = '';
  export let disabled: boolean = false;
  export let readonly: boolean = false;
  export let min: number | undefined = undefined;
  export let max: number | undefined = undefined;
  export let step: number | undefined = undefined;
</script>

<style>
  .input-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }
  .input-label {
    color: var(--ui-text-muted);
    font-size: 0.9rem;
    font-weight: 500;
  }
  .ui-input {
    padding: var(--spacing-sm);
    border: 1px solid var(--ui-border);
    border-radius: var(--radius-sm);
    background: var(--ui-bg);
    color: var(--ui-text);
    font-family: inherit;
    font-size: 0.875rem;
    height: var(--input-height);
    box-sizing: border-box;
    transition: var(--transition-fast);
    width: 100%;
  }
  .ui-input:focus {
    outline: none;
    border-color: var(--ui-border-focus);
    background: var(--ui-bg-focus);
  }
  .ui-input:hover:not(:disabled):not(:readonly) {
    background: var(--ui-bg-hover);
  }
  .ui-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .ui-input:readonly {
    background: rgba(255, 255, 255, 0.05);
    cursor: default;
  }
  .ui-input::placeholder {
    color: var(--ui-text-disabled);
  }
  .ui-input.error {
    border-color: var(--accent-danger);
    background: rgba(255, 107, 107, 0.1);
  }
  .error-message {
    color: var(--accent-danger);
    font-size: 0.8rem;
    padding: var(--spacing-xs) var(--spacing-sm);
    background: rgba(255, 107, 107, 0.1);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(255, 107, 107, 0.3);
  }
  .ui-input[type='number']::-webkit-outer-spin-button,
  .ui-input[type='number']::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .ui-input[type='number'] {
    -moz-appearance: textfield;
  }
</style>
