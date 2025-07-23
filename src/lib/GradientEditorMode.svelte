<SimulationLayout
  simulationName="Gradient Editor"
  {running}
  {loading}
  {showUI}
  {currentFps}
  {controlsVisible}
  {menuPosition}
  showCenterControls={false}
  showRightControls={false}
  on:back={goBack}
  on:toggleUI={toggleBackendGui}
  on:pause={stopSimulation}
  on:resume={resumeSimulation}
  on:userInteraction={handleUserInteraction}
  on:mouseEvent={handleMouseEvent}
>
  <!-- Custom GEM Header -->
  <div class="gem-header">
    <div class="header-left">
      <h2>Gradient Editor</h2>
    </div>
    <div class="header-right">
      <button class="save-button" on:click={saveLUT} disabled={!lutName.trim()}>
        💾 Save LUT
      </button>
    </div>
  </div>

  <!-- Success Message -->
  {#if saveSuccessMessage}
    <div class="success-message">
      {saveSuccessMessage}
    </div>
  {/if}

  <!-- Gradient Stops Overlay -->
  <div class="gradient-stops-overlay">
    <div class="stops-container">
      <div class="gradient-bar" 
           on:dblclick={addStopAtPosition}
           on:keydown={(e) => e.key === 'Enter' && addStopAtPosition(e)}>
        {#each gradientStops as stop, index}
          <div class="color-stop"
               class:selected={selectedStopIndex === index}
               style="left: {stop.position * 100}%; background-color: {stop.color}"
               on:click={() => selectStop(index)}
               on:mousedown={(e) => startDragging(e, index)}>
            <div class="stop-handle"></div>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- Control Panel -->
  <div class="control-panel">
    <!-- Header Section - Grid layout -->
    <div class="control-header">
      <div class="header-grid">
        <div class="name-section">
          <label>Name:</label>
          <input type="text" bind:value={lutName} placeholder="LUT name" class="lut-name-input" />
        </div>
        <div class="preset-section">
          <label>Preset:</label>
          <Selector
            options={['Custom', 'Rainbow', 'Heat', 'Cool', 'Viridis', 'Plasma', 'Inferno']}
            bind:value={selectedPreset}
            on:change={applyPreset}
          />
        </div>
        <div class="space-section">
          <label>Space:</label>
          <Selector
            options={['RGB', 'Lab', 'OkLab', 'Jzazbz', 'HSLuv']}
            bind:value={selectedColorSpace}
            on:change={handleColorSpaceChange}
          />
        </div>
        <div class="display-section">
          <label>Display:</label>
          <Selector
            options={['Smooth', 'Dithered']}
            bind:value={selectedDisplayMode}
            on:change={handleDisplayModeChange}
          />
        </div>
      </div>
    </div>

    <!-- Selected Stop Controls - Grid layout -->
    {#if selectedStopIndex >= 0 && selectedStopIndex < gradientStops.length}
      <div class="stop-controls">
        <div class="stop-header">
          <span class="stop-title">Stop {selectedStopIndex + 1} ({Math.round(gradientStops[selectedStopIndex].position * 100)}%)</span>
          <div class="stop-actions">
            <button type="button" on:click={duplicateStop} class="btn-compact">Copy</button>
            {#if gradientStops.length > 2}
              <button type="button" on:click={deleteStop} class="btn-compact btn-danger">Delete</button>
            {/if}
          </div>
        </div>
        <div class="stop-controls-grid">
          <div class="control-item">
            <label>Color</label>
            <input type="color" bind:value={gradientStops[selectedStopIndex].color} on:input={handleColorInput} class="color-picker" />
          </div>
          <div class="control-item">
            <label>Position</label>
            <input type="range" min="0" max="1" step="0.01" 
                   bind:value={gradientStops[selectedStopIndex].position} 
                   on:input={handlePositionInput} class="position-slider" />
          </div>
        </div>
      </div>
    {/if}

    <!-- Actions Section - Grid layout -->
    <div class="actions-section">
      <div class="actions-grid">
        <div class="action-group">
          <label>Actions</label>
          <div class="button-group">
            <button type="button" on:click={reverseGradient} class="btn-action">Reverse</button>
            <button type="button" on:click={exportLUT} class="btn-action">Export</button>
          </div>
        </div>
        <div class="random-group">
          <label>Random Generator</label>
          <div class="random-controls">
            <div class="random-row">
              <Selector
                options={['Basic', 'Warm', 'Cool', 'Pastel', 'Neon', 'Earth', 'Monochrome', 'Complementary', 'Truly Random']}
                bind:value={selectedRandomScheme}
              />
              <Selector
                options={['Random', 'Even']}
                bind:value={randomStopPlacement}
              />
            </div>
            <div class="random-row">
              <div class="stops-control">
                <span>Stops: {randomStopCount}</span>
                <input type="range" min="2" max="8" step="1" bind:value={randomStopCount} class="stops-range" />
              </div>
              <button type="button" on:click={triggerRandomization} class="btn-generate">Generate</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleBackendGui} on:togglePause={togglePause} />

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import { interpolate, formatHex, rgb } from 'culori';

  const dispatch = createEventDispatcher();

  // State variables
  let lutName = '';
  let selectedColorSpace: 'RGB' | 'Lab' | 'OkLab' | 'Jzazbz' | 'HSLuv' = 'OkLab';
  let selectedPreset = 'Custom';
  let selectedDisplayMode = 'Smooth';
  let selectedRandomScheme: string = 'Basic';
  let randomStopPlacement: 'Even' | 'Random' = 'Random';
  let randomStopCount: number = 3;
  let gradientStops = [
    { position: 0, color: '#0000ff' },
    { position: 1, color: '#ffff00' }
  ];
  let selectedStopIndex = 0;
  let isDragging = false;
  let dragStopIndex = -1;
  let updateTimeout: number | null = null;
  let unlistenSimulationInitialized: (() => void) | null = null;
  let saveSuccessMessage = '';
  let saveSuccessTimeout: number | null = null;

  // Simulation control state
  let running = false;
  let loading = false;
  let showUI = true;
  let currentFps = 0;
  let controlsVisible = true;
  let menuPosition = 'middle';

  // Auto-hide functionality for controls when UI is hidden
  let hideTimeout: number | null = null;

  // Simplified color interpolation functions using culori
  function interpolateColor(color1: string, color2: string, t: number): string {
    try {
      let colorSpace = 'rgb';
      
      switch (selectedColorSpace) {
        case 'RGB':
          colorSpace = 'rgb';
          break;
        case 'Lab':
          colorSpace = 'lab';
          break;
        case 'OkLab':
          colorSpace = 'oklab';
          break;
        case 'Jzazbz':
          // Fallback to lab if jzazbz isn't supported
          colorSpace = 'lab';
          break;
        case 'HSLuv':
          // Fallback to hsl if hsluv isn't supported
          colorSpace = 'hsl';
          break;
      }
      
      // Create interpolator with error handling
      const interpolator = interpolate([color1, color2], colorSpace);
      const result = interpolator(t);
      
      // Convert result to hex, with fallback
      if (result) {
        const hexResult = formatHex(result);
        if (hexResult) {
          return hexResult;
        }
      }
      
      // Fallback: simple RGB interpolation
      const c1 = rgb(color1);
      const c2 = rgb(color2);
      if (c1 && c2) {
        const rgbInterpolator = interpolate([c1, c2], 'rgb');
        const rgbResult = rgbInterpolator(t);
        const hexFallback = formatHex(rgbResult);
        if (hexFallback) {
          return hexFallback;
        }
      }
      
      // Final fallback
      return color1;
    } catch (error) {
      console.error('Color interpolation error:', error);
      return color1;
    }
  }

  // Event handlers
  function handleColorSpaceChange() {
    updateGradient();
  }

  async function handleDisplayModeChange() {
    const mode = selectedDisplayMode === 'Dithered' ? 1 : 0;
    try {
      await invoke('set_gradient_display_mode', { mode });
    } catch (error) {
      console.error('Failed to set display mode:', error);
    }
  }

  function selectStop(index: number) {
    selectedStopIndex = index;
  }

  function addStopAtPosition(event: MouseEvent | KeyboardEvent) {
    if (event instanceof MouseEvent) {
      const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
      const position = (event.clientX - rect.left) / rect.width;
      addStop(position);
    } else {
      addStop(0.5);
    }
  }

  function addStop(position: number) {
    const color = getColorAtPosition(position);
    gradientStops = [...gradientStops, { position, color }];
    gradientStops.sort((a, b) => a.position - b.position);
    selectedStopIndex = gradientStops.findIndex(stop => Math.abs(stop.position - position) < 0.001);
    updateGradient();
  }

  function deleteStop() {
    if (gradientStops.length <= 2) return;
    gradientStops = gradientStops.filter((_, i) => i !== selectedStopIndex);
    if (selectedStopIndex >= gradientStops.length) {
      selectedStopIndex = gradientStops.length - 1;
    }
    updateGradient();
  }

  function duplicateStop() {
    const stop = gradientStops[selectedStopIndex];
    const newStop = { ...stop, position: Math.min(1, stop.position + 0.05) };
    gradientStops = [...gradientStops, newStop];
    gradientStops.sort((a, b) => a.position - b.position);
    selectedStopIndex = gradientStops.findIndex(s => s === newStop);
    updateGradient();
  }

  function startDragging(event: MouseEvent, index: number) {
    event.preventDefault();
    isDragging = true;
    dragStopIndex = index;
    selectedStopIndex = index;

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging) return;
      const gradientBar = document.querySelector('.gradient-bar') as HTMLElement;
      if (!gradientBar) return;
      
      const rect = gradientBar.getBoundingClientRect();
      const position = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      
      // Update position without sorting during drag for better performance
      gradientStops[dragStopIndex].position = position;
      gradientStops = [...gradientStops]; // Trigger reactivity
      
      // Use immediate visual update during drag
      updateGradientImmediate();
    };

    const handleMouseUp = () => {
      isDragging = false;
      // Sort only when drag ends
      gradientStops = [...gradientStops].sort((a, b) => a.position - b.position);
      dragStopIndex = gradientStops.findIndex(stop => Math.abs(stop.position - gradientStops[dragStopIndex].position) < 0.001);
      selectedStopIndex = dragStopIndex;
      // Final update with full LUT generation
      updateGradient();
      
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }

  function getColorAtPosition(position: number): string {
    // Clamp position to [0, 1]
    position = Math.max(0, Math.min(1, position));
    
    // Handle edge cases quickly
    if (position <= gradientStops[0].position) {
      return gradientStops[0].color;
    }
    if (position >= gradientStops[gradientStops.length - 1].position) {
      return gradientStops[gradientStops.length - 1].color;
    }

    // Binary search for better performance with many stops
    let left = 0;
    let right = gradientStops.length - 1;
    
    while (right - left > 1) {
      const mid = Math.floor((left + right) / 2);
      if (gradientStops[mid].position <= position) {
        left = mid;
      } else {
        right = mid;
      }
    }

    const leftStop = gradientStops[left];
    const rightStop = gradientStops[right];
    const t = (position - leftStop.position) / (rightStop.position - leftStop.position);
    
    return interpolateColor(leftStop.color, rightStop.color, t);
  }

  // Immediate visual update during dragging (no backend call)
  function updateGradientImmediate() {
    // Just trigger reactivity for visual feedback
    gradientStops = [...gradientStops];
  }

  // Handle color input
  function handleColorInput() {
    updateGradient();
  }

  // Handle position input with sorting
  function handlePositionInput() {
    gradientStops = [...gradientStops].sort((a, b) => a.position - b.position);
    selectedStopIndex = gradientStops.findIndex(stop => Math.abs(stop.position - gradientStops[selectedStopIndex].position) < 0.001);
    updateGradient();
  }

  // Generate LUT data from current gradient stops
  function generateLutData(): number[] {
    const rArr: number[] = [];
    const gArr: number[] = [];
    const bArr: number[] = [];
    
    for (let i = 0; i < 256; i++) {
      const t = i / 255;
      const color = getColorAtPosition(t);
      const r = parseInt(color.slice(1, 3), 16);
      const g = parseInt(color.slice(3, 5), 16);
      const b = parseInt(color.slice(5, 7), 16);
      rArr.push(r);
      gArr.push(g);
      bArr.push(b);
    }
    
    // Create planar format: [r0, r1, ..., r255, g0, g1, ..., g255, b0, b1, ..., b255]
    return [...rArr, ...gArr, ...bArr];
  }

  async function updateGradient() {
    // Clear any existing timeout
    if (updateTimeout) {
      clearTimeout(updateTimeout);
    }
    
    // Debounce the LUT update to avoid too many rapid calls
    updateTimeout = setTimeout(async () => {
      try {
        const lutData = generateLutData();
        
        // Single optimized call to update gradient preview
        await invoke('update_gradient_preview', { lutData });
      } catch (e) {
        console.error('Failed to update gradient:', e);
      }
    }, 50); // Reduced debounce for better responsiveness
  }



  function applyPreset() {
    switch (selectedPreset) {
      case 'Rainbow':
        gradientStops = [
          { position: 0, color: '#ff0000' },
          { position: 0.17, color: '#ff8000' },
          { position: 0.33, color: '#ffff00' },
          { position: 0.5, color: '#00ff00' },
          { position: 0.67, color: '#0080ff' },
          { position: 0.83, color: '#8000ff' },
          { position: 1, color: '#ff0080' }
        ];
        break;
      case 'Heat':
        gradientStops = [
          { position: 0, color: '#000000' },
          { position: 0.5, color: '#ff0000' },
          { position: 1, color: '#ffff00' }
        ];
        break;
      case 'Cool':
        gradientStops = [
          { position: 0, color: '#0000ff' },
          { position: 0.5, color: '#00ffff' },
          { position: 1, color: '#ffffff' }
        ];
        break;
      case 'Viridis':
        gradientStops = [
          { position: 0, color: '#440154' },
          { position: 0.25, color: '#31688e' },
          { position: 0.5, color: '#35b779' },
          { position: 0.75, color: '#fde725' },
          { position: 1, color: '#fde725' }
        ];
        break;
      case 'Plasma':
        gradientStops = [
          { position: 0, color: '#0d0887' },
          { position: 0.25, color: '#7e03a8' },
          { position: 0.5, color: '#cc4778' },
          { position: 0.75, color: '#f89441' },
          { position: 1, color: '#f0f921' }
        ];
        break;
      case 'Inferno':
        gradientStops = [
          { position: 0, color: '#000004' },
          { position: 0.25, color: '#1b0c41' },
          { position: 0.5, color: '#4a0c6b' },
          { position: 0.75, color: '#781c6d' },
          { position: 1, color: '#ed6925' }
        ];
        break;
    }
    selectedStopIndex = 0;
    updateGradient();
  }

  function reverseGradient() {
    gradientStops = gradientStops.map(stop => ({
      ...stop,
      position: 1 - stop.position
    })).sort((a, b) => a.position - b.position);
    updateGradient();
  }

  function triggerRandomization() {
    randomizeGradient(selectedRandomScheme);
  }

  function generateRandomColors(scheme: string): string[] {
    let colors: string[] = [];
    
    switch (scheme) {
      case 'Basic':
        colors = ['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#ff00ff', '#00ffff', '#ff8000', '#8000ff'];
        break;
      case 'Warm':
        colors = ['#ff4500', '#ff6347', '#ffa500', '#ff8c00', '#dc143c', '#b22222', '#cd853f', '#d2691e'];
        break;
      case 'Cool':
        colors = ['#4169e1', '#0000cd', '#1e90ff', '#00bfff', '#87ceeb', '#20b2aa', '#008b8b', '#4682b4'];
        break;
      case 'Pastel':
        colors = ['#ffb3ba', '#ffdfba', '#ffffba', '#baffc9', '#bae1ff', '#e6baff', '#ffc9ba', '#c9baff'];
        break;
      case 'Neon':
        colors = ['#ff073a', '#39ff14', '#00ffff', '#ff00ff', '#ffff00', '#ff4500', '#8a2be2', '#00ff7f'];
        break;
      case 'Earth':
        colors = ['#8b4513', '#a0522d', '#cd853f', '#daa520', '#b8860b', '#9acd32', '#6b8e23', '#556b2f'];
        break;
      case 'Monochrome':
        const baseHue = Math.floor(Math.random() * 360);
        colors = [];
        for (let i = 0; i < 8; i++) {
          const saturation = 50 + Math.random() * 50; // 50-100%
          const lightness = 20 + Math.random() * 60; // 20-80%
          colors.push(hslToHex(baseHue, saturation, lightness));
        }
        break;
      case 'Complementary':
        const hue1 = Math.floor(Math.random() * 360);
        const hue2 = (hue1 + 180) % 360;
        colors = [
          hslToHex(hue1, 70, 50),
          hslToHex(hue1, 80, 30),
          hslToHex(hue1, 60, 70),
          hslToHex(hue2, 70, 50),
          hslToHex(hue2, 80, 30),
          hslToHex(hue2, 60, 70)
        ];
        break;
      case 'Truly Random':
        colors = [];
        for (let i = 0; i < 8; i++) {
          const r = Math.floor(Math.random() * 256);
          const g = Math.floor(Math.random() * 256);
          const b = Math.floor(Math.random() * 256);
          colors.push(`#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`);
        }
        break;
    }
    
    return colors;
  }

  function generateStopPositions(count: number, placement: 'Even' | 'Random'): number[] {
    if (placement === 'Even') {
      // Generate evenly spaced positions
      const positions: number[] = [];
      for (let i = 0; i < count; i++) {
        positions.push(i / (count - 1));
      }
      return positions;
    } else {
      // Generate random positions
      const positions: number[] = [0, 1]; // Always include start and end
      
      // Add intermediate positions
      for (let i = 2; i < count; i++) {
        positions.push(0.1 + Math.random() * 0.8); // Avoid edges
      }
      
      return positions.sort((a, b) => a - b);
    }
  }



  function randomizeGradient(scheme: string = 'Basic') {
    // Generate colors for the scheme
    const colors = generateRandomColors(scheme);
    
    // Use the slider value for stop count
    const stopCount = randomStopCount;
    
    // Generate stop positions
    const positions = generateStopPositions(stopCount, randomStopPlacement);
    
    // Create gradient stops by combining positions and random colors
    gradientStops = positions.map(position => ({
      position,
      color: colors[Math.floor(Math.random() * colors.length)]
    }));
    
    selectedStopIndex = 0;
    updateGradient();
  }

  async function saveLUT() {
    if (!lutName.trim()) return;
    try {
      const rArr: number[] = [];
      const gArr: number[] = [];
      const bArr: number[] = [];
      
      for (let i = 0; i < 256; i++) {
        const t = i / 255;
        const color = getColorAtPosition(t);
        const r = parseInt(color.slice(1, 3), 16);
        const g = parseInt(color.slice(3, 5), 16);
        const b = parseInt(color.slice(5, 7), 16);
        rArr.push(r);
        gArr.push(g);
        bArr.push(b);
      }
      
      const lutData = [...rArr, ...gArr, ...bArr];
      await invoke('save_custom_lut', { name: lutName, lutData });
      
      // Clear the temporary LUT
      await invoke('clear_temp_lut');
      
      // Update the gradient simulation with the new LUT
      await invoke('apply_lut_by_name', { lutName });
      
      // Clear any existing timeout
      if (saveSuccessTimeout) {
        clearTimeout(saveSuccessTimeout);
      }
      
      // Show success message
      saveSuccessMessage = `LUT "${lutName}" saved successfully!`;
      saveSuccessTimeout = setTimeout(() => {
        saveSuccessMessage = '';
      }, 3000); // Hide after 3 seconds
      
    } catch (e) {
      console.error('Failed to save LUT:', e);
      // Clear any existing timeout
      if (saveSuccessTimeout) {
        clearTimeout(saveSuccessTimeout);
      }
      // Show error message
      saveSuccessMessage = 'Failed to save LUT';
      saveSuccessTimeout = setTimeout(() => {
        saveSuccessMessage = '';
      }, 3000);
    }
  }

  function exportLUT() {
    const lutData = [];
    for (let i = 0; i < 256; i++) {
      const t = i / 255;
      const color = getColorAtPosition(t);
      const r = parseInt(color.slice(1, 3), 16);
      const g = parseInt(color.slice(3, 5), 16);
      const b = parseInt(color.slice(5, 7), 16);
      lutData.push(r, g, b);
    }
    
    const dataStr = lutData.join('\n');
    const dataBlob = new Blob([dataStr], { type: 'text/plain' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${lutName || 'custom'}.lut`;
    link.click();
    URL.revokeObjectURL(url);
  }

  function goBack() {
    // Clear the temporary LUT before going back
    invoke('clear_temp_lut').catch(e => {
      console.error('Failed to clear temporary LUT:', e);
    });
    dispatch('back');
  }

  async function toggleBackendGui() {
    try {
      await invoke('toggle_gui');
      const visible = (await invoke('get_gui_state')) as boolean;
      showUI = visible;

      if (!showUI) {
        showControls();
        startAutoHideTimer();
      } else {
        stopAutoHideTimer();
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

  async function togglePause() {
    if (running) {
      await stopSimulation();
    } else {
      await resumeSimulation();
    }
  }

  function handleUserInteraction() {
    if (!showUI && !controlsVisible) {
      showControls();
      startAutoHideTimer();
    } else if (!showUI && controlsVisible) {
      startAutoHideTimer();
    }
  }

  function handleMouseEvent(_event: CustomEvent) {
    // Handle mouse events if needed
  }

  // Auto-hide functionality
  function startAutoHideTimer() {
    stopAutoHideTimer();
    hideTimeout = window.setTimeout(() => {
      if (!showUI) {
        controlsVisible = false;
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

  function hslToHex(h: number, s: number, l: number): string {
    s /= 100;
    l /= 100;
    const c = (1 - Math.abs(2 * l - 1)) * s;
    const x = c * (1 - Math.abs((h / 60) % 2 - 1));
    const m = l - c / 2;

    let r = 0;
    let g = 0;
    let b = 0;

    if (h < 60) {
      r = c;
      g = x;
      b = 0;
    } else if (h < 120) {
      r = x;
      g = c;
      b = 0;
    } else if (h < 180) {
      r = 0;
      g = c;
      b = x;
    } else if (h < 240) {
      r = 0;
      g = x;
      b = c;
    } else if (h < 300) {
      r = x;
      g = 0;
      b = c;
    } else {
      r = c;
      g = 0;
      b = x;
    }

    r = Math.round((r + m) * 255);
    g = Math.round((g + m) * 255);
    b = Math.round((b + m) * 255);

    const hexR = r.toString(16).padStart(2, '0');
    const hexG = g.toString(16).padStart(2, '0');
    const hexB = b.toString(16).padStart(2, '0');

    return `#${hexR}${hexG}${hexB}`;
  }


  onMount(async () => {
    try {
      // Listen for simulation-initialized event
      unlistenSimulationInitialized = await listen('simulation-initialized', async () => {
        // Now that simulation is initialized, update gradient
        updateGradient();
      });

      // Start gradient simulation
      await invoke('start_simulation', { simulationType: 'gradient' });

      // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts with CameraControls)
      const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
      events.forEach((event) => {
        window.addEventListener(event, handleUserInteraction, { passive: true });
      });
    } catch (e) {
      console.error('Failed to start gradient simulation:', e);
    }
  });

  onDestroy(async () => {
    try {
      // Clear the temporary LUT
      await invoke('clear_temp_lut');
      
      await invoke('destroy_simulation');
    } catch (error) {
      console.error('Failed to destroy simulation on component destroy:', error);
    }

    if (unlistenSimulationInitialized) {
      unlistenSimulationInitialized();
    }

    // Remove event listeners for auto-hide functionality
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.removeEventListener(event, handleUserInteraction);
    });

    // Clear any remaining timeouts
    if (hideTimeout) {
      clearTimeout(hideTimeout);
    }
  });
</script>

<style>
  /* Gradient editor specific styles */
  
  /* Custom GEM Header */
  .gem-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    background: rgba(26, 26, 26, 0.95);
    border-bottom: 1px solid #444;
    margin-bottom: 1rem;
  }

  .header-left h2 {
    margin: 0;
    font-size: 1.5rem;
    color: #646cff;
  }

  .header-right {
    display: flex;
    gap: 0.5rem;
  }

  .save-button {
    background: #646cff;
    color: white;
    border: 1px solid #646cff;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .save-button:hover:not(:disabled) {
    background: #535bf2;
    border-color: #535bf2;
    transform: translateY(-1px);
    box-shadow: 0 4px 8px rgba(100, 108, 255, 0.3);
  }

  .save-button:disabled {
    background: #adb5bd;
    border-color: #adb5bd;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .gradient-stops-overlay {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }

  .stops-container {
    width: 100%;
    max-width: 800px;
  }

  .gradient-bar {
    height: 80px;
    border-radius: 12px;
    position: relative;
    cursor: crosshair;
    border: 3px solid rgba(255, 255, 255, 0.3);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  }

  .color-stop {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 24px;
    height: 48px;
    border: 3px solid white;
    border-radius: 6px;
    cursor: grab;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    transition: all 0.2s ease;
  }

  .color-stop:hover {
    transform: translate(-50%, -50%) scale(1.15);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.6);
  }

  .color-stop.selected {
    border-color: #646cff;
    box-shadow: 0 6px 24px rgba(100, 108, 255, 0.6);
  }

  .stop-handle {
    width: 100%;
    height: 100%;
    background: inherit;
    border-radius: 3px;
  }

  .control-panel {
    background: rgba(26, 26, 26, 0.95);
    border-top: 1px solid #444;
    padding: 0.75rem 1rem;
    overflow-y: auto;
    max-height: 400px;
  }

  .control-header {
    margin-bottom: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid #444;
  }

  .header-grid {
    display: grid;
    grid-template-columns: auto auto;
    gap: 1rem;
    align-items: center;
    justify-content: start;
  }

  .name-section {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .name-section label {
    font-weight: 500;
    color: #ccc;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .lut-name-input {
    background: #333;
    border: 1px solid #555;
    color: white;
    padding: 0.25rem 0.4rem;
    border-radius: 3px;
    width: 120px;
    font-size: 0.8rem;
  }

  .preset-section {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .preset-section label {
    font-weight: 500;
    color: #ccc;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .space-section,
  .display-section,
  .stops-section {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .space-section label,
  .display-section label,
  .stops-section label {
    font-weight: 500;
    color: #ccc;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .stops-count {
    font-weight: 500;
    color: #646cff;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .stop-controls {
    margin-bottom: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid #444;
  }

  .stop-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .stop-title {
    font-weight: 500;
    color: #646cff;
    font-size: 0.9rem;
  }

  .stop-actions {
    display: flex;
    gap: 0.3rem;
  }

  .stop-controls-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .control-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .control-item label {
    font-weight: 500;
    color: #ccc;
    font-size: 0.75rem;
    white-space: nowrap;
  }

  .color-picker {
    width: 100%;
    height: 24px;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  .position-slider {
    width: 100%;
    height: 20px;
  }

  .actions-section {
    margin-top: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid #444;
  }

  .actions-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
    align-items: center;
  }

  .action-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .action-group label {
    font-weight: 500;
    color: #ccc;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .button-group {
    display: flex;
    gap: 0.3rem;
  }

  .random-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .random-group label {
    font-weight: 500;
    color: #ccc;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .random-controls {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .random-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .stops-control {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .stops-control span {
    font-weight: 500;
    color: #646cff;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .stops-range {
    width: 50px;
    height: 18px;
  }

  .btn-action {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 3px;
    border: 1px solid #555;
    background: #333;
    color: white;
    cursor: pointer;
    transition: background-color 0.2s;
    white-space: nowrap;
  }

  .btn-action:hover {
    background: #444;
  }

  .btn-compact {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 3px;
    border: 1px solid #555;
    background: #333;
    color: white;
    cursor: pointer;
    transition: background-color 0.2s;
    white-space: nowrap;
  }

  .btn-compact:hover {
    background: #444;
  }

  .btn-compact.btn-danger {
    background: #dc3545;
    border-color: #dc3545;
  }

  .btn-compact.btn-danger:hover {
    background: #c82333;
  }

  .btn-generate {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 3px;
    border: 1px solid #555;
    background: #333;
    color: white;
    cursor: pointer;
    transition: background-color 0.2s;
    white-space: nowrap;
  }

  .btn-generate:hover {
    background: #444;
  }

  .success-message {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    background-color: #4CAF50; /* Green background */
    color: white;
    padding: 10px 20px;
    border-radius: 5px;
    z-index: 1001; /* Above other content */
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    animation: fadeInOut 3s ease-in-out;
  }

  @keyframes fadeInOut {
    0% { opacity: 0; }
    20% { opacity: 1; }
    80% { opacity: 1; }
    100% { opacity: 0; }
  }
</style>