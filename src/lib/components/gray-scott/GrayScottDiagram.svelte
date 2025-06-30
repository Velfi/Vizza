<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  
  export let feedRate: number = 0.055;
  export let killRate: number = 0.062;
  export let diffusionRateU: number = 0.1;
  export let diffusionRateV: number = 0.05;
  export let timestep: number = 1.0;

  const dispatch = createEventDispatcher();

  // Canvas dimensions
  const width = 800;
  const height = 600;
  const margin = 60;

  // Internal state (not reactive to props)
  let internalFeedRate = feedRate;
  let internalKillRate = killRate;
  let internalDiffusionRateU = diffusionRateU;
  let internalDiffusionRateV = diffusionRateV;
  let internalTimestep = timestep;

  // Canvas and context
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;

  // Dragging state
  let isDragging = false;
  let dragTarget: 'feedKill' | 'diffusion' | 'timestep' | null = null;
  let lastUpdateTime = 0;
  const updateThrottle = 50; // ms between updates
  
  // Track which values were updated internally to prevent feedback loops
  let lastInternalUpdate = {
    feedRate: 0,
    killRate: 0,
    diffusionRateU: 0,
    diffusionRateV: 0,
    timestep: 0
  };

  // Handle positions
  let feedKillHandle = { x: 0, y: 0 };
  let diffusionHandle = { x: 0, y: 0 };

  // Parameter ranges
  const feedRateRange = { min: 0.02, max: 0.08 };
  const killRateRange = { min: 0.04, max: 0.08 };
  const diffusionRange = { min: 0.05, max: 0.3 };
  const timestepRange = { min: 0.1, max: 3.0 };

  // Layout calculations
  const timestepHeight = 80;
  const plotAreaHeight = height - timestepHeight - 40;
  const plotSize = Math.min((width - 3 * margin) / 2, plotAreaHeight - 2 * margin);
  const leftPlotX = margin;
  const rightPlotX = width - margin - plotSize;
  const plotY = margin;

  // Convert coordinates for XY plots
  function toCanvasX(value: number, range: { min: number, max: number }, plotX: number): number {
    const normalized = (value - range.min) / (range.max - range.min);
    return plotX + normalized * plotSize;
  }

  function toCanvasY(value: number, range: { min: number, max: number }, plotY: number): number {
    const normalized = (value - range.min) / (range.max - range.min);
    return plotY + plotSize - normalized * plotSize;
  }

  function fromCanvasX(x: number, range: { min: number, max: number }, plotX: number): number {
    const normalized = (x - plotX) / plotSize;
    return range.min + normalized * (range.max - range.min);
  }

  function fromCanvasY(y: number, range: { min: number, max: number }, plotY: number): number {
    const normalized = (plotY + plotSize - y) / plotSize;
    return range.min + normalized * (range.max - range.min);
  }

  // Update handle positions
  function updateHandlePositions() {
    // Feed/Kill handle position (X = feed rate, Y = kill rate)
    feedKillHandle.x = toCanvasX(internalFeedRate, feedRateRange, leftPlotX);
    feedKillHandle.y = toCanvasY(internalKillRate, killRateRange, plotY);
    
    // Diffusion handle position (X = diffusion U, Y = diffusion V)
    diffusionHandle.x = toCanvasX(internalDiffusionRateU, diffusionRange, rightPlotX);
    diffusionHandle.y = toCanvasY(internalDiffusionRateV, diffusionRange, plotY);
  }

  // Draw the diagram
  function draw() {
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Update handle positions
    updateHandlePositions();

    // Draw background
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, width, height);

    // Draw the three sections
    drawTimestepSection();
    drawFeedKillSection();
    drawDiffusionSection();
  }

  function drawTimestepSection() {
    const sectionHeight = timestepHeight;
    const y = height - sectionHeight - 20;
    
    // Background
    ctx.fillStyle = '#1f2937';
    ctx.fillRect(0, y, width, sectionHeight);
    ctx.strokeStyle = '#374151';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, y, width, sectionHeight);

    // Title
    ctx.fillStyle = '#ffffff';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('Timestep (Δt)', width / 2, y + 25);

    // Slider track
    const trackY = y + 45;
    const trackHeight = 8;
    const trackX = margin;
    const trackWidth = width - 2 * margin;
    
    ctx.fillStyle = '#374151';
    ctx.fillRect(trackX, trackY, trackWidth, trackHeight);
    ctx.strokeStyle = '#4b5563';
    ctx.lineWidth = 1;
    ctx.strokeRect(trackX, trackY, trackWidth, trackHeight);

    // Slider handle
    const handleX = trackX + ((internalTimestep - timestepRange.min) / (timestepRange.max - timestepRange.min)) * trackWidth;
    const handleY = trackY - 4;
    const handleRadius = 12;
    
    ctx.fillStyle = '#a855f7';
    ctx.strokeStyle = '#9333ea';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(handleX, handleY, handleRadius, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();

    // Value label
    ctx.fillStyle = '#fbbf24';
    ctx.font = '14px monospace';
    ctx.fillText(internalTimestep.toFixed(1), handleX, trackY - 15);

    // Range labels
    ctx.fillStyle = '#9ca3af';
    ctx.font = '12px sans-serif';
    ctx.fillText(timestepRange.min.toString(), trackX, trackY + 25);
    ctx.fillText(timestepRange.max.toString(), trackX + trackWidth, trackY + 25);
  }

  function drawFeedKillSection() {
    // Background
    ctx.fillStyle = '#1f2937';
    ctx.fillRect(leftPlotX - 10, plotY - 30, plotSize + 20, plotSize + 60);
    ctx.strokeStyle = '#374151';
    ctx.lineWidth = 1;
    ctx.strokeRect(leftPlotX - 10, plotY - 30, plotSize + 20, plotSize + 60);

    // Title
    ctx.fillStyle = '#ffffff';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('Feed Rate (F) vs Kill Rate (K)', leftPlotX + plotSize / 2, plotY - 10);

    // Grid
    ctx.strokeStyle = '#374151';
    ctx.lineWidth = 1;
    ctx.setLineDash([2, 2]);
    
    // Vertical grid lines
    for (let i = 1; i < 10; i++) {
      const x = leftPlotX + (i / 10) * plotSize;
      ctx.beginPath();
      ctx.moveTo(x, plotY);
      ctx.lineTo(x, plotY + plotSize);
      ctx.stroke();
    }
    
    // Horizontal grid lines
    for (let i = 1; i < 10; i++) {
      const gridY = plotY + (i / 10) * plotSize;
      ctx.beginPath();
      ctx.moveTo(leftPlotX, gridY);
      ctx.lineTo(leftPlotX + plotSize, gridY);
      ctx.stroke();
    }
    ctx.setLineDash([]);

    // Plot border
    ctx.strokeStyle = '#4b5563';
    ctx.lineWidth = 2;
    ctx.strokeRect(leftPlotX, plotY, plotSize, plotSize);

    // Axis labels
    ctx.fillStyle = '#9ca3af';
    ctx.font = '12px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('Feed Rate (F)', leftPlotX + plotSize / 2, plotY + plotSize + 20);
    
    ctx.save();
    ctx.translate(leftPlotX - 15, plotY + plotSize / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Kill Rate (K)', 0, 0);
    ctx.restore();

    // Range labels
    ctx.fillText(feedRateRange.min.toFixed(2), leftPlotX, plotY + plotSize + 20);
    ctx.fillText(feedRateRange.max.toFixed(2), leftPlotX + plotSize, plotY + plotSize + 20);
    ctx.fillText(killRateRange.min.toFixed(2), leftPlotX - 5, plotY + plotSize);
    ctx.fillText(killRateRange.max.toFixed(2), leftPlotX - 5, plotY);

    // Handle
    const handleRadius = 8;
    ctx.fillStyle = '#ef4444';
    ctx.strokeStyle = '#dc2626';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(feedKillHandle.x, feedKillHandle.y, handleRadius, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();

    // Value labels
    ctx.fillStyle = '#fbbf24';
    ctx.font = '12px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(`F: ${internalFeedRate.toFixed(3)}`, feedKillHandle.x, feedKillHandle.y - 15);
    ctx.fillText(`K: ${internalKillRate.toFixed(3)}`, feedKillHandle.x, feedKillHandle.y + 25);
  }

  function drawDiffusionSection() {
    // Background
    ctx.fillStyle = '#1f2937';
    ctx.fillRect(rightPlotX - 10, plotY - 30, plotSize + 20, plotSize + 60);
    ctx.strokeStyle = '#374151';
    ctx.lineWidth = 1;
    ctx.strokeRect(rightPlotX - 10, plotY - 30, plotSize + 20, plotSize + 60);

    // Title
    ctx.fillStyle = '#ffffff';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('Diffusion Rate U (Du) vs Diffusion Rate V (Dv)', rightPlotX + plotSize / 2, plotY - 10);

    // Grid
    ctx.strokeStyle = '#374151';
    ctx.lineWidth = 1;
    ctx.setLineDash([2, 2]);
    
    // Vertical grid lines
    for (let i = 1; i < 10; i++) {
      const x = rightPlotX + (i / 10) * plotSize;
      ctx.beginPath();
      ctx.moveTo(x, plotY);
      ctx.lineTo(x, plotY + plotSize);
      ctx.stroke();
    }
    
    // Horizontal grid lines
    for (let i = 1; i < 10; i++) {
      const gridY = plotY + (i / 10) * plotSize;
      ctx.beginPath();
      ctx.moveTo(rightPlotX, gridY);
      ctx.lineTo(rightPlotX + plotSize, gridY);
      ctx.stroke();
    }
    ctx.setLineDash([]);

    // Plot border
    ctx.strokeStyle = '#4b5563';
    ctx.lineWidth = 2;
    ctx.strokeRect(rightPlotX, plotY, plotSize, plotSize);

    // Axis labels
    ctx.fillStyle = '#9ca3af';
    ctx.font = '12px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('Diffusion Rate U (Du)', rightPlotX + plotSize / 2, plotY + plotSize + 20);
    
    ctx.save();
    ctx.translate(rightPlotX - 15, plotY + plotSize / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Diffusion Rate V (Dv)', 0, 0);
    ctx.restore();

    // Range labels
    ctx.fillText(diffusionRange.min.toFixed(2), rightPlotX, plotY + plotSize + 20);
    ctx.fillText(diffusionRange.max.toFixed(2), rightPlotX + plotSize, plotY + plotSize + 20);
    ctx.fillText(diffusionRange.min.toFixed(2), rightPlotX - 5, plotY + plotSize);
    ctx.fillText(diffusionRange.max.toFixed(2), rightPlotX - 5, plotY);

    // Handle
    const handleRadius = 8;
    ctx.fillStyle = '#22c55e';
    ctx.strokeStyle = '#16a34a';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(diffusionHandle.x, diffusionHandle.y, handleRadius, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();

    // Value labels
    ctx.fillStyle = '#fbbf24';
    ctx.font = '12px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(`Du: ${internalDiffusionRateU.toFixed(3)}`, diffusionHandle.x, diffusionHandle.y - 15);
    ctx.fillText(`Dv: ${internalDiffusionRateV.toFixed(3)}`, diffusionHandle.x, diffusionHandle.y + 25);
  }

  // Check if point is near handle
  function isNearHandle(x: number, y: number, handle: { x: number, y: number }): boolean {
    const distance = Math.sqrt((x - handle.x) ** 2 + (y - handle.y) ** 2);
    return distance <= 12;
  }

  // Check if point is in feed/kill plot area
  function isInFeedKillPlot(x: number, y: number): boolean {
    return x >= leftPlotX && x <= leftPlotX + plotSize && 
           y >= plotY && y <= plotY + plotSize;
  }

  // Check if point is in diffusion plot area
  function isInDiffusionPlot(x: number, y: number): boolean {
    return x >= rightPlotX && x <= rightPlotX + plotSize && 
           y >= plotY && y <= plotY + plotSize;
  }

  // Check if point is near timestep slider
  function isNearTimestepSlider(x: number, y: number): boolean {
    const sectionHeight = timestepHeight;
    const sectionY = height - sectionHeight - 20;
    const trackY = sectionY + 45;
    const trackHeight = 8;
    const trackX = margin;
    const trackWidth = width - 2 * margin;
    
    return x >= trackX && x <= trackX + trackWidth && 
           y >= trackY - 12 && y <= trackY + trackHeight + 12;
  }

  // Handle mouse events
  function handlePointerDown(event: PointerEvent) {
    event.preventDefault();
    
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    if (isNearHandle(x, y, feedKillHandle) || isInFeedKillPlot(x, y)) {
      isDragging = true;
      dragTarget = 'feedKill';
    } else if (isNearHandle(x, y, diffusionHandle) || isInDiffusionPlot(x, y)) {
      isDragging = true;
      dragTarget = 'diffusion';
    } else if (isNearTimestepSlider(x, y)) {
      isDragging = true;
      dragTarget = 'timestep';
    }

    if (isDragging) {
      lastUpdateTime = Date.now();
      canvas.setPointerCapture(event.pointerId);
    }
  }

  function handlePointerMove(event: PointerEvent) {
    if (!isDragging || !dragTarget) return;
    
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    // Throttle updates to prevent feedback loops
    const now = Date.now();
    if (now - lastUpdateTime < updateThrottle) {
      return;
    }
    lastUpdateTime = now;
    
    switch (dragTarget) {
      case 'feedKill':
        const newFeedRate = Math.max(feedRateRange.min, Math.min(feedRateRange.max, fromCanvasX(x, feedRateRange, leftPlotX)));
        const newKillRate = Math.max(killRateRange.min, Math.min(killRateRange.max, fromCanvasY(y, killRateRange, plotY)));
        internalFeedRate = newFeedRate;
        internalKillRate = newKillRate;
        lastInternalUpdate.feedRate = newFeedRate;
        lastInternalUpdate.killRate = newKillRate;
        dispatch('update', { setting: 'feed_rate', value: newFeedRate });
        dispatch('update', { setting: 'kill_rate', value: newKillRate });
        console.log('Dispatched update for feed/kill:', newFeedRate, newKillRate);
        break;
        
      case 'diffusion':
        const newDiffusionU = Math.max(diffusionRange.min, Math.min(diffusionRange.max, fromCanvasX(x, diffusionRange, rightPlotX)));
        const newDiffusionV = Math.max(diffusionRange.min, Math.min(diffusionRange.max, fromCanvasY(y, diffusionRange, plotY)));
        internalDiffusionRateU = newDiffusionU;
        internalDiffusionRateV = newDiffusionV;
        lastInternalUpdate.diffusionRateU = newDiffusionU;
        lastInternalUpdate.diffusionRateV = newDiffusionV;
        dispatch('update', { setting: 'diffusion_rate_u', value: newDiffusionU });
        dispatch('update', { setting: 'diffusion_rate_v', value: newDiffusionV });
        console.log('Dispatched update for diffusion:', newDiffusionU, newDiffusionV);
        break;
        
      case 'timestep':
        const trackX = margin;
        const trackWidth = width - 2 * margin;
        const normalizedX = Math.max(0, Math.min(1, (x - trackX) / trackWidth));
        const newTimestep = timestepRange.min + normalizedX * (timestepRange.max - timestepRange.min);
        internalTimestep = newTimestep;
        lastInternalUpdate.timestep = newTimestep;
        dispatch('update', { setting: 'timestep', value: newTimestep });
        break;
    }
    
    draw();
  }

  function handlePointerUp(event: PointerEvent) {
    if (isDragging) {
      isDragging = false;
      dragTarget = null;
      canvas.releasePointerCapture(event.pointerId);
    }
  }

  // Sync with external props when they change
  $: if (feedRate !== internalFeedRate && !isDragging && Math.abs(feedRate - lastInternalUpdate.feedRate) > 0.0001) {
    internalFeedRate = feedRate;
    draw();
  }
  
  $: if (killRate !== internalKillRate && !isDragging && Math.abs(killRate - lastInternalUpdate.killRate) > 0.0001) {
    internalKillRate = killRate;
    draw();
  }
  
  $: if (diffusionRateU !== internalDiffusionRateU && !isDragging && Math.abs(diffusionRateU - lastInternalUpdate.diffusionRateU) > 0.0001) {
    internalDiffusionRateU = diffusionRateU;
    draw();
  }
  
  $: if (diffusionRateV !== internalDiffusionRateV && !isDragging && Math.abs(diffusionRateV - lastInternalUpdate.diffusionRateV) > 0.0001) {
    internalDiffusionRateV = diffusionRateV;
    draw();
  }
  
  $: if (timestep !== internalTimestep && !isDragging && Math.abs(timestep - lastInternalUpdate.timestep) > 0.01) {
    internalTimestep = timestep;
    draw();
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
    
    // Update internal values
    internalFeedRate = defaults.feed_rate;
    internalKillRate = defaults.kill_rate;
    internalDiffusionRateU = defaults.diffusion_rate_u;
    internalDiffusionRateV = defaults.diffusion_rate_v;
    internalTimestep = defaults.timestep;
    
    // Dispatch updates to parent
    dispatch('update', { setting: 'feed_rate', value: defaults.feed_rate });
    dispatch('update', { setting: 'kill_rate', value: defaults.kill_rate });
    dispatch('update', { setting: 'diffusion_rate_u', value: defaults.diffusion_rate_u });
    dispatch('update', { setting: 'diffusion_rate_v', value: defaults.diffusion_rate_v });
    dispatch('update', { setting: 'timestep', value: defaults.timestep });
    
    // Redraw the diagram
    draw();
  }

  // Initialize internal values on mount
  onMount(() => {
    ctx = canvas.getContext('2d')!;
    
    // Initialize internal values from props
    internalFeedRate = feedRate;
    internalKillRate = killRate;
    internalDiffusionRateU = diffusionRateU;
    internalDiffusionRateV = diffusionRateV;
    internalTimestep = timestep;
    
    draw();
  });
</script>

<div class="instructions">
  <div class="top-controls">
    <span>Drag the handles to adjust reaction-diffusion parameters</span>
    <button class="reset-button" on:click={resetToDefaults}>
      Reset to Defaults
    </button>
  </div>
</div>

<canvas 
  bind:this={canvas}
  {width} 
  {height} 
  class="reaction-diagram"
  on:pointerdown={handlePointerDown}
  on:pointermove={handlePointerMove}
  on:pointerup={handlePointerUp}
  aria-label="Interactive Gray-Scott reaction-diffusion diagram with draggable parameter handles"
>
</canvas>

<div class="parameter-display">
  <div class="parameter-grid">
    <div class="parameter-item">
      <span class="parameter-label">Feed Rate (F):</span>
      <span class="parameter-value">{internalFeedRate.toFixed(3)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Kill Rate (K):</span>
      <span class="parameter-value">{internalKillRate.toFixed(3)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Diffusion U (Du):</span>
      <span class="parameter-value">{internalDiffusionRateU.toFixed(3)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Diffusion V (Dv):</span>
      <span class="parameter-value">{internalDiffusionRateV.toFixed(3)}</span>
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
    <li><strong>🟣 Timestep Slider:</strong> Drag the purple handle to adjust simulation speed</li>
  </ul>
  <div class="equation-note">
    <small><strong>Equations:</strong> ∂u/∂t = Du∇²u - uv² + F(1-u) | ∂v/∂t = Dv∇²v + uv² - (K+F)v</small>
  </div>
</div>

<style>
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
  }
  
  .reset-button:hover {
    background: #4b5563;
  }
  
  .reaction-diagram {
    border: 1px solid #374151;
    border-radius: 0.5rem;
    background: #1a1a1a;
    display: block;
    margin: 0 auto;
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
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
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
  }
</style> 