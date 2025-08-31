<div class="instructions">
  <div class="top-controls">
    <span>Drag the threshold handle to see how cells become alive or die</span>
    <button class="reset-button" on:click={resetToDefaults}> Reset to Defaults </button>
  </div>
</div>

<canvas
  bind:this={canvas}
  {width}
  {height}
  class="threshold-diagram"
  on:pointerdown={handlePointerDown}
  on:pointermove={handlePointerMove}
  on:pointerup={handlePointerUp}
  aria-label="Interactive VCA threshold diagram showing alive ratio vs threshold"
>
</canvas>

<div class="parameter-display">
  <div class="parameter-grid">
    <div class="parameter-item">
      <span class="parameter-label">Alive Threshold:</span>
      <span class="parameter-value">{internalAliveThreshold.toFixed(3)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Neighbor Radius:</span>
      <span class="parameter-value">{internalNeighborRadius.toFixed(0)}px</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Sample Alive Ratio:</span>
      <span class="parameter-value">{sampleAliveRatio.toFixed(3)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Cell State:</span>
      <span class="parameter-value cell-state {sampleAliveRatio > internalAliveThreshold ? 'alive' : 'dead'}">
        {sampleAliveRatio > internalAliveThreshold ? 'ALIVE' : 'DEAD'}
      </span>
    </div>
    <div class="parameter-note">
      <strong>Rule:</strong> A cell becomes alive when its alive neighbor ratio exceeds the threshold.
      The alive ratio is calculated as: alive_neighbors / (alive_neighbors + dead_neighbors).
      <br><br>
      <strong>Neighborhood:</strong> Only cells within the neighbor radius are considered when calculating ratios.
      Larger radius = more neighbors considered, potentially smoother transitions.
    </div>
  </div>
</div>

<div class="sample-controls">
  <label for="sample-ratio-slider">
    <span class="sample-label">Sample Neighborhood Ratio:</span>
    <span class="sample-value">{sampleAliveRatio.toFixed(3)}</span>
  </label>
  <input
    id="sample-ratio-slider"
    type="range"
    min="0.0"
    max="1.0"
    step="0.01"
    bind:value={sampleAliveRatio}
    on:input={draw}
    class="sample-slider"
  />
</div>

<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  export let aliveThreshold: number = 0.5;
  export let neighborRadius: number = 60;

  const dispatch = createEventDispatcher();

  // Canvas properties
  export let width: number = 600;
  export let height: number = 450;
  let canvas: HTMLCanvasElement;

  // Internal state for smooth dragging
  let internalAliveThreshold = aliveThreshold;
  let internalNeighborRadius = neighborRadius;
  let sampleAliveRatio = 0.6; // Example neighborhood ratio

  // Layout constants
  const margin = 60;
  const plotWidth = width - 2 * margin;
  const plotHeight = height - 2 * margin - 100; // Reserve space for radius visualization
  const radiusVizHeight = 80;

  // Interaction state
  let isDragging = false;
  let dragTarget: 'threshold' | 'radius' | null = null;

  // Handle positions
  let thresholdHandle = { x: 0, y: 0, size: 12 };
  let radiusHandle = { x: 0, y: 0, size: 10 };

  function updateHandlePositions() {
    // Threshold handle - vertical line at threshold position
    thresholdHandle.x = margin + (internalAliveThreshold * plotWidth);
    thresholdHandle.y = margin + plotHeight / 2;
    
    // Radius handle - in the radius visualization area
    const radiusVizY = margin + plotHeight + 20;
    const maxRadius = 200; // Max radius for visualization
    const radiusRatio = Math.min(internalNeighborRadius / maxRadius, 1.0);
    radiusHandle.x = margin + (radiusRatio * plotWidth);
    radiusHandle.y = radiusVizY + radiusVizHeight / 2;
  }

  function draw() {
    const ctx = canvas?.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Update handle positions
    updateHandlePositions();

    // Draw background - dark theme
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, width, height);

    // Draw plot area background
    ctx.fillStyle = '#2a2a2a';
    ctx.fillRect(margin, margin, plotWidth, plotHeight);

    // Draw grid lines
    ctx.strokeStyle = '#444444';
    ctx.lineWidth = 1;
    ctx.setLineDash([2, 2]);

    // Vertical grid lines (ratio markers)
    for (let i = 0; i <= 10; i++) {
      const x = margin + (i / 10) * plotWidth;
      ctx.beginPath();
      ctx.moveTo(x, margin);
      ctx.lineTo(x, margin + plotHeight);
      ctx.stroke();
    }

    // Horizontal center line
    const centerY = margin + plotHeight / 2;
    ctx.beginPath();
    ctx.moveTo(margin, centerY);
    ctx.lineTo(margin + plotWidth, centerY);
    ctx.stroke();

    ctx.setLineDash([]);

    // Draw zones
    // Dead zone (left of threshold) - red
    ctx.fillStyle = 'rgba(239, 68, 68, 0.2)';
    ctx.strokeStyle = 'rgba(239, 68, 68, 0.4)';
    ctx.lineWidth = 2;
    ctx.fillRect(margin, margin, thresholdHandle.x - margin, plotHeight);
    ctx.strokeRect(margin, margin, thresholdHandle.x - margin, plotHeight);

    // Alive zone (right of threshold) - green
    ctx.fillStyle = 'rgba(34, 197, 94, 0.2)';
    ctx.strokeStyle = 'rgba(34, 197, 94, 0.4)';
    ctx.fillRect(thresholdHandle.x, margin, margin + plotWidth - thresholdHandle.x, plotHeight);
    ctx.strokeRect(thresholdHandle.x, margin, margin + plotWidth - thresholdHandle.x, plotHeight);

    // Draw threshold line
    ctx.strokeStyle = '#fbbf24';
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(thresholdHandle.x, margin);
    ctx.lineTo(thresholdHandle.x, margin + plotHeight);
    ctx.stroke();

    // Draw sample ratio indicator
    const sampleX = margin + sampleAliveRatio * plotWidth;
    const sampleColor = sampleAliveRatio > internalAliveThreshold ? '#22c55e' : '#ef4444';
    
    ctx.strokeStyle = sampleColor;
    ctx.lineWidth = 4;
    ctx.setLineDash([8, 4]);
    ctx.beginPath();
    ctx.moveTo(sampleX, margin);
    ctx.lineTo(sampleX, margin + plotHeight);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw sample point
    ctx.fillStyle = sampleColor;
    ctx.beginPath();
    ctx.arc(sampleX, centerY, 8, 0, 2 * Math.PI);
    ctx.fill();
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw threshold handle
    ctx.fillStyle = '#fbbf24';
    ctx.beginPath();
    ctx.arc(thresholdHandle.x, thresholdHandle.y, thresholdHandle.size, 0, 2 * Math.PI);
    ctx.fill();
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw axis labels
    ctx.fillStyle = '#ffffff';
    ctx.font = '14px monospace';
    ctx.textAlign = 'center';

    // X-axis labels
    ctx.fillText('0.0', margin, height - 20);
    ctx.fillText('0.5', margin + plotWidth / 2, height - 20);
    ctx.fillText('1.0', margin + plotWidth, height - 20);
    ctx.fillText('Alive Neighbor Ratio', width / 2, height - 5);

    // Y-axis label
    ctx.save();
    ctx.translate(15, height / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Cell State', 0, 0);
    ctx.restore();

    // Zone labels
    ctx.font = '16px monospace';
    ctx.fillStyle = '#ef4444';
    ctx.fillText('DEAD', margin + (thresholdHandle.x - margin) / 2, centerY - 20);
    
    ctx.fillStyle = '#22c55e';
    const aliveZoneWidth = margin + plotWidth - thresholdHandle.x;
    ctx.fillText('ALIVE', thresholdHandle.x + aliveZoneWidth / 2, centerY - 20);

    // Threshold value label
    ctx.fillStyle = '#fbbf24';
    ctx.font = '12px monospace';
    ctx.fillText(
      `Threshold: ${internalAliveThreshold.toFixed(3)}`,
      thresholdHandle.x,
      margin - 10
    );

    // Sample ratio label
    ctx.fillStyle = sampleColor;
    ctx.fillText(
      `Sample: ${sampleAliveRatio.toFixed(3)}`,
      sampleX,
      margin + plotHeight + 15
    );

    // Draw radius visualization section
    const radiusVizY = margin + plotHeight + 20;
    
    // Radius visualization background
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(margin, radiusVizY, plotWidth, radiusVizHeight);
    ctx.strokeStyle = '#444444';
    ctx.lineWidth = 1;
    ctx.strokeRect(margin, radiusVizY, plotWidth, radiusVizHeight);

    // Draw radius visualization - show a center cell with radius circle
    const radiusCenterX = margin + plotWidth / 2;
    const radiusCenterY = radiusVizY + radiusVizHeight / 2;
    const maxVisualizationRadius = Math.min(plotWidth, radiusVizHeight) / 3;
    const maxRadius = 200;
    const visualRadius = (internalNeighborRadius / maxRadius) * maxVisualizationRadius;

    // Draw radius circle
    ctx.strokeStyle = '#3b82f6';
    ctx.lineWidth = 2;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.arc(radiusCenterX, radiusCenterY, visualRadius, 0, 2 * Math.PI);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw center cell
    ctx.fillStyle = '#fbbf24';
    ctx.beginPath();
    ctx.arc(radiusCenterX, radiusCenterY, 6, 0, 2 * Math.PI);
    ctx.fill();

    // Draw some neighbor cells within radius
    const numNeighbors = Math.min(8, Math.floor(internalNeighborRadius / 20));
    for (let i = 0; i < numNeighbors; i++) {
      const angle = (i / numNeighbors) * 2 * Math.PI;
      const distance = visualRadius * 0.7;
      const nx = radiusCenterX + Math.cos(angle) * distance;
      const ny = radiusCenterY + Math.sin(angle) * distance;
      
      ctx.fillStyle = i < numNeighbors * sampleAliveRatio ? '#22c55e' : '#ef4444';
      ctx.beginPath();
      ctx.arc(nx, ny, 4, 0, 2 * Math.PI);
      ctx.fill();
    }

    // Draw radius handle and slider track
    ctx.strokeStyle = '#3b82f6';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(margin, radiusHandle.y);
    ctx.lineTo(margin + plotWidth, radiusHandle.y);
    ctx.stroke();

    // Draw radius handle
    ctx.fillStyle = '#3b82f6';
    ctx.beginPath();
    ctx.arc(radiusHandle.x, radiusHandle.y, radiusHandle.size, 0, 2 * Math.PI);
    ctx.fill();
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Radius labels
    ctx.fillStyle = '#ffffff';
    ctx.font = '12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('Radius: 0', margin, radiusVizY - 5);
    ctx.textAlign = 'right';
    ctx.fillText('Radius: 200', margin + plotWidth, radiusVizY - 5);
    ctx.textAlign = 'center';
    ctx.fillStyle = '#3b82f6';
    ctx.fillText(
      `Current: ${internalNeighborRadius.toFixed(0)}px`,
      radiusHandle.x,
      radiusVizY + radiusVizHeight + 15
    );
  }

  function isPointInHandle(x: number, y: number, handle: typeof thresholdHandle): boolean {
    const dx = x - handle.x;
    const dy = y - handle.y;
    return Math.sqrt(dx * dx + dy * dy) <= handle.size + 5;
  }

  function handlePointerDown(event: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    if (isPointInHandle(x, y, thresholdHandle)) {
      isDragging = true;
      dragTarget = 'threshold';
      canvas.setPointerCapture(event.pointerId);
    } else if (isPointInHandle(x, y, radiusHandle)) {
      isDragging = true;
      dragTarget = 'radius';
      canvas.setPointerCapture(event.pointerId);
    }
  }

  function handlePointerMove(event: PointerEvent) {
    if (!isDragging || !dragTarget) return;

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;

    if (dragTarget === 'threshold') {
      // Horizontal dragging affects threshold
      const ratio = Math.max(0, Math.min(1, (x - margin) / plotWidth));
      internalAliveThreshold = ratio;
      dispatch('update', { setting: 'aliveThreshold', value: ratio });
    } else if (dragTarget === 'radius') {
      // Horizontal dragging affects radius
      const ratio = Math.max(0, Math.min(1, (x - margin) / plotWidth));
      const maxRadius = 200;
      const newRadius = ratio * maxRadius;
      internalNeighborRadius = newRadius;
      dispatch('update', { setting: 'neighborRadius', value: newRadius });
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
  $: if (aliveThreshold !== internalAliveThreshold && !isDragging) {
    internalAliveThreshold = aliveThreshold;
    draw();
  }

  $: if (neighborRadius !== internalNeighborRadius && !isDragging) {
    internalNeighborRadius = neighborRadius;
    draw();
  }

  // Reset to default values
  function resetToDefaults() {
    const defaults = {
      aliveThreshold: 0.5,
      neighborRadius: 60,
      sampleAliveRatio: 0.6,
    };

    internalAliveThreshold = defaults.aliveThreshold;
    internalNeighborRadius = defaults.neighborRadius;
    sampleAliveRatio = defaults.sampleAliveRatio;

    dispatch('update', { setting: 'aliveThreshold', value: defaults.aliveThreshold });
    dispatch('update', { setting: 'neighborRadius', value: defaults.neighborRadius });
    draw();
  }

  onMount(() => {
    draw();
  });
</script>

<style>
  .instructions {
    margin-bottom: 10px;
  }

  .top-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 14px;
    color: #cccccc;
  }

  .reset-button {
    background: #374151;
    color: #ffffff;
    border: 1px solid #6b7280;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .reset-button:hover {
    background: #4b5563;
  }

  .threshold-diagram {
    border: 1px solid #444444;
    border-radius: 4px;
    cursor: crosshair;
    background: #1a1a1a;
  }

  .parameter-display {
    margin-top: 15px;
    padding: 15px;
    background: #2a2a2a;
    border-radius: 6px;
    border: 1px solid #444444;
  }

  .parameter-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    align-items: center;
  }

  .parameter-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .parameter-label {
    font-weight: 500;
    color: #cccccc;
    font-size: 14px;
  }

  .parameter-value {
    font-family: 'Courier New', monospace;
    font-weight: bold;
    color: #ffffff;
    font-size: 14px;
  }

  .cell-state.alive {
    color: #22c55e;
  }

  .cell-state.dead {
    color: #ef4444;
  }

  .parameter-note {
    grid-column: 1 / -1;
    margin-top: 10px;
    padding: 10px;
    background: #1a1a1a;
    border-radius: 4px;
    font-size: 13px;
    color: #cccccc;
    line-height: 1.4;
  }

  .sample-controls {
    margin-top: 15px;
    padding: 15px;
    background: #2a2a2a;
    border-radius: 6px;
    border: 1px solid #444444;
  }

  .sample-controls label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .sample-label {
    font-weight: 500;
    color: #cccccc;
    font-size: 14px;
  }

  .sample-value {
    font-family: 'Courier New', monospace;
    font-weight: bold;
    color: #ffffff;
    font-size: 14px;
  }

  .sample-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: #444444;
    outline: none;
    -webkit-appearance: none;
    appearance: none;
  }

  .sample-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #3b82f6;
    cursor: pointer;
    border: 2px solid #ffffff;
  }

  .sample-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #3b82f6;
    cursor: pointer;
    border: 2px solid #ffffff;
  }
</style>
