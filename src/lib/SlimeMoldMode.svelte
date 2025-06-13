<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './NumberDragBox.svelte';
  import AgentCountInput from './AgentCountInput.svelte';

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

    // Display Settings
    fps_limit: 60,
    fps_limit_enabled: false,
    lut_index: 0,
    lut_reversed: false
  };

  // Agent count tracked separately (not part of preset settings)
  let currentAgentCount = 1_000_000;

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
  
  // Helper function to format numbers with commas
  const formatNumber = (num: number) => num.toLocaleString();

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
    // Store as degrees in frontend, convert to radians for backend
    settings.agent_turn_rate = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_turn_rate', 
        value: (value * Math.PI) / 180 // Convert degrees to radians
      });
    } catch (e) {
      console.error('Failed to update turn rate:', e);
    }
  }

  async function updateSensorAngle(value: number) {
    // Store as degrees in frontend, convert to radians for backend
    settings.agent_sensor_angle = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'agent_sensor_angle', 
        value: (value * Math.PI) / 180 // Convert degrees to radians
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
      await syncSettingsFromBackend(); // Sync UI with new settings
      // Reset agents asynchronously to avoid blocking the UI
      invoke('reset_agents').catch(e => console.error('Failed to reset agents:', e));
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

  // Gradient editor state
  let gradientStops = [
    { position: 0.0, color: '#0000ff' },
    { position: 1.0, color: '#ffff00' }
  ];
  let selectedStopIndex = 0;
  let isDragging = false;
  let dragStartX = 0;
  let dragStopIndex = -1;

  async function showGradientEditor() {
    // Store original state for cancel functionality
    originalGradientStops = JSON.parse(JSON.stringify(gradientStops));
    
    // TODO: Initialize with current LUT if needed
    // For now, start with default black to white gradient
    
    show_gradient_editor = true;
    
    // Immediately apply the default gradient to show the editable gradient in the simulation
    updateLivePreview();
  }

  function handleStopMouseDown(event: MouseEvent, index: number) {
    isDragging = true;
    dragStopIndex = index;
    selectedStopIndex = index;
    dragStartX = event.clientX;
    
    // Prevent text selection while dragging
    event.preventDefault();
    
    // Add global event listeners
    document.addEventListener('mousemove', handleStopMouseMove);
    document.addEventListener('mouseup', handleStopMouseUp);
  }

  function handleStopMouseMove(event: MouseEvent) {
    if (!isDragging || dragStopIndex === -1) return;
    
    // Find the gradient container to calculate relative position
    const container = document.querySelector('.gradient-stops-container');
    if (!container) return;
    
    const rect = container.getBoundingClientRect();
    const newPosition = (event.clientX - rect.left) / rect.width;
    const clampedPosition = Math.max(0, Math.min(1, newPosition));
    
    // Update the position of the dragged stop
    const stop = gradientStops[dragStopIndex];
    stop.position = clampedPosition;
    
    // Re-sort and find the new index of the moved stop
    gradientStops = gradientStops.sort((a, b) => a.position - b.position);
    dragStopIndex = gradientStops.findIndex(s => s === stop);
    selectedStopIndex = dragStopIndex;
    
    updateLivePreview();
  }

  function handleStopMouseUp() {
    isDragging = false;
    dragStopIndex = -1;
    
    // Remove global event listeners
    document.removeEventListener('mousemove', handleStopMouseMove);
    document.removeEventListener('mouseup', handleStopMouseUp);
  }

  function addGradientStop(position: number) {
    // Find the two stops this position falls between to interpolate color
    let leftStop = gradientStops[0];
    let rightStop = gradientStops[gradientStops.length - 1];
    
    for (let i = 0; i < gradientStops.length - 1; i++) {
      if (gradientStops[i].position <= position && gradientStops[i + 1].position >= position) {
        leftStop = gradientStops[i];
        rightStop = gradientStops[i + 1];
        break;
      }
    }
    
    // Interpolate color between left and right stops
    let ratio = 0.5; // Default to middle if positions are the same
    if (rightStop.position !== leftStop.position) {
      ratio = (position - leftStop.position) / (rightStop.position - leftStop.position);
    }
    
    const leftRgb = hexToRgb(leftStop.color);
    const rightRgb = hexToRgb(rightStop.color);
    const interpolatedRgb = {
      r: Math.round(leftRgb.r + (rightRgb.r - leftRgb.r) * ratio),
      g: Math.round(leftRgb.g + (rightRgb.g - leftRgb.g) * ratio),
      b: Math.round(leftRgb.b + (rightRgb.b - leftRgb.b) * ratio)
    };
    
    const newStop = {
      position: Math.max(0, Math.min(1, position)),
      color: rgbToHex(interpolatedRgb.r, interpolatedRgb.g, interpolatedRgb.b)
    };
    
    gradientStops = [...gradientStops, newStop].sort((a, b) => a.position - b.position);
    selectedStopIndex = gradientStops.findIndex(stop => stop === newStop);
    updateLivePreview();
  }

  function removeGradientStop(index: number) {
    if (gradientStops.length <= 2) return; // Keep at least 2 stops
    gradientStops = gradientStops.filter((_, i) => i !== index);
    selectedStopIndex = Math.min(selectedStopIndex, gradientStops.length - 1);
    updateLivePreview();
  }

  function updateStopPosition(index: number, position: number) {
    const clampedPosition = Math.max(0, Math.min(1, position));
    const stop = gradientStops[index];
    stop.position = clampedPosition;
    
    // Re-sort and find the new index of the moved stop
    gradientStops = gradientStops.sort((a, b) => a.position - b.position);
    selectedStopIndex = gradientStops.findIndex(s => s === stop);
    updateLivePreview();
  }

  function updateStopColor(index: number, color: string) {
    gradientStops[index].color = color;
    updateLivePreview();
  }

  let previewTimeout: NodeJS.Timeout | null = null;
  
  async function updateLivePreview() {
    // Throttle preview updates to avoid too many backend calls
    if (previewTimeout) {
      clearTimeout(previewTimeout);
    }
    
    previewTimeout = setTimeout(async () => {
      const lutData = generateLutFromGradient(gradientStops);
      try {
        await invoke('apply_custom_lut', { lutData });
      } catch (e) {
        console.error('Failed to apply custom LUT preview:', e);
      }
    }, 100); // 100ms delay
  }

  function generateLutFromGradient(stops: Array<{position: number, color: string}>): number[] {
    const lutSize = 256;
    const redChannel = [];
    const greenChannel = [];
    const blueChannel = [];
    
    for (let i = 0; i < lutSize; i++) {
      const position = i / (lutSize - 1);
      
      // Find the two stops this position falls between
      let leftStop = stops[0];
      let rightStop = stops[stops.length - 1];
      
      for (let j = 0; j < stops.length - 1; j++) {
        if (stops[j].position <= position && stops[j + 1].position >= position) {
          leftStop = stops[j];
          rightStop = stops[j + 1];
          break;
        }
      }
      
      // Interpolate between the two colors
      const ratio = (position - leftStop.position) / (rightStop.position - leftStop.position);
      const leftRgb = hexToRgb(leftStop.color);
      const rightRgb = hexToRgb(rightStop.color);
      
      const r = Math.round(leftRgb.r + (rightRgb.r - leftRgb.r) * ratio);
      const g = Math.round(leftRgb.g + (rightRgb.g - leftRgb.g) * ratio);
      const b = Math.round(leftRgb.b + (rightRgb.b - leftRgb.b) * ratio);
      
      // Store in separate channels as expected by backend
      redChannel.push(r);
      greenChannel.push(g);
      blueChannel.push(b);
    }
    
    // Combine channels: [R0..R255, G0..G255, B0..B255]
    return [...redChannel, ...greenChannel, ...blueChannel];
  }

  function hexToRgb(hex: string) {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16)
    } : { r: 0, g: 0, b: 0 };
  }

  function rgbToHex(r: number, g: number, b: number): string {
    return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
  }

  async function saveCustomLut() {
    if (!custom_lut_name.trim()) {
      alert('Please enter a name for the custom LUT');
      return;
    }
    
    const lutData = generateLutFromGradient(gradientStops);
    try {
      await invoke('save_custom_lut', { 
        name: custom_lut_name.trim(), 
        lutData 
      });
      show_gradient_editor = false;
      custom_lut_name = '';
      
      // Refresh the available LUTs list
      await loadAvailableLuts();
      console.log('Custom LUT saved successfully');
    } catch (e) {
      console.error('Failed to save custom LUT:', e);
      alert('Failed to save custom LUT. Please try again.');
    }
  }

  function setNewPresetName(value: string) {
    new_preset_name = value;
    dispatch('command', { type: 'SetNewPresetName', value: new_preset_name });
  }

  function setCustomLutName(value: string) {
    custom_lut_name = value;
    dispatch('command', { type: 'SetCustomLutName', value: custom_lut_name });
  }

  // Input values for gradient center
  let gradientCenterXInput = gradient_center_x_percent;
  let gradientCenterYInput = gradient_center_y_percent;

  // Update handlers for gradient center (agent count updates are handled explicitly by AgentCountInput)
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

  async function resumeSimulation() {
    if (running || loading) return;
    
    try {
      // Just restart the render loop without recreating the simulation
      await invoke('resume_simulation');
      running = true;
      currentFps = 0;
    } catch (e) {
      console.error('Failed to resume simulation:', e);
    }
  }

  async function stopSimulation() {
    running = false;
    
    try {
      // Just pause the render loop, don't destroy simulation
      await invoke('stop_simulation');
      
      // Reset FPS
      currentFps = 0;
      
      // Immediately render a frame to show the triangle instead of last simulation frame
      await invoke('render_frame');
    } catch (e) {
      console.error('Failed to stop simulation:', e);
    }
  }

  async function destroySimulation() {
    running = false;
    
    try {
      // Actually destroy the simulation completely
      await invoke('destroy_simulation');
      
      // Reset FPS
      currentFps = 0;
      
      // Render a frame to show the triangle
      await invoke('render_frame');
    } catch (e) {
      console.error('Failed to destroy simulation:', e);
    }
  }

  async function returnToMenu() {
    await destroySimulation();
    
    
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
          ...currentSettings
        };
        
        // Update computed values
        gradientCenterXInput = settings.gradient_center_x * 100;
        gradientCenterYInput = settings.gradient_center_y * 100;
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
        currentAgentCount = agentCount;
      }
    } catch (e) {
      console.error('Failed to sync agent count from backend:', e);
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
      console.log('Simulation initialized, syncing settings and agent count...');
      await syncSettingsFromBackend();
      await syncAgentCountFromBackend();
    });

    // Listen for FPS updates from backend
    fpsUpdateUnlisten = await listen('fps-update', (event) => {
      currentFps = event.payload as number;
      
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
    
    // Clean up drag event listeners
    document.removeEventListener('mousemove', handleStopMouseMove);
    document.removeEventListener('mouseup', handleStopMouseUp);
    
    // Clear any pending preview timeout
    if (previewTimeout) {
      clearTimeout(previewTimeout);
    }
    
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
          <span>{formatNumber(currentAgentCount)} agents at {currentFps} FPS</span>
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
            <NumberDragBox 
              bind:value={settings.fps_limit}
              min={1}
              max={1200}
              step={1}
              precision={0}
              on:change={async (e) => {
                try {
                  await invoke('set_fps_limit', { 
                    enabled: settings.fps_limit_enabled, 
                    limit: e.detail 
                  });
                  console.log(`FPS limit set to: ${e.detail}`);
                } catch (err) {
                  console.error('Failed to update FPS limit:', err);
                }
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
            <div class="dialog-content gradient-editor-content">
              <h3>Custom LUT Editor</h3>
              
              <!-- LUT Name Input -->
              <div class="control-group">
                <label for="customLutName">LUT Name</label>
                <input 
                  type="text" 
                  id="customLutName"
                  bind:value={custom_lut_name}
                  placeholder="Enter LUT name..."
                />
              </div>

              <!-- Gradient Preview -->
              <div class="gradient-preview-container">
                <div class="gradient-preview" 
                     style="background: linear-gradient(to right, {gradientStops.map(stop => `${stop.color} ${stop.position * 100}%`).join(', ')})">
                </div>
                <div class="gradient-stops-container"
                     on:click={(e) => {
                       const rect = e.currentTarget.getBoundingClientRect();
                       const position = (e.clientX - rect.left) / rect.width;
                       addGradientStop(position);
                     }}>
                  {#each gradientStops as stop, index}
                    <div class="gradient-stop" 
                         class:selected={index === selectedStopIndex}
                         class:dragging={isDragging && dragStopIndex === index}
                         style="left: {stop.position * 100}%; background-color: {stop.color}"
                         on:mousedown={(e) => handleStopMouseDown(e, index)}
                         on:click|stopPropagation={() => selectedStopIndex = index}>
                      {#if gradientStops.length > 2}
                        <button class="remove-stop" 
                                on:click|stopPropagation={() => removeGradientStop(index)}>×</button>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>

              <!-- Selected Stop Controls -->
              {#if selectedStopIndex >= 0 && selectedStopIndex < gradientStops.length}
                <div class="stop-controls">
                  <h4>Color Stop {selectedStopIndex + 1}</h4>
                  <div class="control-row">
                    <div class="control-group">
                      <label for="stopColor">Color</label>
                      <input 
                        type="color" 
                        id="stopColor"
                        value={gradientStops[selectedStopIndex].color}
                        on:input={(e) => {
                          const color = (e.target as HTMLInputElement).value;
                          updateStopColor(selectedStopIndex, color);
                        }}
                      />
                    </div>
                  </div>
                </div>
              {/if}

              <!-- Instructions -->
              <div class="gradient-instructions">
                <p><strong>Instructions:</strong></p>
                <ul>
                  <li>Click on the gradient to add new color stops</li>
                  <li>Click on a color stop to select it</li>
                  <li>Use the controls below to adjust position and color</li>
                  <li>Click × on a stop to remove it (minimum 2 stops required)</li>
                  <li>Changes apply to the simulation in real-time</li>
                </ul>
              </div>

              <!-- Dialog Actions -->
              <div class="dialog-actions">
                <button 
                  type="button"
                  class="primary-button"
                  on:click={saveCustomLut}
                  disabled={!custom_lut_name.trim()}
                >
                  💾 Save LUT
                </button>
                <button 
                  type="button"
                  on:click={async () => {
                    // Restore the currently active LUT from the dropdown
                    try {
                      await invoke('apply_lut_by_index', { lutIndex: settings.lut_index });
                    } catch (e) {
                      console.error('Failed to restore original LUT:', e);
                    }
                    
                    // Close the editor
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
          <button type="button" on:click={resumeSimulation} disabled={running}>▶ Resume</button>
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
              await invoke('reset_trails'); // Also reset trails to make agent redistribution visible
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update pheromone decay rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="depositionRate">Deposition Rate (%)</label>
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update pheromone deposition rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="diffusionRate">Diffusion Rate (%)</label>
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update pheromone diffusion rate:', err);
              }
            }}
          />
        </div>
      </fieldset>

      <!-- 6. Agent Settings -->
      <fieldset>
        <legend>Agent Settings</legend>
        <div class="control-group">
          <label for="agentCount">Agent Count (millions)</label>
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
        <div class="control-group">
          <label for="minSpeed">Min Speed</label>
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update min speed:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="maxSpeed">Max Speed</label>
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update max speed:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="turnRate">Turn Rate (degrees)</label>
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
                  value: (e.detail * Math.PI) / 180 // Convert degrees to radians
                });
              } catch (err) {
                console.error('Failed to update turn rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="jitter">Jitter</label>
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update agent jitter:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="sensorAngle">Sensor Angle (degrees)</label>
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
                  value: (e.detail * Math.PI) / 180 // Convert degrees to radians
                });
              } catch (err) {
                console.error('Failed to update sensor angle:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="sensorDistance">Sensor Distance</label>
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
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update sensor distance:', err);
              }
            }}
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
            <NumberDragBox 
              bind:value={settings.gradient_strength}
              min={0}
              max={100}
              step={1}
              precision={0}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'gradient_strength', 
                    value: e.detail 
                  });
                } catch (err) {
                  console.error('Failed to update gradient strength:', err);
                }
              }}
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
                    value: e.detail 
                  });
                } catch (err) {
                  console.error('Failed to update gradient size:', err);
                }
              }}
            />
          </div>
          <div class="control-group">
            <label for="gradientAngle">Angle (degrees)</label>
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
                    value: e.detail 
                  });
                } catch (err) {
                  console.error('Failed to update gradient angle:', err);
                }
              }}
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
    background: transparent;
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

  /* Gradient Editor Styles */
  .gradient-editor-content {
    min-width: 500px;
    max-width: 600px;
    color: black;
  }

  .gradient-preview-container {
    margin: 1rem 0;
    position: relative;
  }

  .gradient-preview {
    height: 40px;
    border: 2px solid #ccc;
    border-radius: 4px;
    margin-bottom: 10px;
  }

  .gradient-stops-container {
    position: relative;
    height: 30px;
    background: #f5f5f5;
    border: 1px solid #ddd;
    border-radius: 4px;
    cursor: pointer;
  }

  .gradient-stop {
    position: absolute;
    top: 50%;
    transform: translateX(-50%) translateY(-50%);
    width: 20px;
    height: 20px;
    border: 2px solid white;
    border-radius: 50%;
    cursor: grab;
    box-shadow: 0 2px 4px rgba(0,0,0,0.3);
    transition: all 0.2s ease;
    user-select: none;
  }

  .gradient-stop:hover {
    transform: translateX(-50%) translateY(-50%) scale(1.1);
  }

  .gradient-stop.selected {
    border-color: #646cff;
    border-width: 3px;
    box-shadow: 0 2px 8px rgba(100, 108, 255, 0.4);
  }

  .gradient-stop.dragging {
    cursor: grabbing;
    transform: translateX(-50%) translateY(-50%) scale(1.2);
    z-index: 10;
    transition: none;
  }

  .remove-stop {
    position: absolute;
    top: -8px;
    right: -8px;
    width: 16px;
    height: 16px;
    background: #ff4444;
    color: white;
    border: none;
    border-radius: 50%;
    font-size: 10px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .remove-stop:hover {
    background: #ff6666;
  }

  .stop-controls {
    background: #f8f9fa;
    padding: 1rem;
    border-radius: 4px;
    margin: 1rem 0;
  }

  .stop-controls h4 {
    margin: 0 0 0.5rem 0;
    color: #333;
  }

  .control-row {
    display: flex;
    gap: 1rem;
    align-items: end;
  }

  .control-row .control-group {
    flex: 1;
  }

  .gradient-instructions {
    background: #e3f2fd;
    padding: 1rem;
    border-radius: 4px;
    margin: 1rem 0;
    font-size: 0.9rem;
  }

  .gradient-instructions p {
    margin: 0 0 0.5rem 0;
    color: #1976d2;
  }

  .gradient-instructions ul {
    margin: 0;
    padding-left: 1.2rem;
  }

  .gradient-instructions li {
    margin: 0.2rem 0;
    color: #333;
  }

  .primary-button {
    background: #646cff;
    color: white;
    border: 1px solid #646cff;
  }

  .primary-button:hover:not(:disabled) {
    background: #535bf2;
    border-color: #535bf2;
  }

  .primary-button:disabled {
    background: #ccc;
    border-color: #ccc;
    cursor: not-allowed;
  }
</style>