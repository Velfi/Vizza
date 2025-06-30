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

<div class="cursor-config">
  <div class="control-group">
    <label for="cursorSize">Cursor Size</label>
    <input 
      type="range" 
      id="cursorSize"
      value={cursorSize}
      min={sizeMin}
      max={sizeMax}
      step={sizeStep}
      on:input={handleSizeChange}
    />
    <span class="range-value">{cursorSize.toFixed(sizePrecision)}</span>
  </div>
  
  <div class="control-group">
    <label for="cursorStrength">Cursor Strength</label>
    <input 
      type="range" 
      id="cursorStrength"
      value={cursorStrength}
      min={strengthMin}
      max={strengthMax}
      step={strengthStep}
      on:input={handleStrengthChange}
    />
    <span class="range-value">{cursorStrength.toFixed(strengthPrecision)}</span>
  </div>
</div>

<style>
  .cursor-config {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .control-group {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    color: rgba(255, 255, 255, 0.8);
    min-width: 120px;
  }

  input[type="range"] {
    flex: 1;
    min-width: 150px;
  }

  .range-value {
    color: rgba(255, 255, 255, 0.8);
    font-family: monospace;
    font-size: 0.9rem;
    min-width: 60px;
    text-align: right;
  }
</style> 