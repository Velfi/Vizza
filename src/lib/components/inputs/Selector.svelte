<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  /**
   * Selector Component
   * 
   * A reusable component that implements the arrow-button + select box pattern:
   * ⬅️ SelectBox ➡️
   * 
   * Features:
   * - Left/right arrow buttons to cycle through options
   * - Dropdown select for direct selection
   * - Optional label
   * - Disabled state support
   * - Placeholder text support
   * 
   * Usage:
   * <Selector
   *   options={['Option 1', 'Option 2', 'Option 3']}
   *   bind:value={selectedValue}
   *   label="Choose an option"
   *   on:change={({ detail }) => console.log('Selected:', detail.value)}
   * />
   */

  export let options: string[] = [];
  export let value: string = '';
  export let placeholder: string = 'Select...';
  export let disabled: boolean = false;
  export let label: string = '';
  export let id: string = '';

  const dispatch = createEventDispatcher<{
    change: { value: string };
  }>();

  function cycleBack() {
    if (options.length === 0 || disabled) return;
    const currentIndex = options.indexOf(value);
    const newIndex = currentIndex > 0 ? currentIndex - 1 : options.length - 1;
    const newValue = options[newIndex];
    value = newValue;
    dispatch('change', { value: newValue });
  }

  function cycleForward() {
    if (options.length === 0 || disabled) return;
    const currentIndex = options.indexOf(value);
    const newIndex = currentIndex < options.length - 1 ? currentIndex + 1 : 0;
    const newValue = options[newIndex];
    value = newValue;
    dispatch('change', { value: newValue });
  }

  function handleSelect(event: Event) {
    const select = event.target as HTMLSelectElement;
    const selectedValue = select.value;
    value = selectedValue;
    dispatch('change', { value: selectedValue });
  }
</script>

<div class="selector">
  {#if label}
    <label for={id || 'selector'}>
      {label}
    </label>
  {/if}
  
  <div class="selector-controls">
    <button 
      type="button" 
      class="control-btn left-btn"
      on:click={cycleBack}
      disabled={disabled || options.length === 0}
      title="Previous option"
    >
      ◀
    </button>
    
    <select 
      {id}
      {value}
      {placeholder}
      {disabled}
      on:change={handleSelect}
      class="selector-select"
    >
      {#if placeholder && !options.includes(value)}
        <option value="" disabled>{placeholder}</option>
      {/if}
      {#each options as option}
        <option value={option}>{option}</option>
      {/each}
    </select>
    
    <button 
      type="button" 
      class="control-btn right-btn"
      on:click={cycleForward}
      disabled={disabled || options.length === 0}
      title="Next option"
    >
      ▶
    </button>
  </div>
</div>

<style>
  .selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .selector label {
    display: block;
    margin-bottom: 0.25rem;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
  }

  .selector-controls {
    display: flex;
    align-items: center;
    position: relative;
  }

  .control-btn {
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s ease;
    min-width: 35px;
    height: 35px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-right: none;
  }

  .control-btn.left-btn {
    border-radius: 4px 0 0 4px;
  }

  .control-btn.right-btn {
    border-radius: 0 4px 4px 0;
    border-right: 1px solid rgba(255, 255, 255, 0.2);
    border-left: none;
  }

  .control-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 1);
  }

  .control-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .control-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .selector-select {
    flex: 1;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-left: none;
    border-right: none;
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.9rem;
    min-height: 35px;
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='rgba(255,255,255,0.6)' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6,9 12,15 18,9'%3e%3c/polyline%3e%3c/svg%3e");
    background-repeat: no-repeat;
    background-position: right 0.5rem center;
    background-size: 1rem;
    padding-right: 2rem;
  }

  .selector-select:focus {
    outline: none;
    border-color: rgba(255, 255, 255, 0.5);
    background-color: rgba(255, 255, 255, 0.15);
  }

  .selector-select:hover {
    background-color: rgba(255, 255, 255, 0.15);
  }

  .selector-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .selector-select option {
    background: #1f2937;
    color: rgba(255, 255, 255, 0.9);
  }

  /* Ensure all elements have the same height */
  .control-btn,
  .selector-select {
    height: 35px;
    box-sizing: border-box;
  }
</style> 