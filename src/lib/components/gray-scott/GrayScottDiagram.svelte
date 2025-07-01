<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import XYPlot from './XYPlot.svelte';
  
  export let feedRate: number = 0.055;
  export let killRate: number = 0.062;
  export let diffusionRateU: number = 0.1;
  export let diffusionRateV: number = 0.05;
  export let timestep: number = 1.0;

  const dispatch = createEventDispatcher();

  // Parameter ranges
  const feedRateRange = { min: 0.02, max: 0.08 };
  const killRateRange = { min: 0.04, max: 0.08 };
  const diffusionRange = { min: 0.05, max: 0.3 };
  const timestepRange = { min: 0.1, max: 3.0 };

  // Internal state for timestep
  let internalTimestep = timestep;

  // Handle timestep slider change
  function handleTimestepChange(event: Event) {
    const target = event.target as HTMLInputElement;
    const newTimestep = parseFloat(target.value);
    internalTimestep = newTimestep;
    dispatch('update', { setting: 'timestep', value: newTimestep });
  }

  // Handle XY plot updates
  function handleFeedKillUpdate(event: CustomEvent) {
    const { x, y } = event.detail;
    dispatch('update', { setting: 'feed_rate', value: x });
    dispatch('update', { setting: 'kill_rate', value: y });
  }

  function handleDiffusionUpdate(event: CustomEvent) {
    const { x, y } = event.detail;
    dispatch('update', { setting: 'diffusion_rate_u', value: x });
    dispatch('update', { setting: 'diffusion_rate_v', value: y });
  }

  // Reset to default values
  function resetToDefaults() {
    const defaults = {
      feed_rate: 0.055,
      kill_rate: 0.062,
      diffusion_rate_u: 0.1,
      diffusion_rate_v: 0.05,
      timestep: 1.0
    };
    
    // Update internal timestep
    internalTimestep = defaults.timestep;
    
    // Dispatch updates to parent
    dispatch('update', { setting: 'feed_rate', value: defaults.feed_rate });
    dispatch('update', { setting: 'kill_rate', value: defaults.kill_rate });
    dispatch('update', { setting: 'diffusion_rate_u', value: defaults.diffusion_rate_u });
    dispatch('update', { setting: 'diffusion_rate_v', value: defaults.diffusion_rate_v });
    dispatch('update', { setting: 'timestep', value: defaults.timestep });
  }

  // Sync with external props when they change
  $: if (timestep !== internalTimestep && Math.abs(timestep - internalTimestep) > 0.01) {
    internalTimestep = timestep;
  }
</script>

<div class="diagram-container">
  <div class="instructions">
      <span>Drag the colorful handles to adjust reaction-diffusion parameters</span>
  </div>

  <div class="plots-container">
    <!-- Feed Rate vs Kill Rate Plot -->
    <div class="plot-section">
      <XYPlot
        xValue={feedRate}
        yValue={killRate}
        xRange={feedRateRange}
        yRange={killRateRange}
        xLabel="Feed Rate (F)"
        yLabel="Kill Rate (K)"
        title="Feed Rate (F) vs Kill Rate (K)"
        handleColor="#ef4444"
        handleStrokeColor="#dc2626"
        valueLabelX="F"
        valueLabelY="K"
        width={400}
        height={300}
        margin={40}
        on:update={handleFeedKillUpdate}
      />
    </div>

    <!-- Diffusion U vs Diffusion V Plot -->
    <div class="plot-section">
      <XYPlot
        xValue={diffusionRateU}
        yValue={diffusionRateV}
        xRange={diffusionRange}
        yRange={diffusionRange}
        xLabel="Diffusion Rate U (Du)"
        yLabel="Diffusion Rate V (Dv)"
        title="Diffusion Rate U (Du) vs Diffusion Rate V (Dv)"
        handleColor="#22c55e"
        handleStrokeColor="#16a34a"
        valueLabelX="Du"
        valueLabelY="Dv"
        width={400}
        height={300}
        margin={40}
        on:update={handleDiffusionUpdate}
      />
    </div>
  </div>

  <!-- HTML-based timestep slider -->
  <div class="timestep-section">
    <div class="timestep-header">
      <h4>Timestep (Δt)</h4>
      <span class="timestep-value">{internalTimestep.toFixed(1)}</span>
    </div>
    <div class="slider-container">
      <input
        type="range"
        min={timestepRange.min}
        max={timestepRange.max}
        step="0.1"
        value={internalTimestep}
        on:input={handleTimestepChange}
        class="timestep-slider"
        aria-label="Timestep slider"
      />
      <div class="slider-labels">
        <span>{timestepRange.min}</span>
        <span>{timestepRange.max}</span>
      </div>
    </div>
  </div>

  <div class="parameter-display">
    <div class="parameter-grid">
      <div class="parameter-item">
        <span class="parameter-label">Feed Rate (F):</span>
        <span class="parameter-value">{feedRate.toFixed(3)}</span>
      </div>
      <div class="parameter-item">
        <span class="parameter-label">Kill Rate (K):</span>
        <span class="parameter-value">{killRate.toFixed(3)}</span>
      </div>
      <div class="parameter-item">
        <span class="parameter-label">Diffusion U (Du):</span>
        <span class="parameter-value">{diffusionRateU.toFixed(3)}</span>
      </div>
      <div class="parameter-item">
        <span class="parameter-label">Diffusion V (Dv):</span>
        <span class="parameter-value">{diffusionRateV.toFixed(3)}</span>
      </div>
      <div class="parameter-item">
        <span class="parameter-label">Timestep (Δt):</span>
        <span class="parameter-value">{internalTimestep.toFixed(1)}</span>
      </div>
    </div>
  </div>

  <div class="controls-info">
    <div class="controls-header">
      <h4>Interactive Controls:</h4>
    </div>
    <ul>
      <li><strong>🔴 Feed/Kill Plot:</strong> Drag the red handle to adjust feed rate (X) and kill rate (Y)</li>
      <li><strong>🟢 Diffusion Plot:</strong> Drag the green handle to adjust diffusion U (X) and diffusion V (Y)</li>
      <li><strong>🟣 Timestep Slider:</strong> Use the slider to adjust simulation speed</li>
    </ul>
    <div class="equation-note">
      <small><strong>Equations:</strong> ∂u/∂t = Du∇²u - uv² + F(1-u) | ∂v/∂t = Dv∇²v + uv² - (K+F)v</small>
    </div>
  </div>
</div>

<style>
  .diagram-container {
    width: 100%;
    max-width: 100%;
    margin: 0;
    padding: 0;
  }
  
  .instructions {
    margin: 0 0 15px 0;
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.9em;
    font-style: italic;
  }
  
  .top-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }
  
  .reset-button {
    background: #374151;
    color: white;
    border: 1px solid #4b5563;
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    transition: background-color 0.2s;
    white-space: nowrap;
  }
  
  .reset-button:hover {
    background: #4b5563;
  }
  
  .plots-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  
  .plot-section {
    display: flex;
    flex-direction: column;
  }
  
  .timestep-section {
    background: #1f2937;
    border: 1px solid #374151;
    border-radius: 0.5rem;
    padding: 1rem;
    margin-bottom: 1rem;
  }
  
  .timestep-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  
  .timestep-header h4 {
    margin: 0;
    color: #ffffff;
    font-size: 1rem;
  }
  
  .timestep-value {
    color: #fbbf24;
    font-weight: bold;
    font-family: monospace;
    font-size: 1rem;
  }
  
  .slider-container {
    position: relative;
  }
  
  .timestep-slider {
    width: 100%;
    height: 8px;
    border-radius: 4px;
    background: #374151;
    outline: none;
    -webkit-appearance: none;
    appearance: none;
    cursor: pointer;
  }
  
  .timestep-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #a855f7;
    border: 2px solid #9333ea;
    cursor: pointer;
  }
  
  .timestep-slider::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #a855f7;
    border: 2px solid #9333ea;
    cursor: pointer;
    border: none;
  }
  
  .slider-labels {
    display: flex;
    justify-content: space-between;
    margin-top: 0.5rem;
    color: #9ca3af;
    font-size: 0.875rem;
  }
  
  .parameter-display {
    margin: 1rem 0;
    padding: 1rem;
    background: #1f2937;
    border-radius: 0.5rem;
    border: 1px solid #374151;
  }
  
  .parameter-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.75rem;
  }
  
  .parameter-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    background: #111827;
    border-radius: 0.375rem;
    border: 1px solid #374151;
  }
  
  .parameter-label {
    color: #d1d5db;
    font-size: 0.875rem;
  }
  
  .parameter-value {
    color: #fbbf24;
    font-weight: bold;
    font-family: monospace;
    font-size: 0.875rem;
  }
  
  .controls-info {
    margin: 1rem 0;
    padding: 1rem;
    background: #1f2937;
    border-radius: 0.5rem;
    border: 1px solid #374151;
  }
  
  .controls-header h4 {
    margin: 0 0 0.75rem 0;
    color: #f9fafb;
    font-size: 1rem;
  }
  
  .controls-info ul {
    margin: 0 0 1rem 0;
    padding-left: 1.5rem;
    color: #d1d5db;
  }
  
  .controls-info li {
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
  }
  
  .equation-note {
    padding: 0.75rem;
    background: #111827;
    border-radius: 0.375rem;
    border-left: 3px solid #3b82f6;
    color: #9ca3af;
    font-size: 0.8rem;
    line-height: 1.4;
    word-break: break-word;
  }
  
  /* Mobile responsive styles */
  @media (max-width: 768px) {
    .top-controls {
      flex-direction: column;
      align-items: stretch;
      gap: 0.5rem;
    }
    
    .instructions {
      font-size: 0.8em;
      text-align: center;
    }
    
    .plots-container {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
    
    .timestep-section {
      padding: 0.75rem;
    }
    
    .timestep-header h4 {
      font-size: 0.9rem;
    }
    
    .timestep-value {
      font-size: 0.9rem;
    }
    
    .parameter-grid {
      grid-template-columns: 1fr;
      gap: 0.5rem;
    }
    
    .parameter-item {
      padding: 0.75rem;
    }
    
    .controls-info {
      padding: 0.75rem;
    }
    
    .controls-info li {
      font-size: 0.8rem;
    }
    
    .equation-note {
      font-size: 0.75rem;
    }
  }
</style> 