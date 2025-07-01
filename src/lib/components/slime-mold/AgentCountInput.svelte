<div class="agent-count-input">
  <div class="input-container">
    <input
      type="number"
      value={inputValue}
      {min}
      {max}
      step="0.1"
      class="count-input"
      class:invalid={!isValid}
      on:input={handleInput}
      on:keydown={handleKeyDown}
      on:focus={handleFocus}
      on:blur={handleBlur}
      disabled={isUpdating}
      placeholder="Enter agent count..."
    />
    <button
      class="update-button"
      class:updating={isUpdating}
      disabled={!isValid || isUpdating}
      on:click={handleUpdate}
    >
      {isUpdating ? 'Updating...' : 'Update'}
    </button>
  </div>

  {#if !isValid && errorMessage}
    <div class="error-message">
      {errorMessage}
    </div>
  {/if}
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let value: number = 1;
  export let min: number = 0;
  export let max: number = 100;

  const dispatch = createEventDispatcher();

  let inputValue: string = value.toString();
  let isValid: boolean = true;
  let errorMessage: string = '';
  let isUpdating: boolean = false;

  function validateInput(val: string): { valid: boolean; message: string; numValue?: number } {
    if (val.trim() === '') {
      return { valid: false, message: 'Agent count cannot be empty' };
    }

    const numValue = parseFloat(val);

    if (isNaN(numValue)) {
      return { valid: false, message: 'Must be a valid number' };
    }

    if (numValue < min) {
      return { valid: false, message: `Must be at least ${min}` };
    }

    if (numValue > max) {
      return { valid: false, message: `Must be at most ${max}` };
    }

    if (numValue % 0.1 !== 0 && numValue % 1 !== 0) {
      return { valid: false, message: 'Must be a whole number or single decimal place' };
    }

    return { valid: true, message: '', numValue };
  }

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    inputValue = target.value;
    userIsEditing = true;

    const validation = validateInput(inputValue);
    isValid = validation.valid;
    errorMessage = validation.message;
  }

  async function handleUpdate() {
    const validation = validateInput(inputValue);

    if (!validation.valid || validation.numValue === undefined) {
      isValid = false;
      errorMessage = validation.message;
      return;
    }

    isUpdating = true;

    try {
      dispatch('update', validation.numValue);
      value = validation.numValue;
      isValid = true;
      errorMessage = '';
      userIsEditing = false; // Clear editing flag after successful update
    } catch (error) {
      isValid = false;
      errorMessage = 'Failed to update agent count';
      console.error('Agent count update failed:', error);
    } finally {
      isUpdating = false;
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      handleUpdate();
    }
  }

  function handleFocus() {
    userIsEditing = true;
  }

  function handleBlur() {
    // Don't immediately clear editing flag on blur - let the update handle it
    // This prevents the input from resetting if user clicks elsewhere briefly
  }

  // Update input when value prop changes externally
  // But don't reset if the user is currently editing the input
  let userIsEditing = false;

  $: if (value.toString() !== inputValue && !isUpdating && !userIsEditing) {
    inputValue = value.toString();
    const validation = validateInput(inputValue);
    isValid = validation.valid;
    errorMessage = validation.message;
  }
</script>

<style>
  .agent-count-input {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-container {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .count-input {
    flex: 1;
    padding: 0.5rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    font-family: inherit;
    font-size: 0.9rem;
    transition: all 0.2s ease;
  }

  .count-input:focus {
    outline: none;
    border-color: rgba(100, 108, 255, 0.5);
    background: rgba(255, 255, 255, 0.15);
  }

  .count-input.invalid {
    border-color: rgba(255, 75, 75, 0.7);
    background: rgba(255, 75, 75, 0.1);
  }

  .count-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .update-button {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(100, 108, 255, 0.5);
    border-radius: 4px;
    background: rgba(100, 108, 255, 0.2);
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.9rem;
    transition: all 0.2s ease;
    white-space: nowrap;
  }

  .update-button:hover:not(:disabled) {
    background: rgba(100, 108, 255, 0.3);
    border-color: rgba(100, 108, 255, 0.7);
  }

  .update-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    background: rgba(128, 128, 128, 0.2);
    border-color: rgba(128, 128, 128, 0.3);
  }

  .update-button.updating {
    background: rgba(255, 165, 0, 0.2);
    border-color: rgba(255, 165, 0, 0.5);
  }

  .error-message {
    color: rgba(255, 75, 75, 0.9);
    font-size: 0.8rem;
    padding: 0.25rem 0.5rem;
    background: rgba(255, 75, 75, 0.1);
    border-radius: 4px;
    border: 1px solid rgba(255, 75, 75, 0.3);
  }

  /* Input number spinner removal */
  .count-input::-webkit-outer-spin-button,
  .count-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .count-input[type='number'] {
    -moz-appearance: textfield;
  }
</style>
