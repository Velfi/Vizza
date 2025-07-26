<div class="cursor-config">
  <div class="control-group">
    <div class="control-header">
      <label for="cursorSize">Size</label>
      <span class="range-value">{cursorSize.toFixed(sizePrecision)}</span>
    </div>
    <input
      type="range"
      id="cursorSize"
      bind:value={cursorSize}
      min={sizeMin}
      max={sizeMax}
      step={sizeStep}
      on:input={handleSizeChange}
    />
  </div>

  <div class="control-group">
    <div class="control-header">
      <label for="cursorStrength">Strength</label>
      <span class="range-value">{cursorStrength.toFixed(strengthPrecision)}</span>
    </div>
    <input
      type="range"
      id="cursorStrength"
      bind:value={cursorStrength}
      min={strengthMin}
      max={strengthMax}
      step={strengthStep}
      on:input={handleStrengthChange}
    />
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  // Props
  export let cursorSize: number = 0.5;
  export let cursorStrength: number = 1.0;
  export let sizeMin: number = 0.05;
  export let sizeMax: number = 1.0;
  export let sizeStep: number = 0.05;
  export let strengthMin: number = 0;
  export let strengthMax: number = 20;
  export let strengthStep: number = 0.5;
  export let sizePrecision: number = 2;
  export let strengthPrecision: number = 1;

  function handleSizeChange(event: Event) {
    const value = parseFloat((event.target as HTMLInputElement).value);
    cursorSize = value;
    dispatch('sizechange', value);
  }

  function handleStrengthChange(event: Event) {
    const value = parseFloat((event.target as HTMLInputElement).value);
    cursorStrength = value;
    dispatch('strengthchange', value);
  }
</script>

<style>
  .cursor-config {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .control-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  label {
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.85rem;
    font-weight: 500;
    flex-shrink: 0;
  }

  input[type='range'] {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.1);
    outline: none;
    -webkit-appearance: none;
    appearance: none;
  }

  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #646cff;
    cursor: pointer;
    border: 2px solid rgba(255, 255, 255, 0.3);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  input[type='range']::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #646cff;
    cursor: pointer;
    border: 2px solid rgba(255, 255, 255, 0.3);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  input[type='range']:hover::-webkit-slider-thumb {
    background: #7c82ff;
    transform: scale(1.1);
  }

  input[type='range']:hover::-moz-range-thumb {
    background: #7c82ff;
    transform: scale(1.1);
  }

  .range-value {
    color: rgba(255, 255, 255, 0.8);
    font-family: monospace;
    font-size: 0.8rem;
    font-weight: 500;
    background: rgba(255, 255, 255, 0.1);
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    min-width: 40px;
    text-align: center;
  }

  /* Mobile responsive design */
  @media (max-width: 768px) {
    .cursor-config {
      gap: 0.5rem;
      padding: 0.4rem;
    }

    .control-group {
      gap: 0.2rem;
    }

    .control-header {
      gap: 0.3rem;
    }

    label {
      font-size: 0.8rem;
    }

    .range-value {
      font-size: 0.75rem;
      padding: 0.15rem 0.3rem;
      min-width: 35px;
    }

    input[type='range'] {
      height: 5px;
    }

    input[type='range']::-webkit-slider-thumb {
      width: 14px;
      height: 14px;
    }

    input[type='range']::-moz-range-thumb {
      width: 14px;
      height: 14px;
    }
  }

  /* Very small screens */
  @media (max-width: 480px) {
    .cursor-config {
      padding: 0.3rem;
    }

    .control-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.2rem;
    }

    .range-value {
      align-self: flex-end;
    }
  }
</style>
