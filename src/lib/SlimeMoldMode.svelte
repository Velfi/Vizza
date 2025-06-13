<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

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
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

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
  async function updateAgentCount(value: number) {
    settings.agent_count = fromMillions(value);
    try {
      await invoke('update_agent_count', { count: settings.agent_count });
    } catch (e) {
      console.error('Failed to update agent count:', e);
    }
  }

  async function updateGradientCenterX(value: number) {
    settings.gradient_center_x = value / 100;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_center_x', 
        value: settings.gradient_center_x 
      });
    } catch (e) {
      console.error('Failed to update gradient center X:', e);
    }
  }

  async function updateGradientCenterY(value: number) {
    settings.gradient_center_y = value / 100;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_center_y', 
        value: settings.gradient_center_y 
      });
    } catch (e) {
      console.error('Failed to update gradient center Y:', e);
    }
  }

  async function updateTurnRate(value: number) {
    settings.agent_turn_rate = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_turn_rate', 
        value: settings.agent_turn_rate 
      });
    } catch (e) {
      console.error('Failed to update turn rate:', e);
    }
  }

  async function updateSensorAngle(value: number) {
    settings.agent_sensor_angle = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_sensor_angle', 
        value: settings.agent_sensor_angle 
      });
    } catch (e) {
      console.error('Failed to update sensor angle:', e);
    }
  }

  async function updateFpsLimit(value: number) {
    settings.fps_limit = value;
    try {
      await invoke('set_fps_limit', { 
        enabled: settings.fps_limit_enabled, 
        limit: settings.fps_limit 
      });
      console.log(`FPS limit set to: ${value}`);
    } catch (e) {
      console.error('Failed to update FPS limit:', e);
    }
  }

  async function updateFpsLimitEnabled(value: boolean) {
    settings.fps_limit_enabled = value;
    try {
      await invoke('set_fps_limit', { 
        enabled: settings.fps_limit_enabled, 
        limit: settings.fps_limit 
      });
      console.log(`FPS limiting ${value ? 'enabled' : 'disabled'}`);
    } catch (e) {
      console.error('Failed to update FPS limit enabled:', e);
    }
  }

  async function updateLutIndex(value: number) {
    settings.lut_index = value;
    try {
      await invoke('apply_lut_by_index', { lutIndex: value });
    } catch (e) {
      console.error('Failed to update LUT index:', e);
    }
  }

  async function updateLutReversed(value: boolean) {
    settings.lut_reversed = value;
    try {
      await invoke('toggle_lut_reversed');
    } catch (e) {
      console.error('Failed to toggle LUT reversed:', e);
    }
  }

  // Helper functions for direct input event handlers
  async function handlePheromoneDecayRate(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.pheromone_decay_rate = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'pheromone_decay_rate', 
        value: settings.pheromone_decay_rate 
      });
    } catch (err) {
      console.error('Failed to update pheromone decay rate:', err);
    }
  }

  async function handlePheromoneDepositionRate(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.pheromone_deposition_rate = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'pheromone_deposition_rate', 
        value: settings.pheromone_deposition_rate 
      });
    } catch (err) {
      console.error('Failed to update pheromone deposition rate:', err);
    }
  }

  async function handlePheromoneDiffusionRate(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.pheromone_diffusion_rate = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'pheromone_diffusion_rate', 
        value: settings.pheromone_diffusion_rate 
      });
    } catch (err) {
      console.error('Failed to update pheromone diffusion rate:', err);
    }
  }

  async function handleAgentSpeedMin(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.agent_speed_min = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_speed_min', 
        value: settings.agent_speed_min 
      });
    } catch (err) {
      console.error('Failed to update min speed:', err);
    }
  }

  async function handleAgentSpeedMax(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.agent_speed_max = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_speed_max', 
        value: settings.agent_speed_max 
      });
    } catch (err) {
      console.error('Failed to update max speed:', err);
    }
  }

  async function handleAgentJitter(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.agent_jitter = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_jitter', 
        value: settings.agent_jitter 
      });
    } catch (err) {
      console.error('Failed to update agent jitter:', err);
    }
  }

  async function handleAgentSensorDistance(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.agent_sensor_distance = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_sensor_distance', 
        value: settings.agent_sensor_distance 
      });
    } catch (err) {
      console.error('Failed to update sensor distance:', err);
    }
  }

  async function handleGradientType(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    settings.gradient_type = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_type', 
        value: settings.gradient_type 
      });
    } catch (err) {
      console.error('Failed to update gradient type:', err);
    }
  }

  async function handleGradientStrength(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.gradient_strength = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_strength', 
        value: settings.gradient_strength 
      });
    } catch (err) {
      console.error('Failed to update gradient strength:', err);
    }
  }

  async function handleGradientSize(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.gradient_size = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_size', 
        value: settings.gradient_size 
      });
    } catch (err) {
      console.error('Failed to update gradient size:', err);
    }
  }

  async function handleGradientAngle(e: Event) {
    const value = parseFloat((e.target as HTMLInputElement).value);
    settings.gradient_angle = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_angle', 
        value: settings.gradient_angle 
      });
    } catch (err) {
      console.error('Failed to update gradient angle:', err);
    }
  }

  async function updatePreset(value: string) {
    current_preset = value;
    try {
      await invoke('apply_preset', { presetName: value });
      await invoke('reset_trails'); // Clear all existing trails
      await invoke('reset_agents'); // Reset agents to new positions with preset settings
      await syncSettingsFromBackend(); // Sync UI with new settings
      console.log(`Applied preset: ${value}`);
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
  }

  async function cyclePresetBack() {
    const currentIndex = available_presets.indexOf(current_preset);
    const newIndex = currentIndex > 0 ? currentIndex - 1 : available_presets.length - 1;
    const newPreset = available_presets[newIndex];
    await updatePreset(newPreset);
  }

  async function cyclePresetForward() {
    const currentIndex = available_presets.indexOf(current_preset);
    const newIndex = currentIndex < available_presets.length - 1 ? currentIndex + 1 : 0;
    const newPreset = available_presets[newIndex];
    await updatePreset(newPreset);
  }

  async function cycleLutBack() {
    const newIndex = settings.lut_index > 0 ? settings.lut_index - 1 : available_luts.length - 1;
    await updateLutIndex(newIndex);
  }

  async function cycleLutForward() {
    const newIndex = settings.lut_index < available_luts.length - 1 ? settings.lut_index + 1 : 0;
    await updateLutIndex(newIndex);
  }

  async function savePreset() {
    try {
      await invoke('save_preset', { presetName: new_preset_name });
      show_save_preset_dialog = false;
      new_preset_name = '';
      // Refresh the available presets list
      await loadAvailablePresets();
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function deletePreset() {
    try {
      await invoke('delete_preset', { presetName: current_preset });
      // Refresh the available presets list
      await loadAvailablePresets();
      // Reset to first available preset
      if (available_presets.length > 0) {
        current_preset = available_presets[0];
        await updatePreset(current_preset);
      }
    } catch (e) {
      console.error('Failed to delete preset:', e);
    }
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
  
  // FPS tracking (now received from backend)
  let currentFps = 0;
  
  // UI visibility toggle
  let showUI = true;

  async function startSimulation() {
    if (running || loading) return;
    
    loading = true;

    try {
      await invoke('start_slime_mold_simulation');
      loading = false;
      running = true;
      
      // Backend now handles the render loop, we just track state
      currentFps = 0;
    } catch (e) {
      console.error('Failed to start simulation:', e);
      loading = false;
      running = false;
    }
  }

  async function stopSimulation() {
    running = false;
    
    try {
      // Stop the backend simulation and render loop
      await invoke('stop_simulation');
      
      // Reset window title
      const { appWindow } = await import('@tauri-apps/api/window');
      await appWindow.setTitle('Slime Mold Simulation');
      
      // Reset FPS
      currentFps = 0;
      
      // Immediately render a frame to show the triangle instead of last simulation frame
      await invoke('render_frame');
    } catch (e) {
      console.error('Failed to stop simulation:', e);
    }
  }

  async function returnToMenu() {
    await stopSimulation();
    
    // Reset window title when returning to menu
    try {
      const { appWindow } = await import('@tauri-apps/api/window');
      await appWindow.setTitle('Sim-Pix');
    } catch (e) {
      console.error('Failed to reset window title:', e);
    }
    
    dispatch('back');
  }

  // Load available presets from backend
  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_available_presets');
      if (available_presets.length > 0 && !current_preset) {
        current_preset = available_presets[0];
      }
    } catch (e) {
      console.error('Failed to load available presets:', e);
    }
  }

  // Load available LUTs from backend
  async function loadAvailableLuts() {
    try {
      available_luts = await invoke('get_available_luts');
    } catch (e) {
      console.error('Failed to load available LUTs:', e);
    }
  }

  // Sync settings from backend to frontend
  async function syncSettingsFromBackend() {
    try {
      const currentSettings = await invoke('get_current_settings');
      if (currentSettings) {
        // Handle gradient type conversion from enum to lowercase string
        if (currentSettings.gradient_type) {
          currentSettings.gradient_type = currentSettings.gradient_type.toLowerCase();
        }
        
        // Update the settings object with current backend values
        settings = {
          ...settings,
          ...currentSettings
        };
        
        // Update computed values
        agentCountInput = toMillions(settings.agent_count);
        gradientCenterXInput = settings.gradient_center_x * 100;
        gradientCenterYInput = settings.gradient_center_y * 100;
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  let simulationInitializedUnlisten: (() => void) | null = null;
  let fpsUpdateUnlisten: (() => void) | null = null;

  // Keyboard event handler
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === '/') {
      event.preventDefault();
      showUI = !showUI;
    } else if (event.key === 'r' || event.key === 'R') {
      event.preventDefault();
      randomizeSimulation();
    }
  }

  async function randomizeSimulation() {
    try {
      await invoke('randomize_settings');
      await syncSettingsFromBackend();
      console.log('Settings randomized via keyboard shortcut');
    } catch (e) {
      console.error('Failed to randomize settings:', e);
    }
  }

  onMount(async () => {
    // Load presets and LUTs first
    await loadAvailablePresets();
    await loadAvailableLuts();
    
    // Add keyboard event listener
    window.addEventListener('keydown', handleKeydown);
    
    // Listen for simulation initialization event
    simulationInitializedUnlisten = await listen('simulation-initialized', async () => {
      console.log('Simulation initialized, syncing settings...');
      await syncSettingsFromBackend();
    });

    // Listen for FPS updates from backend
    fpsUpdateUnlisten = await listen('fps-update', (event) => {
      currentFps = event.payload as number;
      
      // Update window title with FPS
      (async () => {
        try {
          const { appWindow } = await import('@tauri-apps/api/window');
          await appWindow.setTitle(`Slime Mold Simulation - ${currentFps} FPS`);
        } catch (e) {
          console.error('Failed to update window title:', e);
        }
      })();
    });
    
    // Then start simulation
    startSimulation();
    
    return () => {
      stopSimulation();
    };
  });

  onDestroy(() => {
    // Remove keyboard event listener
    window.removeEventListener('keydown', handleKeydown);
    
    if (simulationInitializedUnlisten) {
      simulationInitializedUnlisten();
    }
    if (fpsUpdateUnlisten) {
      fpsUpdateUnlisten();
    }
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

  {#if showUI}
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
      <!-- 1. FPS Display & Limiter -->
      <fieldset>
        <legend>FPS & Display</legend>
        <div class="control-group">
          <label>FPS:</label>
          <span>{currentFps}</span>
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
        {#if settings.fps_limit_enabled}
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
        {/if}
      </fieldset>

      <!-- 2. Preset Controls -->
      <fieldset>
        <legend>Presets</legend>
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
          <!-- TODO: Implement preset deletion -->
          <!-- <button 
            type="button"
            on:click={deletePreset}
          >
            🗑 Delete
          </button> -->
        </div>
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

      <!-- 3. LUT Controls (Color Scheme) -->
      <fieldset>
        <legend>Color Scheme</legend>
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
        <div class="control-group">
          <button 
            type="button"
            on:click={showGradientEditor}
          >
            🎨 Create Custom LUT
          </button>
        </div>
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

      <!-- 4. Controls (Pause/Resume, Reset Trails, Reset Agents, Randomize) -->
      <fieldset>
        <legend>Controls</legend>
        <div class="control-group">
          <button type="button" on:click={startSimulation} disabled={running}>▶ Resume</button>
          <button type="button" on:click={stopSimulation} disabled={!running}>⏸ Pause</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('reset_trails');
              console.log('Trails reset successfully');
            } catch (e) {
              console.error('Failed to reset trails:', e);
            }
          }}>Reset Trails</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('reset_agents');
              console.log('Agents reset successfully');
            } catch (e) {
              console.error('Failed to reset agents:', e);
            }
          }}>Reset Agents</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('randomize_settings');
              await syncSettingsFromBackend(); // Sync UI with new random settings
              console.log('Settings randomized successfully');
            } catch (e) {
              console.error('Failed to randomize settings:', e);
            }
          }}>🎲 Randomize Settings</button>
        </div>
      </fieldset>

      <!-- 5. Pheromone Settings -->
      <fieldset>
        <legend>Pheromone Settings</legend>
        <div class="control-group">
          <label for="decayRate">Decay Rate (%)</label>
          <input 
            type="number" 
            id="decayRate" 
            min="0" 
            max="10" 
            step="0.1" 
            bind:value={settings.pheromone_decay_rate}
            on:input={handlePheromoneDecayRate}
          />
        </div>
        <div class="control-group">
          <label for="depositionRate">Deposition Rate (%)</label>
          <input 
            type="number" 
            id="depositionRate" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={settings.pheromone_deposition_rate}
            on:input={handlePheromoneDepositionRate}
          />
        </div>
        <div class="control-group">
          <label for="diffusionRate">Diffusion Rate (%)</label>
          <input 
            type="number" 
            id="diffusionRate" 
            min="0" 
            max="100" 
            step="1" 
            bind:value={settings.pheromone_diffusion_rate}
            on:input={handlePheromoneDiffusionRate}
          />
        </div>
      </fieldset>

      <!-- 6. Agent Settings -->
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
            type="number" 
            id="minSpeed" 
            min="0" 
            max="500" 
            step="0.1" 
            bind:value={settings.agent_speed_min}
            on:input={handleAgentSpeedMin}
          />
        </div>
        <div class="control-group">
          <label for="maxSpeed">Max Speed</label>
          <input 
            type="number" 
            id="maxSpeed" 
            min="0" 
            max="500" 
            step="0.1" 
            bind:value={settings.agent_speed_max}
            on:input={handleAgentSpeedMax}
          />
        </div>
        <div class="control-group">
          <label for="turnRate">Turn Rate (degrees)</label>
          <input 
            type="number" 
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
            type="number" 
            id="jitter" 
            min="0" 
            max="5" 
            step="0.01" 
            bind:value={settings.agent_jitter}
            on:input={handleAgentJitter}
          />
        </div>
        <div class="control-group">
          <label for="sensorAngle">Sensor Angle (degrees)</label>
          <input 
            type="number" 
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
            type="number" 
            id="sensorDistance" 
            min="0" 
            max="500" 
            step="1" 
            bind:value={settings.agent_sensor_distance}
            on:input={handleAgentSensorDistance}
          />
        </div>
      </fieldset>

      <!-- 7. Gradient Settings -->
      <fieldset>
        <legend>Gradient Settings</legend>
        <div class="control-group">
          <label for="gradientType">Gradient Type</label>
          <select 
            id="gradientType"
            bind:value={settings.gradient_type}
            on:change={handleGradientType}
          >
            <option value="disabled">Disabled</option>
            <option value="radial">Radial</option>
            <option value="linear">Linear</option>
            <option value="spiral">Spiral</option>
          </select>
        </div>
        {#if settings.gradient_type !== 'disabled'}
          <div class="control-group">
            <label for="gradientStrength">Gradient Strength</label>
            <input 
              type="number" 
              id="gradientStrength" 
              min="0" 
              max="100" 
              step="1" 
              bind:value={settings.gradient_strength}
              on:input={handleGradientStrength}
            />
          </div>
          <div class="control-group">
            <label for="gradientCenterX">Center X (%)</label>
            <input 
              type="number" 
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
              type="number" 
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
              type="number" 
              id="gradientSize" 
              min="0.1" 
              max="2" 
              step="0.01" 
              bind:value={settings.gradient_size}
              on:input={handleGradientSize}
            />
          </div>
          <div class="control-group">
            <label for="gradientAngle">Angle (degrees)</label>
            <input 
              type="number" 
              id="gradientAngle" 
              min="0" 
              max="360" 
              step="1" 
              bind:value={settings.gradient_angle}
              on:input={handleGradientAngle}
            />
          </div>
        {/if}
      </fieldset>
    </form>
    </div>
  {/if}
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
    background: rgba(0, 0, 0, 1.0);

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