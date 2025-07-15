<SimulationLayout
  simulationName="Ecosystem"
  running={!isPaused}
  {showUI}
  currentFps={fps}
  {controlsVisible}
  {menuPosition}
  on:back={navigateBack}
  on:toggleUI={toggleGui}
  on:pause={pauseSimulation}
  on:resume={resumeSimulation}
  on:reset={resetSimulation}
  on:randomize={randomizeSettings}
  on:userInteraction={handleUserInteraction}
  on:mouseEvent={handleMouseEvent}
>
  <form on:submit|preventDefault>
    <!-- About this simulation -->
    <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
      <p>
        The Ecosystem simulation models a complex biological environment where multiple species
        interact in dynamic relationships. Agents move through the world, consuming resources,
        reproducing, and competing for survival in a realistic ecosystem.
      </p>
      <p>
        Each species has unique characteristics like size, speed, energy requirements, and
        reproductive strategies. The simulation includes food sources, predator-prey relationships,
        and environmental factors that affect population dynamics.
      </p>
      <p>
        Watch as populations rise and fall, species adapt to environmental pressures, and complex
        ecological patterns emerge from simple behavioral rules. The ecosystem maintains balance
        through natural selection and resource competition.
      </p>
    </CollapsibleFieldset>

    <!-- Ecosystem Status -->
    <fieldset>
      <legend>Ecosystem Status</legend>
      <div class="control-group">
        <div class="stat-grid">
          <div class="stat-item">
            <span class="stat-label">Agent Count:</span>
            <span class="stat-value">{agentCount}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">Species:</span>
            <span class="stat-value">{speciesCount}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">Alive Agents:</span>
            <span class="stat-value">{aliveAgents}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">Total Energy:</span>
            <span class="stat-value">{totalEnergy.toFixed(1)}</span>
          </div>
        </div>
      </div>
    </fieldset>

    <!-- Legend -->
    <fieldset>
      <legend>Species Legend</legend>
      <div class="control-group">
        <EcosystemLegend syncTrigger={visibilitySyncTrigger} />
      </div>
    </fieldset>

    <!-- Settings -->
    <fieldset>
      <legend>Settings</legend>
      <div class="control-group">
        <EcosystemSettings settings={currentSettings} on:settingChange={handleSettingChange} />
      </div>
    </fieldset>

    <!-- Actions -->
    <fieldset>
      <legend>Actions</legend>
      <div class="control-group">
        <div class="action-buttons">
          <button type="button" on:click={resetSimulation}> 🔄 Reset Simulation </button>
          <button type="button" on:click={randomizeSettings}> 🎲 Randomize Settings </button>
        </div>
      </div>
    </fieldset>

    <!-- Controls -->
    <fieldset>
      <legend>Controls</legend>
      <div class="interaction-controls-grid">
        <div class="interaction-help">
          <div class="control-group">
            <span>🖱️ Mouse interaction available</span>
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
          <CursorConfig />
        </div>
      </div>
    </fieldset>
  </form>
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleGui} on:togglePause={togglePause} />

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import EcosystemSettings from './components/ecosystem/EcosystemSettings.svelte';
  import EcosystemLegend from './components/ecosystem/EcosystemLegend.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';

  const dispatch = createEventDispatcher();

  export let menuPosition: string = 'middle';

  interface EcosystemSettings {
    agent_count?: number;
    species_count?: number;
    [key: string]: unknown;
  }

  interface EcosystemState {
    agent_count?: number;
    species_count?: number;
    total_energy?: number;
    alive_agents?: number;
    [key: string]: unknown;
  }

  let currentSettings: EcosystemSettings = {};
  // Remove unused currentState variable
  let isPaused = false;
  let isGuiVisible = true;
  let showUI = true;
  let fps = 0;

  // Ecosystem-specific state
  let agentCount = 1000;
  let speciesCount = 3;
  let totalEnergy = 0;
  let aliveAgents = 0;

  // Auto-hide functionality for controls when UI is hidden
  let controlsVisible = true;
  let hideTimeout: number | null = null;

  // Cursor hiding functionality
  let cursorHidden = false;
  let cursorHideTimeout: number | null = null;

  // Event listeners
  let unlistenFps: (() => void) | null = null;

  // Visibility sync trigger
  let visibilitySyncTrigger = 0;

  // Toggle for expandable about section
  let show_about_section = false;

  async function navigateBack() {
    await destroySimulation();

    // Reset UI state
    isPaused = false;
    isGuiVisible = true;
    showUI = true;
    controlsVisible = true;
    cursorHidden = false;

    // Stop any active timers
    stopAutoHideTimer();
    stopCursorHideTimer();
    showCursor();

    dispatch('back');
  }

  async function destroySimulation() {
    try {
      // Stop the ecosystem simulation and clean up
      await invoke('destroy_simulation');
      isPaused = true; // Mark as stopped

      // Reset FPS
      fps = 0;

      // Render a frame to show the main menu background
      await invoke('render_frame');
    } catch (error) {
      console.error('Failed to destroy ecosystem simulation:', error);
    }
  }

  async function startSimulation() {
    try {
      await invoke('start_ecosystem_simulation');
      await loadSettings();
      await loadState();
    } catch (error) {
      console.error('Failed to start ecosystem simulation:', error);
    }
  }

  async function loadSettings() {
    try {
      const settings = (await invoke('get_current_settings')) as EcosystemSettings;
      if (settings) {
        currentSettings = settings;
        agentCount = currentSettings.agent_count || 1000;
        speciesCount = currentSettings.species_count || 3;
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
  }

  async function loadState() {
    try {
      const state = (await invoke('get_current_state')) as EcosystemState;
      if (state) {
        agentCount = state.agent_count || agentCount;
        speciesCount = state.species_count || speciesCount;
        totalEnergy = state.total_energy || 0;
        aliveAgents = state.alive_agents || 0;
      }
    } catch (error) {
      console.error('Failed to load state:', error);
    }
  }

  async function handleSettingChange(event: CustomEvent) {
    const { setting, value } = event.detail;

    try {
      await invoke('update_simulation_setting', { settingName: setting, value });
      await loadSettings();
    } catch (error) {
      console.error(`Failed to update setting ${setting}:`, error);
    }
  }

  async function pauseSimulation() {
    try {
      await invoke('pause_simulation');
      isPaused = true;
    } catch (error) {
      console.error('Failed to pause simulation:', error);
    }
  }

  async function resumeSimulation() {
    try {
      await invoke('resume_simulation');
      isPaused = false;
    } catch (error) {
      console.error('Failed to resume simulation:', error);
    }
  }

  async function togglePause() {
    if (isPaused) {
      await resumeSimulation();
    } else {
      await pauseSimulation();
    }
  }

  async function resetSimulation() {
    try {
      await invoke('reset_simulation');
      await loadSettings();
      await loadState();
      visibilitySyncTrigger++; // Trigger visibility sync
    } catch (error) {
      console.error('Failed to reset simulation:', error);
    }
  }

  async function randomizeSettings() {
    try {
      await invoke('randomize_settings');
      await loadSettings();
      await loadState();
    } catch (error) {
      console.error('Failed to randomize settings:', error);
    }
  }

  async function toggleGui() {
    try {
      await invoke('toggle_gui');

      // Get the current GUI state
      const visible = (await invoke('get_gui_state')) as boolean;
      isGuiVisible = visible;
      showUI = visible;

      // Handle auto-hide when UI is hidden
      if (!isGuiVisible) {
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

  // Auto-hide functionality
  function startAutoHideTimer() {
    stopAutoHideTimer();
    hideTimeout = window.setTimeout(() => {
      controlsVisible = false;
      // Also hide cursor when controls are hidden
      if (!isGuiVisible) {
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
      if (!isGuiVisible && !controlsVisible) {
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
    if (!isGuiVisible && !controlsVisible) {
      showControls();
      showCursor();
      startAutoHideTimer();
    } else if (!isGuiVisible && controlsVisible) {
      showCursor();
      startAutoHideTimer();
      startCursorHideTimer();
    }
  }

  // Mouse event handling for camera controls
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
    }
  }

  onMount(async () => {
    // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts with CameraControls)
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
    });

    // Start the simulation first
    await startSimulation();

    // Listen for FPS updates
    unlistenFps = await listen('fps-update', (event: { payload: number }) => {
      fps = event.payload;
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
</script>

<style>
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

  .control-group:last-child {
    margin-bottom: 0;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .stat-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0;
  }

  .stat-label {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.9rem;
  }

  .stat-value {
    color: rgba(255, 255, 255, 0.9);
    font-weight: 500;
    font-size: 0.9rem;
  }

  .action-buttons {
    display: flex;
    gap: 1rem;
    justify-content: center;
  }

  /* Interaction controls styling */
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
    font-weight: bold;
    color: rgba(255, 255, 255, 0.9);
  }

  /* Mobile responsive design */
  @media (max-width: 768px) {
    .action-buttons {
      flex-direction: column;
      gap: 0.75rem;
    }

    .stat-grid {
      grid-template-columns: 1fr;
      gap: 0.4rem;
    }

    .interaction-controls-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
