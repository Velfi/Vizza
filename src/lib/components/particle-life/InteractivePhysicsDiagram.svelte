<div class="instructions">
  <div class="top-controls">
    <button
      class="toggle-button {useNarrowRange ? 'active' : ''}"
      on:click={() => {
        useNarrowRange = !useNarrowRange;
        draw();
      }}
    >
      {useNarrowRange ? 'Narrow (0.01-0.1)' : 'Wide (0.01-1.0)'}
    </button>
    <span>Drag the handles to adjust physics parameters</span>
    <button class="reset-button" on:click={resetToDefaults}> Reset to Defaults </button>
  </div>
</div>

<canvas
  bind:this={canvas}
  {width}
  {height}
  class="force-diagram"
  on:pointerdown={handlePointerDown}
  on:pointermove={handlePointerMove}
  on:pointerup={handlePointerUp}
  aria-label="Interactive physics diagram showing force vs distance with draggable handles"
>
</canvas>

<div class="parameter-display">
  <div class="parameter-grid">
    <div class="parameter-item">
      <span class="parameter-label">Max Force (F_max):</span>
      <span class="parameter-value">{internalMaxForce.toFixed(2)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Max Distance (r_max):</span>
      <span class="parameter-value">{internalMaxDistance.toFixed(3)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Beta (β):</span>
      <span class="parameter-value">{internalForceBeta.toFixed(2)}</span>
    </div>
    <div class="parameter-item">
      <span class="parameter-label">Transition Point (β×r_max):</span>
      <span class="parameter-value">{(internalForceBeta * internalMaxDistance).toFixed(4)}</span>
    </div>
    <div class="parameter-note">
      <strong>Distance Units:</strong> All distances are in world coordinate units. The simulation world
      spans from 0.0 to 1.0 in each direction, so a distance of 0.03 represents 3% of the world width.
    </div>
  </div>
</div>

<div class="friction-control">
  <label for="friction-slider">
    <span class="friction-label">Friction:</span>
    <span class="friction-value">{friction.toFixed(3)}</span>
  </label>
  <input
    id="friction-slider"
    type="range"
    min="0.01"
    max="1.0"
    step="0.01"
    bind:value={friction}
    on:input={(e) =>
      dispatch('update', {
        setting: 'friction',
        value: parseFloat((e.target as HTMLInputElement).value),
      })}
    class="friction-slider"
  />
</div>

<div class="brownian-motion-control">
  <label for="brownian-motion-slider">
    <span class="brownian-motion-label">Brownian Motion:</span>
    <span class="brownian-motion-value">{brownianMotion.toFixed(3)}</span>
  </label>
  <input
    id="brownian-motion-slider"
    type="range"
    min="0.0"
    max="1.0"
    step="0.01"
    bind:value={brownianMotion}
    on:input={(e) =>
      dispatch('update', {
        setting: 'brownian_motion',
        value: parseFloat((e.target as HTMLInputElement).value),
      })}
    class="brownian-motion-slider"
  />
</div>

<div class="controls-info">
  <div class="controls-header">
    <h4>Interactive Controls:</h4>
  </div>
  <ul>
    <li><strong>🟢 Max Force:</strong> Drag vertically to adjust force strength</li>
    <li><strong>🔵 Max Distance:</strong> Drag horizontally to adjust interaction range</li>
    <li><strong>🟡 Beta:</strong> Drag horizontally to adjust transition point</li>
  </ul>
  <div class="units-note">
    <small
      ><strong>Note:</strong> Use the toggle button to switch between wide (0.01-1.0) and narrow (0.01-0.1)
      distance ranges for better control over small values.</small
    >
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  export let maxForce: number = 1.0;
  export let maxDistance: number = 0.03;
  export let forceBeta: number = 0.3;
  export let friction: number = 0.85;
  export let brownianMotion: number = 0.5;

  const dispatch = createEventDispatcher();

  // Canvas dimensions
  const width = 600;
  const height = 400;
  const margin = 50;
  const plotWidth = width - 2 * margin;
  const plotHeight = height - 2 * margin;

  // Internal state (not reactive to props)
  let internalMaxForce = maxForce;
  let internalMaxDistance = maxDistance;
  let internalForceBeta = forceBeta;
  let betaDistance = internalForceBeta * internalMaxDistance;

  // Range toggle state
  let useNarrowRange = true; // true = narrow range (0.01-0.1), false = wide range (0.01-1.0)

  // Canvas and context
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;

  // Dragging state
  let isDragging = false;
  let dragTarget: 'maxForce' | 'maxDistance' | 'beta' | null = null;
  let dragStartX = 0;
  let originalValues = { maxForce: 0, maxDistance: 0, forceBeta: 0 };

  // Handle positions
  const maxForceHandle = { x: 0, y: 0 };
  const maxDistanceHandle = { x: 0, y: 0 };
  const betaHandle = { x: 0, y: 0 };

  // Scale factors - these will be recalculated in draw()
  // Use a dynamic maximum force for scaling based on the range mode
  function getMaxScaleForce() {
    return useNarrowRange ? 1.0 : 10.0; // Narrow mode: 0-1, Wide mode: 0-10
  }

  let yScale = plotHeight / (2 * getMaxScaleForce());
  const yOffset = margin + plotHeight / 2;

  // Convert coordinates
  function toCanvasY(y: number): number {
    return yOffset - y * yScale;
  }

  // Dynamic plot range based on toggle
  function getMaxPlotDistance() {
    return useNarrowRange ? 0.1 : 1.0;
  }

  // Calculate force at a given distance
  function calculateForce(distance: number): number {
    const minDist = 0.001;
    const betaRmax = internalForceBeta * internalMaxDistance;
    if (distance < betaRmax) {
      // Close range: linear repulsion
      const effectiveDistance = Math.max(distance, minDist);
      return (effectiveDistance / betaRmax - 1.0) * internalMaxForce;
    } else if (distance <= internalMaxDistance) {
      // Far range: species-specific attraction/repulsion
      const farRangeForce = internalMaxForce * 0.5;
      return (
        farRangeForce *
        (1.0 -
          (1.0 + internalForceBeta - (2.0 * distance) / internalMaxDistance) /
            (1.0 - internalForceBeta))
      );
    }
    return 0;
  }

  // Update handle positions
  function updateHandlePositions() {
    // Max Distance handle position - use linear scaling for X position
    const distanceRatio = internalMaxDistance / getMaxPlotDistance();
    maxDistanceHandle.x = margin + distanceRatio * plotWidth;
    maxDistanceHandle.y = toCanvasY(0);

    // Max Force handle is positioned at the left edge and moves freely on Y axis
    maxForceHandle.x = margin; // Position at left margin
    maxForceHandle.y = toCanvasY(internalMaxForce);

    // Beta handle is positioned at the beta distance - use linear scaling
    const betaRatio = Math.min(
      betaDistance / getMaxPlotDistance(),
      internalMaxDistance / getMaxPlotDistance()
    );
    betaHandle.x = margin + betaRatio * plotWidth;
    betaHandle.y = toCanvasY(0);
  }

  // Draw the diagram
  function draw() {
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Update scales
    yScale = plotHeight / (2 * getMaxScaleForce());
    betaDistance = internalForceBeta * internalMaxDistance;

    // Update handle positions
    updateHandlePositions();

    // Draw background - dark theme
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, width, height);

    // Draw inactive area (right of max distance handle) as darker gray
    ctx.fillStyle = '#0f0f0f';
    ctx.fillRect(maxDistanceHandle.x, margin, margin + plotWidth - maxDistanceHandle.x, plotHeight);

    // Draw grid lines - dark theme
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 1;

    // Vertical grid lines (only in active area) - use logarithmic spacing
    ctx.beginPath();
    ctx.moveTo(margin, margin);
    ctx.lineTo(margin, height - margin);
    ctx.stroke();

    // Draw dynamic grid lines for distance reference
    const plotRange = getMaxPlotDistance();
    const gridCount = 10; // Number of grid lines
    ctx.setLineDash([2, 2]);
    ctx.strokeStyle = '#444444';
    for (let i = 1; i <= gridCount; i++) {
      const gridValue = (i / gridCount) * plotRange;
      if (gridValue <= internalMaxDistance) {
        const ratio = gridValue / plotRange;
        const x = margin + ratio * plotWidth;
        ctx.beginPath();
        ctx.moveTo(x, margin);
        ctx.lineTo(x, height - margin);
        ctx.stroke();
      }
    }
    ctx.setLineDash([]);
    ctx.strokeStyle = '#333333';

    // Beta line (dashed, only if within active area)
    if (betaDistance <= internalMaxDistance) {
      ctx.setLineDash([5, 5]);
      ctx.beginPath();
      ctx.moveTo(betaHandle.x, margin);
      ctx.lineTo(betaHandle.x, height - margin);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Max distance line (at blue handle position)
    ctx.beginPath();
    ctx.moveTo(maxDistanceHandle.x, margin);
    ctx.lineTo(maxDistanceHandle.x, height - margin);
    ctx.stroke();

    // Max force line (at green handle position) - vertical slider track
    ctx.strokeStyle = '#51cf66';
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.moveTo(maxForceHandle.x, margin);
    ctx.lineTo(maxForceHandle.x, height - margin);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.strokeStyle = '#333333';

    // Horizontal grid lines (span full width)
    ctx.beginPath();
    ctx.moveTo(margin, yOffset);
    ctx.lineTo(width - margin, yOffset);
    ctx.stroke();

    // Draw grid line at current max force level
    ctx.beginPath();
    ctx.moveTo(margin, toCanvasY(internalMaxForce));
    ctx.lineTo(width - margin, toCanvasY(internalMaxForce));
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(margin, toCanvasY(-internalMaxForce));
    ctx.lineTo(width - margin, toCanvasY(-internalMaxForce));
    ctx.stroke();

    // Draw additional grid lines for scale reference
    ctx.setLineDash([2, 2]);
    ctx.strokeStyle = '#444444';
    ctx.beginPath();
    ctx.moveTo(margin, toCanvasY(getMaxScaleForce()));
    ctx.lineTo(width - margin, toCanvasY(getMaxScaleForce()));
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(margin, toCanvasY(-getMaxScaleForce()));
    ctx.lineTo(width - margin, toCanvasY(-getMaxScaleForce()));
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.strokeStyle = '#333333';

    // Draw zones (only in active area) - dark theme colors
    // Close range zone
    if (betaDistance <= internalMaxDistance) {
      ctx.fillStyle = 'rgba(239, 68, 68, 0.2)';
      ctx.strokeStyle = 'rgba(239, 68, 68, 0.4)';
      ctx.lineWidth = 2;
      ctx.fillRect(margin, margin, betaHandle.x - margin, plotHeight);
      ctx.strokeRect(margin, margin, betaHandle.x - margin, plotHeight);

      // Far range zone
      ctx.fillStyle = 'rgba(59, 130, 246, 0.2)';
      ctx.strokeStyle = 'rgba(59, 130, 246, 0.4)';
      ctx.fillRect(betaHandle.x, margin, maxDistanceHandle.x - betaHandle.x, plotHeight);
      ctx.strokeRect(betaHandle.x, margin, maxDistanceHandle.x - betaHandle.x, plotHeight);
    } else {
      // If beta distance is beyond max distance, show only one zone
      ctx.fillStyle = 'rgba(239, 68, 68, 0.2)';
      ctx.strokeStyle = 'rgba(239, 68, 68, 0.4)';
      ctx.lineWidth = 2;
      ctx.fillRect(margin, margin, maxDistanceHandle.x - margin, plotHeight);
      ctx.strokeRect(margin, margin, maxDistanceHandle.x - margin, plotHeight);
    }

    // Draw force curve (only in active area) - bright red for visibility on dark background
    ctx.strokeStyle = '#ef4444';
    ctx.lineWidth = 3;
    ctx.beginPath();

    const steps = 200;
    let firstPoint = true;

    for (let i = 0; i <= steps; i++) {
      // Create linear progression from 0 to max distance
      const distance = (i / steps) * internalMaxDistance;
      const ratio = distance / getMaxPlotDistance();

      // Calculate force and position
      const force = calculateForce(distance);
      const x = margin + ratio * plotWidth;
      const y = toCanvasY(force);

      if (firstPoint) {
        ctx.moveTo(x, y);
        firstPoint = false;
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();

    // Draw handles - updated colors for dark theme
    const handleRadius = 8;

    // Max Force handle (green)
    ctx.fillStyle = '#51cf66';
    ctx.strokeStyle = '#2f9e44';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(maxForceHandle.x, maxForceHandle.y, handleRadius, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();

    // Max Distance handle (blue)
    ctx.fillStyle = '#3b82f6';
    ctx.strokeStyle = '#1d4ed8';
    ctx.beginPath();
    ctx.arc(maxDistanceHandle.x, maxDistanceHandle.y, handleRadius, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();

    // Beta handle (yellow)
    ctx.fillStyle = '#fbbf24';
    ctx.strokeStyle = '#d97706';
    ctx.beginPath();
    ctx.arc(betaHandle.x, betaHandle.y, handleRadius, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();

    // Draw labels - white text for dark theme
    ctx.fillStyle = '#ffffff';
    ctx.font = '12px sans-serif';
    ctx.textAlign = 'left';
    ctx.fillText('Max Force', maxForceHandle.x + 15, maxForceHandle.y - 5);

    ctx.textAlign = 'center';
    ctx.fillText('Max Distance', maxDistanceHandle.x, maxDistanceHandle.y + 25);
    ctx.fillText('β (Transition)', betaHandle.x, betaHandle.y - 15);

    // Draw zone labels (only if zones are visible in active area)
    if (betaDistance <= internalMaxDistance) {
      ctx.font = '14px sans-serif';
      ctx.fillStyle = '#ffffff';
      ctx.fillText('Close Range', margin + (betaHandle.x - margin) / 2, margin + 25);
      ctx.font = '11px sans-serif';
      ctx.fillStyle = '#cccccc';
      ctx.fillText('Repulsion Zone', margin + (betaHandle.x - margin) / 2, margin + 42);

      ctx.font = '14px sans-serif';
      ctx.fillStyle = '#ffffff';
      ctx.fillText(
        'Far Range',
        betaHandle.x + (maxDistanceHandle.x - betaHandle.x) / 2,
        margin + 25
      );
      ctx.font = '11px sans-serif';
      ctx.fillStyle = '#cccccc';
      ctx.fillText(
        'Attraction/Repulsion Zone',
        betaHandle.x + (maxDistanceHandle.x - betaHandle.x) / 2,
        margin + 42
      );
    } else {
      // Show single zone label if beta is beyond max distance
      ctx.font = '14px sans-serif';
      ctx.fillStyle = '#ffffff';
      ctx.fillText('Close Range', margin + (maxDistanceHandle.x - margin) / 2, margin + 25);
      ctx.font = '11px sans-serif';
      ctx.fillStyle = '#cccccc';
      ctx.fillText('Repulsion Zone', margin + (maxDistanceHandle.x - margin) / 2, margin + 42);
    }

    // Draw axis labels
    ctx.font = '14px sans-serif';
    ctx.fillStyle = '#ffffff';
    ctx.textAlign = 'center';
    ctx.fillText('Distance (r)', width / 2, height - 15);

    ctx.save();
    ctx.translate(15, height / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Force (F)', 0, 0);
    ctx.restore();

    // Draw value labels
    ctx.font = '11px sans-serif';
    ctx.fillStyle = '#cccccc';
    ctx.textAlign = 'center';
    const currentPlotRange = getMaxPlotDistance();
    ctx.fillText('0.0', margin, height - 25);
    ctx.fillText(currentPlotRange.toFixed(1), margin + plotWidth, height - 25);

    // Only show beta label if it's within the active area
    if (betaDistance <= internalMaxDistance) {
      ctx.fillText('β×r_max', betaHandle.x, height - 25);
    }

    ctx.fillText('r_max', maxDistanceHandle.x, height - 25);

    ctx.textAlign = 'right';
    ctx.fillText('0', margin - 8, yOffset + 4);
    ctx.fillText('+F_max', margin - 8, toCanvasY(internalMaxForce) + 4);
    ctx.fillText('-F_max', margin - 8, toCanvasY(-internalMaxForce) + 4);

    // Show scale reference labels
    ctx.fillStyle = '#999999';
    ctx.font = '9px sans-serif';
    ctx.fillText(`+${getMaxScaleForce()}`, margin - 8, toCanvasY(getMaxScaleForce()) + 4);
    ctx.fillText(`-${getMaxScaleForce()}`, margin - 8, toCanvasY(-getMaxScaleForce()) + 4);
  }

  // Check if point is near handle
  function isNearHandle(x: number, y: number, handle: { x: number; y: number }): boolean {
    const distance = Math.sqrt((x - handle.x) ** 2 + (y - handle.y) ** 2);
    return distance <= 12; // Slightly larger than handle radius for easier clicking
  }

  // Handle mouse events
  function handlePointerDown(event: PointerEvent) {
    event.preventDefault();

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    if (isNearHandle(x, y, maxForceHandle)) {
      isDragging = true;
      dragTarget = 'maxForce';
    } else if (isNearHandle(x, y, maxDistanceHandle)) {
      isDragging = true;
      dragTarget = 'maxDistance';
    } else if (isNearHandle(x, y, betaHandle)) {
      isDragging = true;
      dragTarget = 'beta';
    }

    if (isDragging) {
      dragStartX = x;
      originalValues = {
        maxForce: internalMaxForce,
        maxDistance: internalMaxDistance,
        forceBeta: internalForceBeta,
      };
      canvas.setPointerCapture(event.pointerId);
    }
  }

  function handlePointerMove(event: PointerEvent) {
    if (!isDragging || !dragTarget) return;

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    switch (dragTarget) {
      case 'maxForce':
        // Vertical dragging affects max force - handle moves like a slider on Y axis
        // Convert mouse Y position directly to force value
        const forceFromY = (yOffset - y) / yScale;
        const newMaxForce = Math.max(0.1, Math.min(getMaxScaleForce(), forceFromY));
        internalMaxForce = newMaxForce;
        dispatch('update', { setting: 'max_force', value: newMaxForce });
        break;

      case 'maxDistance':
        // Horizontal dragging affects max distance - use linear scaling
        // Convert mouse X position to linear distance value
        const ratio = Math.max(0, Math.min(1, (x - margin) / plotWidth));
        const newMaxDistance = ratio * getMaxPlotDistance();
        internalMaxDistance = newMaxDistance;
        dispatch('update', { setting: 'max_distance', value: newMaxDistance });
        break;

      case 'beta':
        // Horizontal dragging affects beta (transition point) - uses delta-based calculation
        const deltaX = x - dragStartX;
        const newBeta = Math.max(0.1, Math.min(0.9, originalValues.forceBeta + deltaX * 0.002));
        internalForceBeta = newBeta;
        dispatch('update', { setting: 'force_beta', value: newBeta });
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
  $: if (maxForce !== internalMaxForce && !isDragging) {
    internalMaxForce = maxForce;
    draw();
  }

  $: if (maxDistance !== internalMaxDistance && !isDragging) {
    internalMaxDistance = maxDistance;
    draw();
  }

  $: if (forceBeta !== internalForceBeta && !isDragging) {
    internalForceBeta = forceBeta;
    draw();
  }

  // Reset to default values
  function resetToDefaults() {
    const defaults = {
      max_force: 0.5,
      max_distance: 0.01,
      force_beta: 0.3,
      friction: 0.5,
      brownian_motion: 0.5,
    };

    // Update internal values
    internalMaxForce = defaults.max_force;
    internalMaxDistance = defaults.max_distance;
    internalForceBeta = defaults.force_beta;
    friction = defaults.friction;
    brownianMotion = defaults.brownian_motion;

    // Dispatch updates to parent
    dispatch('update', { setting: 'max_force', value: defaults.max_force });
    dispatch('update', { setting: 'max_distance', value: defaults.max_distance });
    dispatch('update', { setting: 'force_beta', value: defaults.force_beta });
    dispatch('update', { setting: 'friction', value: defaults.friction });
    dispatch('update', { setting: 'brownian_motion', value: defaults.brownian_motion });

    // Redraw the diagram
    draw();
  }

  onMount(() => {
    ctx = canvas.getContext('2d')!;
    draw();
  });
</script>

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

  .top-controls span {
    flex: 1;
    text-align: center;
  }

  .force-diagram {
    display: block;
    margin: 0 auto;
    cursor: default;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    width: 600px;
    height: 400px;
  }

  .force-diagram:hover {
    cursor: grab;
  }

  .force-diagram:active {
    cursor: grabbing;
  }

  .parameter-display {
    margin: 15px 0;
    padding: 12px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }

  .parameter-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }

  .parameter-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 4px;
  }

  .parameter-label {
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9em;
    font-weight: 500;
  }

  .parameter-value {
    color: #3b82f6;
    font-family: monospace;
    font-weight: 600;
    font-size: 0.95em;
  }

  .parameter-note {
    grid-column: span 2;
    padding: 12px;
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.3);
    border-radius: 4px;
    border-left: 3px solid #3b82f6;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.85em;
    line-height: 1.4;
  }

  .parameter-note strong {
    color: #93c5fd;
  }

  .friction-control {
    margin: 15px 0;
    padding: 12px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }

  .friction-control label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    font-size: 0.9em;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
  }

  .friction-label {
    color: rgba(255, 255, 255, 0.8);
  }

  .friction-value {
    color: #3b82f6;
    font-family: monospace;
    font-weight: 600;
  }

  .friction-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.1);
    outline: none;
    -webkit-appearance: none;
    appearance: none;
  }

  .friction-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #3b82f6;
    cursor: pointer;
    border: 2px solid #1a1a1a;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  }

  .friction-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #3b82f6;
    cursor: pointer;
    border: 2px solid #1a1a1a;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  }

  .friction-slider:hover::-webkit-slider-thumb {
    background: #1d4ed8;
  }

  .friction-slider:hover::-moz-range-thumb {
    background: #1d4ed8;
  }

  .controls-info {
    margin-top: 15px;
    padding: 15px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    border-left: 4px solid #51cf66;
  }

  .controls-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }

  .controls-info h4 {
    margin: 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1em;
  }

  .toggle-button {
    background: #6c757d;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 6px 12px;
    font-size: 0.85em;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .toggle-button:hover {
    background: #5a6268;
  }

  .toggle-button.active {
    background: #fbbf24;
    color: #1a1a1a;
  }

  .toggle-button.active:hover {
    background: #d97706;
  }

  .reset-button {
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 6px 12px;
    font-size: 0.85em;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .reset-button:hover {
    background: #1d4ed8;
  }

  .reset-button:active {
    transform: translateY(1px);
  }

  .controls-info ul {
    margin: 0;
    padding-left: 20px;
  }

  .controls-info li {
    margin-bottom: 5px;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9em;
    line-height: 1.4;
  }

  .controls-info strong {
    color: rgba(255, 255, 255, 0.9);
  }

  .units-note {
    margin-top: 10px;
    padding: 8px 12px;
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.3);
    border-radius: 4px;
    border-left: 3px solid #3b82f6;
  }

  .units-note small {
    color: #93c5fd;
    font-size: 0.85em;
    line-height: 1.3;
  }

  .brownian-motion-control {
    margin: 15px 0;
    padding: 12px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }

  .brownian-motion-control label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    font-size: 0.9em;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
  }

  .brownian-motion-label {
    color: rgba(255, 255, 255, 0.8);
  }

  .brownian-motion-value {
    color: #fbbf24;
    font-family: monospace;
    font-weight: 600;
  }

  .brownian-motion-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.1);
    outline: none;
    -webkit-appearance: none;
    appearance: none;
  }

  .brownian-motion-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fbbf24;
    cursor: pointer;
    border: 2px solid #1a1a1a;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  }

  .brownian-motion-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fbbf24;
    cursor: pointer;
    border: 2px solid #1a1a1a;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  }

  .brownian-motion-slider:hover::-webkit-slider-thumb {
    background: #d97706;
  }

  .brownian-motion-slider:hover::-moz-range-thumb {
    background: #d97706;
  }
</style>
