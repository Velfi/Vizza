<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  import SimulationControlBar from './components/shared/SimulationControlBar.svelte';
  import SimulationMenuContainer from './components/shared/SimulationMenuContainer.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import EcosystemSettings from './components/ecosystem/EcosystemSettings.svelte';

  const dispatch = createEventDispatcher();

  let currentSettings: any = {};
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
    } catch (error) {
      console.error('Failed to start ecosystem simulation:', error);
    }
  }

  async function loadSettings() {
    try {
      const settings = await invoke('get_current_settings');
      if (settings) {
        currentSettings = settings;
        agentCount = currentSettings.agent_count || 1000;
        speciesCount = currentSettings.species_count || 3;
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
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

  async function resetSimulation() {
    try {
      await invoke('reset_simulation');
      await loadSettings();
    } catch (error) {
      console.error('Failed to reset simulation:', error);
    }
  }

  async function randomizeSettings() {
    try {
      await invoke('randomize_settings');
      await loadSettings();
    } catch (error) {
      console.error('Failed to randomize settings:', error);
    }
  }

  async function toggleGui() {
    try {
      await invoke('toggle_gui');
      
      // Get the current GUI state
      const visible = await invoke('get_gui_state') as boolean;
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

  onMount(async () => {
    // Start the simulation first
    await startSimulation();

    // Listen for FPS updates
    unlistenFps = await listen('fps-update', (event: any) => {
      fps = event.payload;
    });

    // Add event listeners for auto-hide functionality
    const events = ['mousedown', 'mousemove', 'keydown', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
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
    const events = ['mousedown', 'mousemove', 'keydown', 'wheel', 'touchstart'];
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

<div class="ecosystem-container">
  <SimulationControlBar
    simulationName="Ecosystem"
    running={!isPaused}
    showUI={showUI}
    currentFps={fps}
    controlsVisible={controlsVisible}
    on:back={navigateBack}
    on:toggleUI={toggleGui}
    on:pause={pauseSimulation}
    on:resume={resumeSimulation}
    on:reset={resetSimulation}
    on:randomize={randomizeSettings}
    on:userInteraction={handleUserInteraction}
  />

  <SimulationMenuContainer {showUI}>
      <div class="ecosystem-menu">
        <div class="ecosystem-stats">
          <h3>Ecosystem Status</h3>
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

        <EcosystemSettings
          settings={currentSettings}
          on:settingChange={handleSettingChange}
        />

        <div class="ecosystem-actions">
          <button class="action-button reset-button" on:click={resetSimulation}>
            🔄 Reset Simulation
          </button>
          <button class="action-button randomize-button" on:click={randomizeSettings}>
            🎲 Randomize Settings
          </button>
        </div>

        <CursorConfig />
      </div>
    </SimulationMenuContainer>
</div>

<style>
  .ecosystem-container {
    position: relative;
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .ecosystem-menu {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    max-height: 80vh;
    overflow-y: auto;
  }

  .ecosystem-stats {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .ecosystem-stats h3 {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1.1rem;
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

  .ecosystem-actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
    padding: 1rem 0;
  }

  .action-button {
    padding: 0.75rem 1.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.9rem;
    transition: all 0.3s ease;
    flex: 1;
    max-width: 150px;
  }

  .action-button:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.4);
    transform: translateY(-1px);
  }

  .reset-button:hover {
    background: rgba(255, 193, 7, 0.2);
    border-color: rgba(255, 193, 7, 0.4);
    color: #ffc107;
  }

  .randomize-button:hover {
    background: rgba(156, 39, 176, 0.2);
    border-color: rgba(156, 39, 176, 0.4);
    color: #9c27b0;
  }
</style> 