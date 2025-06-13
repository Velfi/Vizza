<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

  // Simulation state
  let settings = {
    // Pheromone Settings
    pheromone_decay_rate: 1.0,
    pheromone_deposition_rate: 0.5,
    pheromone_diffusion_rate: 0.5,
    decay_frequency: 1,
    diffusion_frequency: 1,

    // Agent Settings
    agent_count: 1_000_000,
    agent_speed_min: 100,
    agent_speed_max: 200,
    agent_turn_rate: Math.PI, // 180 degrees
    agent_jitter: 0.1,
    agent_sensor_angle: Math.PI / 4, // 45 degrees
    agent_sensor_distance: 50,

    // Gradient Settings
    gradient_type: 'disabled',
    gradient_strength: 50,
    gradient_center_x: 0.5,
    gradient_center_y: 0.5,
    gradient_size: 1.0,
    gradient_angle: 0,

    // Display Settings
    fps_limit: 60,
    fps_limit_enabled: false,
    lut_index: 0,
    lut_reversed: false
  };

  // Preset and LUT state
  let current_preset = 'default';
  let available_presets = ['default', 'dense', 'sparse', 'fast', 'slow'];
  let available_luts = [
    'MATPLOTLIB_bone_r',
    'MATPLOTLIB_viridis',
    'MATPLOTLIB_plasma',
    'MATPLOTLIB_magma',
    'MATPLOTLIB_inferno'
  ];

  // Dialog state
  let show_save_preset_dialog = false;
  let show_gradient_editor = false;
  let new_preset_name = '';
  let custom_lut_name = '';

  // Helper function to convert agent count to millions
  const toMillions = (count: number) => count / 1_000_000;
  const fromMillions = (millions: number) => millions * 1_000_000;

  // Computed values
  $: agent_count_millions = toMillions(settings.agent_count);
  $: gradient_center_x_percent = settings.gradient_center_x * 100;
  $: gradient_center_y_percent = settings.gradient_center_y * 100;

  // Two-way binding handlers
  function updateAgentCount(value: number) {
    settings.agent_count = fromMillions(value);
    dispatch('command', { type: 'SetAgentCount', value: settings.agent_count });
  }

  function updateGradientCenterX(value: number) {
    settings.gradient_center_x = value / 100;
    dispatch('command', { type: 'SetSetting', value: { name: 'gradient_center_x', value: settings.gradient_center_x } });
  }

  function updateGradientCenterY(value: number) {
    settings.gradient_center_y = value / 100;
    dispatch('command', { type: 'SetSetting', value: { name: 'gradient_center_y', value: settings.gradient_center_y } });
  }

  function updateTurnRate(value: number) {
    settings.agent_turn_rate = value;
    dispatch('command', { type: 'SetSetting', value: { name: 'agent_turn_rate', value: settings.agent_turn_rate } });
  }

  function updateSensorAngle(value: number) {
    settings.agent_sensor_angle = value;
    dispatch('command', { type: 'SetSetting', value: { name: 'agent_sensor_angle', value: settings.agent_sensor_angle } });
  }

  function updateFpsLimit(value: number) {
    settings.fps_limit = value;
    dispatch('command', { type: 'SetFpsLimit', value: settings.fps_limit });
  }

  function updateFpsLimitEnabled(value: boolean) {
    settings.fps_limit_enabled = value;
    dispatch('command', { type: 'SetFpsLimitEnabled', value: settings.fps_limit_enabled });
  }

  function updateLutIndex(value: number) {
    settings.lut_index = value;
    dispatch('command', { type: 'SetLutIndex', value: settings.lut_index });
  }

  function updateLutReversed(value: boolean) {
    settings.lut_reversed = value;
    dispatch('command', { type: 'SetLutReversed' });
  }

  function updatePreset(value: string) {
    current_preset = value;
    dispatch('command', { type: 'ApplyPreset', value: current_preset });
  }

  function cyclePresetBack() {
    dispatch('command', { type: 'CyclePresetBack' });
  }

  function cyclePresetForward() {
    dispatch('command', { type: 'CyclePresetForward' });
  }

  function cycleLutBack() {
    dispatch('command', { type: 'CycleLutBack' });
  }

  function cycleLutForward() {
    dispatch('command', { type: 'CycleLutForward' });
  }

  function savePreset() {
    dispatch('command', { type: 'SavePreset', value: new_preset_name });
    show_save_preset_dialog = false;
    new_preset_name = '';
  }

  function deletePreset() {
    dispatch('command', { type: 'DeletePreset', value: current_preset });
  }

  function showGradientEditor() {
    show_gradient_editor = true;
    dispatch('command', { type: 'ShowGradientEditor', value: true });
  }

  function saveCustomLut() {
    dispatch('command', { type: 'SaveCustomLut', value: { name: custom_lut_name, data: [] } }); // TODO: Add LUT data
    show_gradient_editor = false;
    custom_lut_name = '';
  }

  function setNewPresetName(value: string) {
    new_preset_name = value;
    dispatch('command', { type: 'SetNewPresetName', value: new_preset_name });
  }

  function setCustomLutName(value: string) {
    custom_lut_name = value;
    dispatch('command', { type: 'SetCustomLutName', value: custom_lut_name });
  }

  // Input values
  let agentCountInput = agent_count_millions;
  let gradientCenterXInput = gradient_center_x_percent;
  let gradientCenterYInput = gradient_center_y_percent;

  // Update handlers
  $: if (agentCountInput !== undefined) updateAgentCount(agentCountInput);
  $: if (gradientCenterXInput !== undefined) updateGradientCenterX(gradientCenterXInput);
  $: if (gradientCenterYInput !== undefined) updateGradientCenterY(gradientCenterYInput);

  let running = false;
  let loading = false;
  let renderLoopId: number | null = null;

  async function startSimulation() {
    if (running || loading) return;
    
    loading = true;
    
    async function renderLoop() {
      if (!running) return;
      try {
        await invoke('render_frame');
      } catch (e) {
        console.error(e);
      }
      renderLoopId = requestAnimationFrame(renderLoop);
    }

    try {
      await invoke('start_slime_mold_simulation');
      loading = false;
      running = true;
      renderLoop();
    } catch (e) {
      console.error('Failed to start simulation:', e);
      loading = false;
      running = false;
    }
  }

  async function stopSimulation() {
    running = false;
    if (renderLoopId !== null) {
      cancelAnimationFrame(renderLoopId);
      renderLoopId = null;
    }
    
    try {
      // Stop the backend simulation
      await invoke('stop_simulation');
      
      // Immediately render a frame to show the triangle instead of last simulation frame
      await invoke('render_frame');
    } catch (e) {
      console.error('Failed to stop simulation:', e);
    }
  }

  function returnToMenu() {
    stopSimulation();
    dispatch('back');
  }

  onMount(() => {
    startSimulation();
    
    return () => {
      stopSimulation();
    };
  });
</script>

<div class="slime-mold-container">
  <!-- Loading Screen -->
  {#if loading}
    <div class="loading-overlay">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <h2>Starting Simulation...</h2>
        <p>Initializing GPU resources and agents</p>
      </div>
    </div>
  {/if}

  <div class="controls">
    <button class="back-button" on:click={returnToMenu}>
      ← Back to Menu
    </button>
    
    <div class="status">
      <span class="status-indicator" class:running></span>
      Slime Mold Simulation {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
    </div>
  </div>

  <!-- Simulation Controls -->
  <div class="simulation-controls">
    <form on:submit|preventDefault>
      <!-- Pheromone Settings -->
      <fieldset>
        <legend>Pheromone Settings</legend>
        
        <div class="control-group">
          <label for="decayRate">Decay Rate (%)</label>
          <input 
            type="range" 
            id="decayRate" 
            min="0" 
            max="10" 
            step="0.1" 
            bind:value={settings.pheromone_decay_rate}
            on:input={(e: Event) => console.log('Decay Rate:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="depositionRate">Deposition Rate (%)</label>
          <input 
            type="range" 
            id="depositionRate" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={settings.pheromone_deposition_rate}
            on:input={(e: Event) => console.log('Deposition Rate:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="diffusionRate">Diffusion Rate (%)</label>
          <input 
            type="range" 
            id="diffusionRate" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={settings.pheromone_diffusion_rate}
            on:input={(e: Event) => console.log('Diffusion Rate:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="decayFrequency">Decay Frequency</label>
          <input 
            type="number" 
            id="decayFrequency" 
            min="1" 
            bind:value={settings.decay_frequency}
            on:input={(e: Event) => console.log('Decay Frequency:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="diffusionFrequency">Diffusion Frequency</label>
          <input 
            type="number" 
            id="diffusionFrequency" 
            min="1" 
            bind:value={settings.diffusion_frequency}
            on:input={(e: Event) => console.log('Diffusion Frequency:', (e.target as HTMLInputElement).value)}
          />
        </div>
      </fieldset>

      <!-- Agent Settings -->
      <fieldset>
        <legend>Agent Settings</legend>
        
        <div class="control-group">
          <label for="agentCount">Agent Count (millions)</label>
          <input 
            type="number" 
            id="agentCount" 
            min="0" 
            max="100" 
            step="0.1" 
            bind:value={agentCountInput}
          />
        </div>

        <div class="control-group">
          <label for="minSpeed">Min Speed</label>
          <input 
            type="range" 
            id="minSpeed" 
            min="0" 
            max="500" 
            step="0.1" 
            bind:value={settings.agent_speed_min}
            on:input={(e: Event) => console.log('Min Speed:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="maxSpeed">Max Speed</label>
          <input 
            type="range" 
            id="maxSpeed" 
            min="0" 
            max="500" 
            step="0.1" 
            bind:value={settings.agent_speed_max}
            on:input={(e: Event) => console.log('Max Speed:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="turnRate">Turn Rate (degrees)</label>
          <input 
            type="range" 
            id="turnRate" 
            min="0" 
            max="360" 
            step="1" 
            bind:value={settings.agent_turn_rate}
            on:input={(e: Event) => {
              const rads = parseFloat((e.target as HTMLInputElement).value);
              updateTurnRate(rads);
            }}
          />
        </div>

        <div class="control-group">
          <label for="jitter">Jitter</label>
          <input 
            type="range" 
            id="jitter" 
            min="0" 
            max="5" 
            step="0.01" 
            bind:value={settings.agent_jitter}
            on:input={(e: Event) => console.log('Jitter:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="sensorAngle">Sensor Angle (degrees)</label>
          <input 
            type="range" 
            id="sensorAngle" 
            min="0" 
            max="180" 
            step="1" 
            bind:value={settings.agent_sensor_angle}
            on:input={(e: Event) => {
              const rads = parseFloat((e.target as HTMLInputElement).value);
              updateSensorAngle(rads);
            }}
          />
        </div>

        <div class="control-group">
          <label for="sensorDistance">Sensor Distance</label>
          <input 
            type="range" 
            id="sensorDistance" 
            min="0" 
            max="500" 
            step="1" 
            bind:value={settings.agent_sensor_distance}
            on:input={(e: Event) => console.log('Sensor Distance:', (e.target as HTMLInputElement).value)}
          />
        </div>
      </fieldset>

      <!-- Gradient Settings -->
      <fieldset>
        <legend>Gradient Settings</legend>
        
        <div class="control-group">
          <label for="gradientType">Gradient Type</label>
          <select 
            id="gradientType"
            bind:value={settings.gradient_type}
            on:change={(e: Event) => console.log('Gradient Type:', (e.target as HTMLSelectElement).value)}
          >
            <option value="disabled">Disabled</option>
            <option value="radial">Radial</option>
            <option value="linear">Linear</option>
            <option value="spiral">Spiral</option>
          </select>
        </div>

        <div class="control-group">
          <label for="gradientStrength">Gradient Strength</label>
          <input 
            type="range" 
            id="gradientStrength" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={settings.gradient_strength}
            on:input={(e: Event) => console.log('Gradient Strength:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="gradientCenterX">Center X (%)</label>
          <input 
            type="range" 
            id="gradientCenterX" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={gradientCenterXInput}
          />
        </div>

        <div class="control-group">
          <label for="gradientCenterY">Center Y (%)</label>
          <input 
            type="range" 
            id="gradientCenterY" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={gradientCenterYInput}
          />
        </div>

        <div class="control-group">
          <label for="gradientSize">Size</label>
          <input 
            type="range" 
            id="gradientSize" 
            min="0.1" 
            max="2" 
            step="0.01" 
            bind:value={settings.gradient_size}
            on:input={(e: Event) => console.log('Gradient Size:', (e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="control-group">
          <label for="gradientAngle">Angle (degrees)</label>
          <input 
            type="range" 
            id="gradientAngle" 
            min="0" 
            max="360" 
            step="1" 
            bind:value={settings.gradient_angle}
            on:input={(e: Event) => console.log('Gradient Angle:', (e.target as HTMLInputElement).value)}
          />
        </div>
      </fieldset>

      <!-- Display Settings -->
      <fieldset>
        <legend>Display Settings</legend>
        
        <div class="control-group">
          <label for="fpsLimit">FPS Limit</label>
          <input 
            type="number" 
            id="fpsLimit" 
            min="1" 
            max="1200" 
            step="1" 
            bind:value={settings.fps_limit}
            on:input={(e: Event) => {
              const value = parseFloat((e.target as HTMLInputElement).value);
              updateFpsLimit(value);
            }}
          />
        </div>

        <div class="control-group">
          <label for="fpsLimitEnabled">Enable FPS Limit</label>
          <input 
            type="checkbox" 
            id="fpsLimitEnabled"
            bind:checked={settings.fps_limit_enabled}
            on:change={(e: Event) => {
              const value = (e.target as HTMLInputElement).checked;
              updateFpsLimitEnabled(value);
            }}
          />
        </div>

        <div class="control-group">
          <label for="lutIndex">Color Scheme</label>
          <select 
            id="lutIndex"
            bind:value={settings.lut_index}
            on:change={(e: Event) => {
              const value = parseInt((e.target as HTMLSelectElement).value);
              updateLutIndex(value);
            }}
          >
            {#each available_luts as lut, i}
              <option value={i}>{lut}</option>
            {/each}
          </select>
        </div>

        <div class="control-group">
          <label for="lutReversed">Reverse Colors</label>
          <input 
            type="checkbox" 
            id="lutReversed"
            bind:checked={settings.lut_reversed}
            on:change={(e: Event) => {
              const value = (e.target as HTMLInputElement).checked;
              updateLutReversed(value);
            }}
          />
        </div>
      </fieldset>

      <!-- Preset Controls -->
      <fieldset>
        <legend>Preset Controls</legend>
        
        <div class="control-group">
          <label for="presetSelector">Current Preset</label>
          <div class="preset-controls">
            <button 
              type="button"
              on:click={cyclePresetBack}
            >
              ◀
            </button>
            <select 
              id="presetSelector"
              bind:value={current_preset}
              on:change={(e: Event) => {
                const value = (e.target as HTMLSelectElement).value;
                updatePreset(value);
              }}
            >
              {#each available_presets as preset}
                <option value={preset}>{preset}</option>
              {/each}
            </select>
            <button 
              type="button"
              on:click={cyclePresetForward}
            >
              ▶
            </button>
          </div>
        </div>

        <div class="control-group preset-actions">
          <button 
            type="button"
            on:click={() => show_save_preset_dialog = true}
          >
            💾 Save Current
          </button>
          <button 
            type="button"
            on:click={deletePreset}
          >
            🗑 Delete
          </button>
        </div>

        <!-- Save Preset Dialog -->
        {#if show_save_preset_dialog}
          <div class="save-preset-dialog">
            <div class="dialog-content">
              <h3>Save Preset</h3>
              <div class="control-group">
                <label for="newPresetName">Preset Name</label>
                <input 
                  type="text" 
                  id="newPresetName"
                  bind:value={new_preset_name}
                  on:input={(e: Event) => {
                    const value = (e.target as HTMLInputElement).value;
                    setNewPresetName(value);
                  }}
                />
              </div>
              <div class="dialog-actions">
                <button 
                  type="button"
                  on:click={savePreset}
                >
                  Save
                </button>
                <button 
                  type="button"
                  on:click={() => {
                    show_save_preset_dialog = false;
                    new_preset_name = '';
                  }}
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        {/if}
      </fieldset>

      <!-- LUT Controls -->
      <fieldset>
        <legend>LUT Controls</legend>
        
        <div class="control-group">
          <label for="lutSelector">Current LUT</label>
          <div class="lut-controls">
            <button 
              type="button"
              on:click={cycleLutBack}
            >
              ◀
            </button>
            <select 
              id="lutSelector"
              bind:value={settings.lut_index}
              on:change={(e: Event) => {
                const value = parseInt((e.target as HTMLSelectElement).value);
                updateLutIndex(value);
              }}
            >
              {#each available_luts as lut, i}
                <option value={i}>{lut}</option>
              {/each}
            </select>
            <button 
              type="button"
              on:click={cycleLutForward}
            >
              ▶
            </button>
          </div>
        </div>

        <div class="control-group">
          <button 
            type="button"
            on:click={showGradientEditor}
          >
            🎨 Create Custom LUT
          </button>
        </div>

        <!-- Gradient Editor Dialog -->
        {#if show_gradient_editor}
          <div class="gradient-editor-dialog">
            <div class="dialog-content">
              <h3>Gradient Editor</h3>
              <div class="control-group">
                <label for="customLutName">LUT Name</label>
                <input 
                  type="text" 
                  id="customLutName"
                  bind:value={custom_lut_name}
                  on:input={(e: Event) => {
                    const value = (e.target as HTMLInputElement).value;
                    setCustomLutName(value);
                  }}
                />
              </div>
              <!-- Gradient editor canvas would go here -->
              <div class="dialog-actions">
                <button 
                  type="button"
                  on:click={saveCustomLut}
                >
                  Save
                </button>
                <button 
                  type="button"
                  on:click={() => {
                    show_gradient_editor = false;
                    custom_lut_name = '';
                  }}
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        {/if}
      </fieldset>
    </form>
  </div>
</div>

<style>
  .slime-mold-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
  }

  .controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .back-button {
    padding: 0.5rem 1rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-family: inherit;
    transition: all 0.3s ease;
  }

  .back-button:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ff6b6b;
    transition: background-color 0.3s ease;
  }

  .status-indicator.running {
    background: #51cf66;
  }

  .simulation-controls {
    padding: 1rem;
    max-width: 800px;
    margin: 0 auto;
  }

  fieldset {
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 1rem;
    margin-bottom: 1rem;
  }

  legend {
    font-weight: bold;
    padding: 0 0.5rem;
  }

  .control-group {
    margin-bottom: 1rem;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
  }

  input[type="range"],
  input[type="number"],
  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
  }

  input[type="checkbox"] {
    margin-right: 0.5rem;
  }

  .preset-controls,
  .lut-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .preset-controls select,
  .lut-controls select {
    flex: 1;
  }

  .preset-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .save-preset-dialog,
  .gradient-editor-dialog {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dialog-content {
    background: white;
    padding: 1rem;
    border-radius: 4px;
    min-width: 300px;
  }

  .dialog-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }

  button {
    padding: 0.5rem 1rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: #f8f9fa;
    cursor: pointer;
  }

  button:hover {
    background: #e9ecef;
  }

  /* Loading Screen Styles */
  .loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: black;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .loading-content {
    text-align: center;
    color: white;
    padding: 2rem;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .loading-content h2 {
    margin: 1rem 0 0.5rem 0;
    font-size: 1.5rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .loading-content p {
    margin: 0;
    color: rgba(255, 255, 255, 0.7);
    font-size: 1rem;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid #646cff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>