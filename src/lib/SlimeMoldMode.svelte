<SimulationLayout
  simulationName="Slime Mold"
  {running}
  {loading}
  {showUI}
  {currentFps}
  {controlsVisible}
  {menuPosition}
  on:back={returnToMenu}
  on:toggleUI={toggleBackendGui}
  on:pause={stopSimulation}
  on:resume={resumeSimulation}
  on:userInteraction={handleUserInteraction}
  on:mouseEvent={handleMouseEvent}
>
  <form on:submit|preventDefault>
    <!-- About this simulation -->
    <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
      <p>
        Slime Mold simulates the fascinating behavior of Physarum polycephalum, a single-celled
        organism that exhibits collective intelligence. Thousands of agents move through space,
        depositing pheromone trails that attract other agents, creating efficient networks.
      </p>
      <p>
        The simulation models how slime molds solve complex problems like finding optimal paths
        through mazes and connecting food sources. Agents sense pheromone gradients and adjust their
        movement accordingly, while the pheromone trails decay and diffuse over time.
      </p>
      <p>
        Watch as simple rules for movement and pheromone interaction lead to the emergence of
        sophisticated transportation networks, branching patterns, and adaptive pathfinding - all
        without central coordination or planning.
      </p>
    </CollapsibleFieldset>

    <!-- Preset Controls -->
    <PresetFieldset
      availablePresets={available_presets}
      bind:currentPreset={current_preset}
      placeholder="Select preset..."
      on:presetChange={({ detail }) => updatePreset(detail.value)}
      on:presetSave={({ detail }) => savePreset(detail.name)}
    />

    <!-- Display Settings -->
    <fieldset>
      <legend>Display Settings</legend>
      <div class="control-group">
        <label for="lutSelector">Color Scheme</label>
        <LutSelector
          bind:available_luts
          current_lut={lut_name}
          reversed={lut_reversed}
          on:select={({ detail }) => updateLutName(detail.name)}
          on:reverse={() => updateLutReversed()}
        />
      </div>
    </fieldset>

    <!-- Controls -->
    <fieldset>
      <legend>Controls</legend>
      <div class="interaction-controls-grid">
        <div class="interaction-help">
          <div class="control-group">
            <span>🖱️ Left click: Attract agents | Right click: Repel agents</span>
          </div>
          <div class="control-group">
            <button type="button" on:click={() => dispatch('navigate', 'how-to-play')}>
              📖 Camera Controls
            </button>
          </div>
          <div class="control-group">
            <span>Camera controls not working? Click the control bar at the top of the screen.</span
            >
          </div>
        </div>
        <div class="cursor-settings">
          <div class="cursor-settings-header">
            <span>🎯 Cursor Settings</span>
          </div>
          <CursorConfig
            {cursorSize}
            {cursorStrength}
            sizeMin={10}
            sizeMax={500}
            sizeStep={5}
            strengthMin={0}
            strengthMax={50}
            strengthStep={0.5}
            sizePrecision={0}
            strengthPrecision={1}
            on:sizechange={(e) => updateCursorSize(e.detail)}
            on:strengthchange={(e) => updateCursorStrength(e.detail)}
          />
        </div>
      </div>
    </fieldset>

    <!-- Combined Settings -->
    <fieldset>
      <legend>Settings</legend>

      <!-- General Settings -->
      <div class="settings-section">
        <div class="control-group">
          <button
            type="button"
            on:click={async () => {
              try {
                await invoke('randomize_settings');
                await syncSettingsFromBackend();
                console.log('Settings randomized successfully');
              } catch (e) {
                console.error('Failed to randomize settings:', e);
              }
            }}>🎲 Randomize Settings</button
          >
          <button
            type="button"
            on:click={async () => {
              try {
                await invoke('reset_trails');
                console.log('Trails reset successfully');
              } catch (e) {
                console.error('Failed to reset trails:', e);
              }
            }}>🧹 Clear Trails</button
          >
        </div>
        <div class="control-group">
          <label for="positionGenerator" class="visually-hidden">Agent Position Generator</label>
          <ButtonSelect
            bind:value={position_generator}
            options={[
              { value: 'Random', label: 'Random', buttonAction: 'randomize' },
              { value: 'Center', label: 'Center', buttonAction: 'randomize' },
              { value: 'UniformCircle', label: 'Uniform Circle', buttonAction: 'randomize' },
              { value: 'CenteredCircle', label: 'Centered Circle', buttonAction: 'randomize' },
              { value: 'Ring', label: 'Ring', buttonAction: 'randomize' },
              { value: 'Line', label: 'Line', buttonAction: 'randomize' },
              { value: 'Spiral', label: 'Spiral', buttonAction: 'randomize' },
            ]}
            buttonText="Reset Agents"
            placeholder="Select position generator..."
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', {
                  settingName: 'position_generator',
                  value: e.detail.value,
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update position generator:', err);
              }
            }}
            on:buttonclick={async () => {
              try {
                await invoke('reset_agents');
                await invoke('reset_trails');
                console.log('Agents randomized via ButtonSelect');
              } catch (err) {
                console.error('Failed to randomize agents:', err);
              }
            }}
          />
        </div>
      </div>

      <!-- Pheromone Settings -->
      <div class="settings-section">
        <h3 class="section-header">Pheromone</h3>
        <div class="settings-grid">
          <div class="setting-item">
            <span class="setting-label">Decay Rate:</span>
            <NumberDragBox
              bind:value={settings.pheromone_decay_rate}
              min={0}
              max={10000}
              step={1}
              precision={2}
              unit="%"
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'pheromone_decay_rate',
                    value: e.detail,
                  });
                } catch (err) {
                  console.error('Failed to update pheromone decay rate:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Deposition Rate:</span>
            <NumberDragBox
              bind:value={settings.pheromone_deposition_rate}
              min={0}
              max={100}
              step={1}
              precision={2}
              unit="%"
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'pheromone_deposition_rate',
                    value: e.detail,
                  });
                } catch (err) {
                  console.error('Failed to update pheromone deposition rate:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Diffusion Rate:</span>
            <NumberDragBox
              bind:value={settings.pheromone_diffusion_rate}
              min={0}
              max={100}
              step={1}
              precision={2}
              unit="%"
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'pheromone_diffusion_rate',
                    value: e.detail,
                  });
                } catch (err) {
                  console.error('Failed to update pheromone diffusion rate:', err);
                }
              }}
            />
          </div>
        </div>
      </div>

      <!-- Agent Settings -->
      <div class="settings-section">
        <h3 class="section-header">Agent</h3>
        <div class="settings-grid">
          <div class="setting-item">
            <span class="setting-label">Agent Count (millions):</span>
            <AgentCountInput
              value={agent_count_millions}
              min={0}
              max={100}
              on:update={async (e) => {
                try {
                  await updateAgentCount(e.detail);
                  console.log(`Agent count updated to ${e.detail} million`);
                } catch (err) {
                  console.error('Failed to update agent count:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Min Speed:</span>
            <NumberDragBox
              bind:value={settings.agent_speed_min}
              min={0}
              max={500}
              step={10}
              precision={1}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'agent_speed_min',
                    value: e.detail,
                  });
                  await syncSettingsFromBackend();
                } catch (err) {
                  console.error('Failed to update min speed:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Max Speed:</span>
            <NumberDragBox
              bind:value={settings.agent_speed_max}
              min={0}
              max={500}
              step={10}
              precision={1}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'agent_speed_max',
                    value: e.detail,
                  });
                  await syncSettingsFromBackend();
                } catch (err) {
                  console.error('Failed to update max speed:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Turn Rate:</span>
            <NumberDragBox
              bind:value={settings.agent_turn_rate}
              min={0}
              max={360}
              step={1}
              precision={0}
              unit="°"
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'agent_turn_rate',
                    value: (e.detail * Math.PI) / 180, // Convert degrees to radians
                  });
                  await syncSettingsFromBackend();
                } catch (err) {
                  console.error('Failed to update turn rate:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Jitter:</span>
            <NumberDragBox
              bind:value={settings.agent_jitter}
              min={0}
              max={5}
              step={0.01}
              precision={2}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'agent_jitter',
                    value: e.detail,
                  });
                  await syncSettingsFromBackend();
                } catch (err) {
                  console.error('Failed to update agent jitter:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Sensor Angle:</span>
            <NumberDragBox
              bind:value={settings.agent_sensor_angle}
              min={0}
              max={180}
              step={1}
              precision={0}
              unit="°"
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'agent_sensor_angle',
                    value: (e.detail * Math.PI) / 180, // Convert degrees to radians
                  });
                  await syncSettingsFromBackend();
                } catch (err) {
                  console.error('Failed to update sensor angle:', err);
                }
              }}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Sensor Distance:</span>
            <NumberDragBox
              bind:value={settings.agent_sensor_distance}
              min={0}
              max={500}
              step={1}
              precision={0}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', {
                    settingName: 'agent_sensor_distance',
                    value: e.detail,
                  });
                  await syncSettingsFromBackend();
                } catch (err) {
                  console.error('Failed to update sensor distance:', err);
                }
              }}
            />
          </div>
        </div>
      </div>

      <!-- Gradient Settings -->
      <div class="settings-section">
        <h3 class="section-header">Gradient</h3>
        <div class="settings-grid">
          <div class="setting-item">
            <span class="setting-label">Gradient Type:</span>
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
            <div class="setting-item">
              <span class="setting-label">Center X:</span>
              <input
                type="number"
                id="gradientCenterX"
                min="0"
                max="100"
                step="1"
                bind:value={gradient_center_x_percent}
                on:change={(e: Event) => {
                  const value = parseFloat((e.target as HTMLInputElement).value);
                  updateGradientCenterX(value);
                }}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Center Y:</span>
              <input
                type="number"
                id="gradientCenterY"
                min="0"
                max="100"
                step="1"
                bind:value={gradient_center_y_percent}
                on:change={(e: Event) => {
                  const value = parseFloat((e.target as HTMLInputElement).value);
                  updateGradientCenterY(value);
                }}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Size:</span>
              <NumberDragBox
                bind:value={settings.gradient_size}
                min={0.1}
                max={2}
                step={0.01}
                precision={2}
                on:change={async (e) => {
                  try {
                    await invoke('update_simulation_setting', {
                      settingName: 'gradient_size',
                      value: e.detail,
                    });
                  } catch (err) {
                    console.error('Failed to update gradient size:', err);
                  }
                }}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Angle:</span>
              <NumberDragBox
                bind:value={settings.gradient_angle}
                min={0}
                max={360}
                step={1}
                precision={0}
                unit="°"
                on:change={async (e) => {
                  try {
                    await invoke('update_simulation_setting', {
                      settingName: 'gradient_angle',
                      value: e.detail,
                    });
                  } catch (err) {
                    console.error('Failed to update gradient angle:', err);
                  }
                }}
              />
            </div>
          {/if}
        </div>
      </div>
    </fieldset>
  </form>
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleBackendGui} />

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './components/inputs/NumberDragBox.svelte';
  import AgentCountInput from './components/slime-mold/AgentCountInput.svelte';
  import LutSelector from './components/shared/LutSelector.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import ButtonSelect from './components/inputs/ButtonSelect.svelte';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
  import PresetFieldset from './components/shared/PresetFieldset.svelte';
  import './shared-theme.css';

  const dispatch = createEventDispatcher();

  export let menuPosition: string = 'middle';

  // Simulation state
  let settings = {
    // Pheromone Settings
    pheromone_decay_rate: 1.0,
    pheromone_deposition_rate: 0.5,
    pheromone_diffusion_rate: 0.5,
    decay_frequency: 1,
    diffusion_frequency: 1,

    // Agent Settings
    agent_speed_min: 100,
    agent_speed_max: 200,
    agent_turn_rate: 180, // degrees
    agent_jitter: 0.1,
    agent_sensor_angle: 45, // degrees
    agent_sensor_distance: 50,

    // Gradient Settings
    gradient_type: 'disabled',
    gradient_strength: 50,
    gradient_center_x: 0.5,
    gradient_center_y: 0.5,
    gradient_size: 1.0,
    gradient_angle: 0,
  };

  // State (not saved in presets)
  let position_generator = 'Random';

  // LUT state (runtime, not saved in presets)
  let lut_name = 'MATPLOTLIB_bone';
  let lut_reversed = true;

  // Agent count tracked separately (not part of preset settings)
  let currentAgentCount = 1_000_000;

  // Cursor interaction state (runtime, not saved in presets)
  let cursorSize = 300.0;
  let cursorStrength = 5.0;

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // UI state
  let show_about_section = false;

  // Simulation control state
  let running = false;
  let loading = false;
  let showUI = true;
  let currentFps = 0;
  let controlsVisible = true;

  // Auto-hide functionality for controls when UI is hidden
  let hideTimeout: number | null = null;

  // Cursor hiding functionality
  let cursorHidden = false;
  let cursorHideTimeout: number | null = null;

  // Event listeners
  let unlistenFps: (() => void) | null = null;

  // Camera controls

  let isMousePressed = false;
  let currentMouseButton = 0;

  async function returnToMenu() {
    try {
      await invoke('destroy_simulation');
      dispatch('back');
    } catch (error) {
      console.error('Failed to return to menu:', error);
    }
  }

  async function toggleBackendGui() {
    try {
      await invoke('toggle_gui');

      // Get the current GUI state
      const visible = (await invoke('get_gui_state')) as boolean;
      showUI = visible;

      // Handle auto-hide when UI is hidden
      if (!showUI) {
        showControls();
        showCursor();
        startAutoHideTimer();
        startCursorHideTimer();
      } else {
        stopAutoHideTimer();
        stopCursorHideTimer();
        showCursor();
        controlsVisible = true;
      }
    } catch (error) {
      console.error('Failed to toggle GUI:', error);
    }
  }

  async function stopSimulation() {
    try {
      await invoke('pause_simulation');
      running = false;
    } catch (error) {
      console.error('Failed to stop simulation:', error);
    }
  }

  async function resumeSimulation() {
    try {
      await invoke('resume_simulation');
      running = true;
    } catch (error) {
      console.error('Failed to resume simulation:', error);
    }
  }

  // Auto-hide functionality
  function startAutoHideTimer() {
    stopAutoHideTimer();
    hideTimeout = window.setTimeout(() => {
      controlsVisible = false;
      // Also hide cursor when controls are hidden
      if (!showUI) {
        hideCursor();
      }
    }, 3000);
  }

  function stopAutoHideTimer() {
    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  }

  function showControls() {
    controlsVisible = true;
  }

  // Cursor hiding functionality
  function hideCursor() {
    if (!cursorHidden) {
      document.body.style.cursor = 'none';
      cursorHidden = true;
    }
  }

  function showCursor() {
    if (cursorHidden) {
      document.body.style.cursor = '';
      cursorHidden = false;
    }
  }

  function startCursorHideTimer() {
    stopCursorHideTimer();
    cursorHideTimeout = window.setTimeout(() => {
      if (!showUI && !controlsVisible) {
        hideCursor();
      }
    }, 2000); // Hide cursor 2 seconds after last interaction
  }

  function stopCursorHideTimer() {
    if (cursorHideTimeout) {
      clearTimeout(cursorHideTimeout);
      cursorHideTimeout = null;
    }
  }

  function handleUserInteraction() {
    if (!showUI && !controlsVisible) {
      showControls();
      showCursor();
      startAutoHideTimer();
    } else if (!showUI && controlsVisible) {
      showCursor();
      startAutoHideTimer();
      startCursorHideTimer();
    }
  }

  // Helper function to convert agent count to millions
  const toMillions = (count: number) => count / 1_000_000;
  const fromMillions = (millions: number) => millions * 1_000_000;

  // Computed values
  $: agent_count_millions = toMillions(currentAgentCount);
  $: gradient_center_x_percent = settings.gradient_center_x * 100;
  $: gradient_center_y_percent = settings.gradient_center_y * 100;

  // Two-way binding handlers
  async function updateAgentCount(value: number) {
    const newCount = fromMillions(value);
    console.log('Updating agent count: input =', value, 'millions, actual count =', newCount);
    try {
      await invoke('update_agent_count', { count: newCount });
      console.log('Backend update completed, syncing from backend...');
      // Sync the actual agent count from backend
      await syncAgentCountFromBackend();
      console.log('Sync completed, currentAgentCount is now:', currentAgentCount);
    } catch (e) {
      console.error('Failed to update agent count:', e);
    }
  }

  async function updateGradientCenterX(value: number) {
    settings.gradient_center_x = value / 100;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'gradient_center_x',
        value: settings.gradient_center_x,
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
        value: settings.gradient_center_y,
      });
    } catch (e) {
      console.error('Failed to update gradient center Y:', e);
    }
  }

  async function updateLutReversed() {
    try {
      await invoke('toggle_lut_reversed');
      await syncSettingsFromBackend(); // Sync UI with backend
    } catch (e) {
      console.error('Failed to toggle LUT reversed:', e);
    }
  }

  // Cursor configuration handlers
  async function updateCursorSize(size: number) {
    cursorSize = size;
    try {
      await invoke('update_simulation_setting', { settingName: 'cursor_size', value: size });
    } catch (e) {
      console.error('Failed to update cursor size:', e);
    }
  }

  async function updateCursorStrength(strength: number) {
    cursorStrength = strength;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'cursor_strength',
        value: strength,
      });
    } catch (e) {
      console.error('Failed to update cursor strength:', e);
    }
  }

  async function handleGradientType(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    settings.gradient_type = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'gradient_type',
        value: settings.gradient_type,
      });
    } catch (err) {
      console.error('Failed to update gradient type:', err);
    }
  }

  async function updatePreset(value: string) {
    current_preset = value;
    try {
      await invoke('apply_preset', { presetName: value });
      await invoke('reset_trails'); // Clear all existing trails
      await syncSettingsFromBackend(); // Sync UI with new settings
      // Reset agents asynchronously to avoid blocking the UI
      invoke('reset_agents').catch((e) => console.error('Failed to reset agents:', e));
      console.log(`Applied preset: ${value}`);
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
  }

  async function savePreset(presetName: string) {
    try {
      await invoke('save_preset', { presetName: presetName.trim() });
      // Refresh the available presets list
      await loadAvailablePresets();
      // Set the current preset to the newly saved one
      current_preset = presetName.trim();
      console.log(`Saved preset: ${presetName}`);
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  // Load available presets from backend
  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_presets_for_simulation_type', {
        simulationType: 'slime_mold',
      });
      console.log('Available presets loaded:', available_presets);
      if (available_presets.length > 0 && !current_preset) {
        current_preset = available_presets[0];
        console.log('Set initial preset to:', current_preset);
      }
    } catch (e) {
      console.error('Failed to load available presets:', e);
    }
  }

  // Load available LUTs from backend
  async function loadAvailableLuts() {
    try {
      available_luts = await invoke('get_available_luts');
      console.log('Available LUTs loaded:', available_luts.length);
    } catch (e) {
      console.error('Failed to load available LUTs:', e);
    }
  }

  // Sync settings from backend to frontend
  async function syncSettingsFromBackend() {
    try {
      const currentSettings = (await invoke('get_current_settings')) as any;
      const currentState = (await invoke('get_current_state')) as {
        current_lut_name: string;
        lut_reversed: boolean;
      } | null;

      console.log('Syncing settings from backend:', { currentSettings, currentState });

      if (currentSettings) {
        // Handle gradient type conversion from enum to lowercase string
        if (currentSettings.gradient_type) {
          currentSettings.gradient_type = currentSettings.gradient_type.toLowerCase();
        }

        // Convert radians to degrees for frontend display
        if (currentSettings.agent_turn_rate !== undefined) {
          currentSettings.agent_turn_rate = (currentSettings.agent_turn_rate * 180) / Math.PI;
        }
        if (currentSettings.agent_sensor_angle !== undefined) {
          currentSettings.agent_sensor_angle = (currentSettings.agent_sensor_angle * 180) / Math.PI;
        }

        // Update the settings object with current backend values
        settings = {
          ...settings,
          ...currentSettings,
        };

        // Update computed values
        gradient_center_x_percent = settings.gradient_center_x * 100;
        gradient_center_y_percent = settings.gradient_center_y * 100;

        console.log('Settings synced from backend:', settings);
      }

      if (currentState) {
        // Update LUT-related settings from state
        lut_name = currentState.current_lut_name;
        lut_reversed = currentState.lut_reversed;

        // Update cursor configuration from state
        if ((currentState as any).cursor_size !== undefined) {
          cursorSize = (currentState as any).cursor_size;
        }
        if ((currentState as any).cursor_strength !== undefined) {
          cursorStrength = (currentState as any).cursor_strength;
        }

        // Update position generator from state
        if ((currentState as any).position_generator !== undefined) {
          position_generator = (currentState as any).position_generator;
        }

        console.log('State synced from backend:', {
          lut_name: lut_name,
          lut_reversed: lut_reversed,
          cursor_size: cursorSize,
          cursor_strength: cursorStrength,
          position_generator: position_generator,
        });
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  // Sync agent count separately from settings
  async function syncAgentCountFromBackend() {
    try {
      const agentCount = await invoke('get_current_agent_count');
      console.log('Backend returned agent count:', agentCount);
      if (agentCount !== null && agentCount !== undefined) {
        console.log('Updating currentAgentCount from', currentAgentCount, 'to', agentCount);
        currentAgentCount = agentCount as number;
      }
    } catch (e) {
      console.error('Failed to sync agent count from backend:', e);
    }
  }

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

  onMount(async () => {
    // Set up event listeners BEFORE starting simulation to avoid race conditions

    // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts with CameraControls)
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
    });

    // Start the simulation first
    await startSimulation();

    // Load available presets and LUTs
    await loadAvailablePresets();
    await loadAvailableLuts();

    // Sync settings from backend
    await syncSettingsFromBackend();
    await syncAgentCountFromBackend();

    // Listen for FPS updates
    unlistenFps = await listen('fps-update', (event: any) => {
      currentFps = event.payload;
    });
  });

  onDestroy(async () => {
    // Clean up the simulation
    try {
      await invoke('destroy_simulation');
    } catch (error) {
      console.error('Failed to destroy simulation on component destroy:', error);
    }

    if (unlistenFps) {
      unlistenFps();
    }

    // Remove auto-hide event listeners
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.removeEventListener(event, handleUserInteraction);
    });

    // Stop auto-hide timer
    stopAutoHideTimer();

    // Stop cursor hide timer and restore cursor
    stopCursorHideTimer();
    showCursor();
  });

  async function updateLutName(value: string) {
    try {
      await invoke('apply_lut_by_name', { lutName: value });
      await syncSettingsFromBackend(); // Sync UI with backend state
    } catch (e) {
      console.error('Failed to update LUT name:', e);
    }
  }

  async function handleMouseEvent(e: CustomEvent) {
    const event = e.detail as MouseEvent | WheelEvent;
    if (event.type === 'wheel') {
      const wheelEvent = event as WheelEvent;
      wheelEvent.preventDefault();

      const zoomDelta = -wheelEvent.deltaY * 0.001;

      // Convert screen coordinates to physical coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = wheelEvent.clientX * devicePixelRatio;
      const physicalCursorY = wheelEvent.clientY * devicePixelRatio;

      try {
        await invoke('zoom_camera_to_cursor', {
          delta: zoomDelta,
          cursorX: physicalCursorX,
          cursorY: physicalCursorY,
        });
      } catch (e) {
        console.error('Failed to zoom camera to cursor:', e);
      }
    } else if (event.type === 'mousedown') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();

      isMousePressed = true;
      currentMouseButton = mouseEvent.button;

      // Convert screen coordinates to world coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      console.log(
        `Mouse interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), raw: (${mouseEvent.clientX}, ${mouseEvent.clientY})`
      );

      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          mouseButton: mouseEvent.button,
        });
      } catch (e) {
        console.error('Failed to handle mouse interaction:', e);
      }
    } else if (event.type === 'mousemove') {
      if (isMousePressed) {
        const mouseEvent = event as MouseEvent;
        mouseEvent.preventDefault();

        // Convert screen coordinates to world coordinates
        const devicePixelRatio = window.devicePixelRatio || 1;
        const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
        const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

        // Use the same button state as when mouse was first pressed
        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            mouseButton: currentMouseButton,
          });
        } catch (e) {
          console.error('Failed to handle mouse interaction:', e);
        }
      }
    } else if (event.type === 'mouseup') {
      isMousePressed = false;

      // Stop cursor interaction when mouse is released
      try {
        await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
      } catch (e) {
        console.error('Failed to stop mouse interaction:', e);
      }
    } else if (event.type === 'contextmenu') {
      // Handle context menu as right-click for simulation interaction
      const mouseEvent = event as MouseEvent;

      // Convert screen coordinates to world coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      console.log(
        `Slime Mold context menu interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: 2`
      );

      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          mouseButton: 2, // Right mouse button
        });
      } catch (e) {
        console.error('Failed to handle Slime Mold context menu interaction:', e);
      }
    }
  }
</script>

<style>
  /* SlimeMold specific styles */

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

  input[type='number'],
  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
  }

  .interaction-controls-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    align-items: start;
  }

  .interaction-help {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cursor-settings {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cursor-settings-header {
    font-size: 0.9rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    padding: 0.25rem 0;
  }

  /* Mobile responsive design */
  @media (max-width: 768px) {
    .interaction-controls-grid {
      grid-template-columns: 1fr;
      gap: 0.75rem;
    }

    .interaction-help {
      gap: 0.4rem;
    }

    .cursor-settings {
      gap: 0.4rem;
    }

    .cursor-settings-header {
      font-size: 0.85rem;
    }
  }

  /* Key/Value pair settings layout */
  .settings-grid {
    display: grid;
    grid-template-columns: 200px 120px;
    gap: 0.25rem 0.5rem;
    justify-content: center;
  }

  .setting-item {
    display: contents;
  }

  .setting-label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
    padding: 0.5rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .setting-item:last-child .setting-label {
    border-bottom: none;
  }

  .setting-item select,
  .setting-item input[type='number'] {
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.9);
    padding: 0.25rem 0.5rem;
    font-family: inherit;
    font-size: 0.875rem;
    width: 100%;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    border: 0;
    padding: 0;
    white-space: nowrap;
    clip-path: inset(100%);
    clip: rect(0 0 0 0);
  }

  /* Settings section styling */
  .settings-section {
    margin-bottom: 1.5rem;
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .section-header {
    font-size: 1rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.9);
    margin: 0 0 0.75rem 0;
    padding: 0.25rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
  }
</style>
