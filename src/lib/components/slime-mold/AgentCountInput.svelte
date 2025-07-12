<div class="agent-count-input">
  <div class="input-container">
    <Input
      type="number"
      value={inputValue}
      {min}
      {max}
      step={0.1}
      disabled={isUpdating}
      placeholder="Enter agent count..."
      error={!isValid ? errorMessage : ''}
      on:input={handleInput}
      on:keydown={handleKeyDown}
      on:focus={handleFocus}
      on:blur={handleBlur}
    />
    <Button
      class={`update-button${isUpdating ? ' updating' : ''}`}
      disabled={!isValid || isUpdating}
      on:click={handleUpdate}
    >
      {isUpdating ? 'Updating...' : 'Update'}
    </Button>
  </div>

  {#if !isValid && errorMessage}
    <div class="error-message">
      {errorMessage}
    </div>
  {/if}
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from '../shared/Button.svelte';
  import Input from '../inputs/Input.svelte';

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
    width: max-content;
  }
  .input-container {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  /* Remove .count-input and .update-button styles, rely on shared-theme.css, Button.svelte, and Input.svelte */
</style>
