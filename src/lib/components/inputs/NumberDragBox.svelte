<div class="number-drag-container">
  {#if showButtons}
    <button
      class="step-button decrement"
      on:click={decrement}
      disabled={min !== undefined && value <= min}
      title="Decrease value"
    >
      -
    </button>
  {/if}

  <div
    class="number-drag-box"
    class:dragging={isDragging}
    class:editing={isEditing}
    on:mousedown={handleMouseDown}
    on:dblclick={handleDoubleClick}
    role="spinbutton"
    tabindex="0"
  >
    {#if isEditing}
      <input
        bind:this={inputElement}
        {id}
        type="number"
        value={displayValue}
        {min}
        {max}
        {step}
        on:keydown={handleInputKeyDown}
        on:blur={handleInputBlur}
        class="drag-box-input"
      />
    {:else}
      <span class="value-display">
        {displayValue}{#if unit}<span class="unit">{unit}</span>{/if}
      </span>
    {/if}
  </div>

  {#if showButtons}
    <button
      class="step-button increment"
      on:click={increment}
      disabled={max !== undefined && value >= max}
      title="Increase value"
    >
      +
    </button>
  {/if}
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let value: number = 0;
  export let min: number | undefined = undefined;
  export let max: number | undefined = undefined;
  export let step: number = 1;
  export let precision: number = 2;
  export let unit: string = '';
  export let showButtons: boolean = true;
  export let id: string = '';

  const dispatch = createEventDispatcher();

  let inputElement: HTMLInputElement;
  let isDragging = false;
  let dragStartX = 0;
  let dragStartValue = 0;
  let isEditing = false;

  // Ensure value is always a valid number
  $: if (typeof value !== 'number' || isNaN(value)) {
    value = 0;
  }

  function clamp(val: number): number {
    let result = val;
    if (min !== undefined) result = Math.max(min, result);
    if (max !== undefined) result = Math.min(max, result);
    return result;
  }

  function formatValue(val: number): string {
    if (typeof val !== 'number' || isNaN(val)) {
      return '0';
    }
    return parseFloat(val.toFixed(precision)).toString();
  }

  function handleMouseDown(event: MouseEvent) {
    if (isEditing) return;

    isDragging = true;
    dragStartX = event.clientX;
    dragStartValue = value;

    event.preventDefault();

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isDragging) return;

    // Prevent any focus changes during dragging
    event.preventDefault();

    const deltaX = event.clientX - dragStartX;
    const deltaValue = (deltaX / 100) * step;
    const rawValue = dragStartValue + deltaValue;

    // Snap to step increments
    const steppedValue = Math.round(rawValue / step) * step;
    const newValue = clamp(steppedValue);

    if (newValue !== value) {
      value = newValue;
      dispatch('change', newValue);
    }
  }

  function handleMouseUp(event: MouseEvent) {
    isDragging = false;
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);

    // Prevent focus changes after dragging
    event.preventDefault();
    event.stopPropagation();
  }

  function handleDoubleClick() {
    isEditing = true;
    setTimeout(() => {
      inputElement?.select();
    }, 0);
  }

  function handleInputKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      handleInputBlur();
    } else if (event.key === 'Escape') {
      inputElement.value = formatValue(value);
      isEditing = false;
    }
  }

  function handleInputBlur() {
    const newValue = clamp(parseFloat(inputElement.value) || 0);
    if (newValue !== value) {
      value = newValue;
      dispatch('change', newValue);
    }
    isEditing = false;
  }

  function increment() {
    const currentValue = typeof value === 'number' && !isNaN(value) ? value : 0;
    const newValue = clamp(currentValue + step);
    if (newValue !== currentValue) {
      value = newValue;
      dispatch('change', newValue);
    }
  }

  function decrement() {
    const currentValue = typeof value === 'number' && !isNaN(value) ? value : 0;
    const newValue = clamp(currentValue - step);
    if (newValue !== currentValue) {
      value = newValue;
      dispatch('change', newValue);
    }
  }

  $: displayValue = formatValue(value);
</script>

<style>
  .number-drag-container {
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .step-button {
    background: rgba(255, 255, 255, 0.1);
    border: none;
    color: rgba(255, 255, 255, 0.8);
    cursor: pointer;
    padding: 0.4rem 0.6rem;
    font-size: 0.9rem;
    font-weight: bold;
    transition: all 0.2s ease;
    border-right: 1px solid rgba(255, 255, 255, 0.2);
    user-select: none;
  }

  .step-button.increment {
    border-right: none;
    border-left: 1px solid rgba(255, 255, 255, 0.2);
  }

  .step-button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 1);
  }

  .step-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .number-drag-box {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-grow: 1;
    padding: 0.4rem 0.8rem;
    background: transparent;
    border: none;
    cursor: ew-resize;
    user-select: none;
    text-align: center;
    transition: all 0.2s ease;
    color: rgba(255, 255, 255, 0.9);
    font-family: monospace;
    font-size: 0.9rem;
    min-width: 0;
  }

  .number-drag-box:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .number-drag-box.dragging {
    background: rgba(100, 108, 255, 0.2);
    cursor: ew-resize;
  }

  .number-drag-box.editing {
    cursor: text;
    background: rgba(255, 255, 255, 0.9);
    color: #333;
  }

  .drag-box-input {
    background: transparent;
    border: none;
    outline: none;
    width: 100%;
    text-align: center;
    color: inherit;
    font-family: inherit;
    font-size: inherit;
    padding: 0;
    margin: 0;
  }

  .value-display {
    display: block;
    width: 100%;
  }

  .unit {
    opacity: 0.7;
    margin-left: 0.2rem;
    font-size: 0.8rem;
  }
</style>
