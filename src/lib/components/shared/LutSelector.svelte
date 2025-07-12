<div class="lut-selector">
  <div class="lut-controls">
    <Selector options={available_luts} bind:value={current_lut} on:change={handleSelect} />
    <button
      type="button"
      class="control-btn reverse-btn"
      class:reversed
      on:click={handleReverse}
      title="Reverse LUT"
    >
      Reverse
    </button>
    <button
      type="button"
      class="control-btn gradient-btn"
      on:click={openGradientEditor}
      title="Create Custom LUT"
    >
      🎨
    </button>
  </div>
</div>

{#if show_gradient_editor}
  <div
    class="gradient-editor-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="gradient-editor-title"
    tabindex="-1"
    on:click|self={closeGradientEditor}
    on:keydown={(e) => e.key === 'Escape' && closeGradientEditor()}
  >
    <div class="dialog-content gradient-editor-content" role="document" on:click|stopPropagation>
      <h3 id="gradient-editor-title">Custom LUT Editor</h3>

      <!-- LUT Name Input -->
      <div class="control-group">
        <label for="customLutName">LUT Name</label>
        <input
          type="text"
          id="customLutName"
          bind:value={custom_lut_name}
          placeholder="Enter LUT name..."
          class="text-input"
        />
      </div>

      <!-- Gradient Preview -->
      <div class="gradient-preview-container">
        <div
          class="gradient-preview"
          style="background: linear-gradient(to right, {gradientStops
            .map((stop) => `${stop.color} ${stop.position * 100}%`)
            .join(', ')})"
        ></div>
        <div
          class="gradient-stops-container"
          role="button"
          tabindex="0"
          aria-label="Gradient editor area - click to add color stops"
          on:click={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const position = (e.clientX - rect.left) / rect.width;
            addGradientStop(position);
          }}
          on:keydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              // Add a stop at the center if activated with keyboard
              addGradientStop(0.5);
            }
          }}
        >
          {#each gradientStops as stop, index}
            <div
              class="gradient-stop"
              class:selected={index === selectedStopIndex}
              class:dragging={isDragging && dragStopIndex === index}
              style="left: {stop.position * 100}%; background-color: {stop.color}"
              role="button"
              tabindex="0"
              aria-label="Color stop {index + 1} at {Math.round(
                stop.position * 100
              )}% - click to select"
              on:mousedown={(e) => handleStopMouseDown(e, index)}
              on:click|stopPropagation={() => (selectedStopIndex = index)}
              on:keydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  selectedStopIndex = index;
                }
              }}
            >
              {#if gradientStops.length > 2}
                <button
                  class="remove-stop"
                  on:click|stopPropagation={() => removeGradientStop(index)}>×</button
                >
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
                class="color-input"
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
        <button type="button" class="secondary-button" on:click={closeGradientEditor}>
          Cancel
        </button>
      </div>
    </div>
  </div>
{/if}

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Selector from '../inputs/Selector.svelte';

  export let available_luts: string[] = [];
  export let current_lut: string = '';
  export let reversed: boolean = false;

  const dispatch = createEventDispatcher<{
    select: { name: string };
    reverse: { reversed: boolean };
  }>();

  // Gradient editor state
  let show_gradient_editor = false;
  let custom_lut_name = '';
  let gradientStops = [
    { position: 0, color: '#000000' },
    { position: 1, color: '#ffffff' },
  ];
  let selectedStopIndex = -1;
  let isDragging = false;
  let dragStopIndex = -1;
  let original_lut_name = ''; // Store the original LUT name to restore on cancel

  // Reactive statements to handle prop changes
  // Note: Don't auto-select the first LUT when current_lut is empty,
  // let the parent component set the initial LUT from backend state

  function handleSelect({ detail }: { detail: { value: string } }) {
    const selectedName = detail.value;
    console.log(`LutSelector: Selected ${selectedName}, was ${current_lut}`);
    current_lut = selectedName; // Update local state
    dispatch('select', { name: selectedName });
  }

  async function handleReverse() {
    reversed = !reversed;
    console.log(`LutSelector: Reversing to ${reversed}, current LUT: ${current_lut}`);
    dispatch('reverse', { reversed });
  }

  // Function to open gradient editor and apply initial gradient
  async function openGradientEditor() {
    original_lut_name = current_lut; // Store the original LUT name
    show_gradient_editor = true;

    // Apply the initial gradient preview immediately
    await updateGradientPreview();
  }

  // Function to close gradient editor and restore original LUT
  async function closeGradientEditor() {
    show_gradient_editor = false;
    custom_lut_name = '';

    // Restore the original LUT
    try {
      await invoke('apply_lut_by_name', { lutName: original_lut_name });
    } catch (e) {
      console.error('Failed to restore original LUT:', e);
    }
  }

  // Gradient editor functions
  function addGradientStop(position: number) {
    // Find the color at this position
    const color = getColorAtPosition(position);
    gradientStops = [...gradientStops, { position, color }];
    gradientStops.sort((a, b) => a.position - b.position);
    updateGradientPreview();
  }

  function removeGradientStop(index: number) {
    if (gradientStops.length <= 2) return;
    gradientStops = gradientStops.filter((_, i) => i !== index);
    if (selectedStopIndex === index) {
      selectedStopIndex = -1;
    }
    updateGradientPreview();
  }

  function updateStopColor(index: number, color: string) {
    gradientStops[index].color = color;
    updateGradientPreview();
  }

  function getColorAtPosition(position: number): string {
    // Find the two stops that bound this position
    let leftStop = gradientStops[0];
    let rightStop = gradientStops[gradientStops.length - 1];

    for (let i = 0; i < gradientStops.length - 1; i++) {
      if (gradientStops[i].position <= position && gradientStops[i + 1].position >= position) {
        leftStop = gradientStops[i];
        rightStop = gradientStops[i + 1];
        break;
      }
    }

    // Interpolate between the two colors
    const t = (position - leftStop.position) / (rightStop.position - leftStop.position);
    return interpolateColor(leftStop.color, rightStop.color, t);
  }

  function interpolateColor(color1: string, color2: string, t: number): string {
    const r1 = parseInt(color1.slice(1, 3), 16);
    const g1 = parseInt(color1.slice(3, 5), 16);
    const b1 = parseInt(color1.slice(5, 7), 16);
    const r2 = parseInt(color2.slice(1, 3), 16);
    const g2 = parseInt(color2.slice(3, 5), 16);
    const b2 = parseInt(color2.slice(5, 7), 16);

    const r = Math.round(r1 + (r2 - r1) * t);
    const g = Math.round(g1 + (g2 - g1) * t);
    const b = Math.round(b1 + (b2 - b1) * t);

    return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
  }

  function handleStopMouseDown(event: MouseEvent, index: number) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = true;
    dragStopIndex = index;
    selectedStopIndex = index;

    // Store the container element reference for consistent position calculation
    const container = (event.currentTarget as HTMLElement).parentElement as HTMLElement;

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging) return;

      // Use the stored container reference and recalculate rect if needed
      const rect = container.getBoundingClientRect();
      const position = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));

      // Update the stop position
      gradientStops[dragStopIndex].position = position;

      // Re-sort stops by position and update the array to trigger reactivity
      gradientStops = [...gradientStops].sort((a, b) => a.position - b.position);

      // Update the drag index to match the new position
      dragStopIndex = gradientStops.findIndex((stop) => Math.abs(stop.position - position) < 0.001);

      updateGradientPreview();
    };

    const handleMouseUp = () => {
      isDragging = false;
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }

  async function updateGradientPreview() {
    try {
      // Build LUT in [r0..r255, g0..g255, b0..b255] format for preview, as integers 0-255
      const rArr: number[] = [];
      const gArr: number[] = [];
      const bArr: number[] = [];
      for (let i = 0; i < 256; i++) {
        const t = i / 255;
        let leftStop = gradientStops[0];
        let rightStop = gradientStops[gradientStops.length - 1];

        // Find the correct stops to interpolate between
        if (t <= gradientStops[0].position) {
          // Before first stop - use first stop color
          leftStop = gradientStops[0];
          rightStop = gradientStops[0];
        } else if (t >= gradientStops[gradientStops.length - 1].position) {
          // After last stop - use last stop color
          leftStop = gradientStops[gradientStops.length - 1];
          rightStop = gradientStops[gradientStops.length - 1];
        } else {
          // Between stops - find the correct pair
          for (let j = 0; j < gradientStops.length - 1; j++) {
            if (gradientStops[j].position <= t && gradientStops[j + 1].position >= t) {
              leftStop = gradientStops[j];
              rightStop = gradientStops[j + 1];
              break;
            }
          }
        }

        // Calculate interpolation factor, handling edge cases
        const positionDiff = rightStop.position - leftStop.position;
        const interp_t = positionDiff === 0 ? 0 : (t - leftStop.position) / positionDiff;
        const interpolatedColor = interpolateColor(leftStop.color, rightStop.color, interp_t);

        const r = parseInt(interpolatedColor.slice(1, 3), 16);
        const g = parseInt(interpolatedColor.slice(3, 5), 16);
        const b = parseInt(interpolatedColor.slice(5, 7), 16);
        rArr.push(r);
        gArr.push(g);
        bArr.push(b);
      }
      const lutData = [...rArr, ...gArr, ...bArr];

      await invoke('update_gradient_preview', { lutData });
    } catch (e) {
      console.error('Failed to update gradient preview:', e);
    }
  }

  async function saveCustomLut() {
    if (!custom_lut_name.trim()) return;
    try {
      // Build LUT in [r0..r255, g0..g255, b0..b255] format as integers
      const rArr: number[] = [];
      const gArr: number[] = [];
      const bArr: number[] = [];
      for (let i = 0; i < 256; i++) {
        const t = i / 255;
        let leftStop = gradientStops[0];
        let rightStop = gradientStops[gradientStops.length - 1];

        // Find the correct stops to interpolate between
        if (t <= gradientStops[0].position) {
          // Before first stop - use first stop color
          leftStop = gradientStops[0];
          rightStop = gradientStops[0];
        } else if (t >= gradientStops[gradientStops.length - 1].position) {
          // After last stop - use last stop color
          leftStop = gradientStops[gradientStops.length - 1];
          rightStop = gradientStops[gradientStops.length - 1];
        } else {
          // Between stops - find the correct pair
          for (let j = 0; j < gradientStops.length - 1; j++) {
            if (gradientStops[j].position <= t && gradientStops[j + 1].position >= t) {
              leftStop = gradientStops[j];
              rightStop = gradientStops[j + 1];
              break;
            }
          }
        }

        // Calculate interpolation factor, handling edge cases
        const positionDiff = rightStop.position - leftStop.position;
        const interp_t = positionDiff === 0 ? 0 : (t - leftStop.position) / positionDiff;
        const interpolatedColor = interpolateColor(leftStop.color, rightStop.color, interp_t);
        const r = parseInt(interpolatedColor.slice(1, 3), 16);
        const g = parseInt(interpolatedColor.slice(3, 5), 16);
        const b = parseInt(interpolatedColor.slice(5, 7), 16);
        rArr.push(r);
        gArr.push(g);
        bArr.push(b);
      }
      const lutData = [...rArr, ...gArr, ...bArr];
      await invoke('save_custom_lut', {
        name: custom_lut_name,
        lut_data: lutData,
      });
      // Close the editor without restoring the original LUT
      show_gradient_editor = false;
      custom_lut_name = '';
      // Refresh available LUTs
      available_luts = await invoke('get_available_luts');
    } catch (e) {
      console.error('Failed to save custom LUT:', e);
    }
  }
</script>

<style>
  .lut-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .lut-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .control-btn {
    padding: 0.5rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s ease;
  }

  .control-btn:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.4);
    color: rgba(255, 255, 255, 1);
  }

  .control-btn.reverse-btn {
    padding: 0.5rem 1rem;
    font-size: 0.8rem;
  }

  .control-btn.reverse-btn.reversed {
    background: #646cff;
    color: white;
    border-color: #646cff;
  }

  .control-btn.gradient-btn {
    font-size: 1.2rem;
    padding: 0.5rem 0.75rem;
  }

  /* Gradient Editor Dialog Styles */
  .gradient-editor-dialog {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .gradient-editor-content {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    min-width: 500px;
    max-width: 600px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
  }

  .gradient-editor-content h3 {
    margin: 0 0 1.5rem 0;
    color: #333;
    font-size: 1.5rem;
  }

  .text-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 1rem;
  }

  .text-input:focus {
    outline: none;
    border-color: #646cff;
    box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
  }

  .gradient-preview-container {
    margin: 1.5rem 0;
  }

  .gradient-preview {
    height: 50px;
    border: 2px solid #ccc;
    border-radius: 6px;
    margin-bottom: 15px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .gradient-stops-container {
    position: relative;
    height: 40px;
    background: #f8f9fa;
    border: 2px solid #dee2e6;
    border-radius: 6px;
    cursor: crosshair;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.1);
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
  }

  .gradient-stops-container:hover {
    border-color: #646cff;
  }

  .gradient-stop {
    position: absolute;
    top: 50%;
    transform: translateX(-50%) translateY(-50%);
    width: 24px;
    height: 24px;
    border: 3px solid white;
    border-radius: 50%;
    cursor: grab;
    box-shadow: 0 3px 8px rgba(0, 0, 0, 0.3);
    transition: all 0.2s ease;
    user-select: none;
  }

  .gradient-stop:hover {
    transform: translateX(-50%) translateY(-50%) scale(1.1);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .gradient-stop.selected {
    border-color: #646cff;
    border-width: 4px;
    box-shadow: 0 4px 16px rgba(100, 108, 255, 0.5);
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
    width: 18px;
    height: 18px;
    background: #dc3545;
    color: white;
    border: none;
    border-radius: 50%;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    transition: all 0.2s ease;
  }

  .remove-stop:hover {
    background: #c82333;
    transform: scale(1.1);
  }

  .stop-controls {
    background: #f8f9fa;
    padding: 1.5rem;
    border-radius: 6px;
    margin: 1.5rem 0;
    border: 1px solid #dee2e6;
  }

  .stop-controls h4 {
    margin: 0 0 1rem 0;
    color: #333;
    font-size: 1.1rem;
  }

  .control-row {
    display: flex;
    gap: 1rem;
    align-items: end;
  }

  .control-row .control-group {
    flex: 1;
  }

  .control-group {
    margin-bottom: 1rem;
  }

  .control-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: #495057;
  }

  .color-input {
    width: 100%;
    height: 40px;
    border: 1px solid #ccc;
    border-radius: 4px;
    cursor: pointer;
  }

  .color-input:focus {
    outline: none;
    border-color: #646cff;
    box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
  }

  .gradient-instructions {
    background: #e3f2fd;
    padding: 1rem;
    border-radius: 6px;
    margin: 1.5rem 0;
    border-left: 4px solid #2196f3;
  }

  .gradient-instructions p {
    margin: 0 0 0.75rem 0;
    color: #1976d2;
    font-weight: 600;
  }

  .gradient-instructions ul {
    margin: 0;
    padding-left: 1.5rem;
  }

  .gradient-instructions li {
    margin: 0.3rem 0;
    color: #333;
    line-height: 1.4;
  }

  .dialog-actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
    margin-top: 2rem;
    padding-top: 1.5rem;
    border-top: 1px solid #dee2e6;
  }

  .primary-button {
    background: #646cff;
    color: white;
    border: 1px solid #646cff;
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .primary-button:hover:not(:disabled) {
    background: #535bf2;
    border-color: #535bf2;
    transform: translateY(-1px);
    box-shadow: 0 4px 8px rgba(100, 108, 255, 0.3);
  }

  .primary-button:disabled {
    background: #adb5bd;
    border-color: #adb5bd;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .secondary-button {
    background: #6c757d;
    color: white;
    border: 1px solid #6c757d;
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .secondary-button:hover {
    background: #5a6268;
    border-color: #5a6268;
    transform: translateY(-1px);
    box-shadow: 0 4px 8px rgba(108, 117, 125, 0.3);
  }
</style>
