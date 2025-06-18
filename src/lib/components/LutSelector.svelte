<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

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
    { position: 1, color: '#ffffff' }
  ];
  let selectedStopIndex = -1;
  let isDragging = false;
  let dragStopIndex = -1;

  function handleSelect(event: Event) {
    const select = event.target as HTMLSelectElement;
    dispatch('select', { name: select.value });
  }

  async function handleReverse() {
    reversed = !reversed;
    dispatch('reverse', { reversed });
  }

  function cycleLutForward() {
    const currentIndex = available_luts.indexOf(current_lut);
    const nextIndex = (currentIndex + 1) % available_luts.length;
    dispatch('select', { name: available_luts[nextIndex] });
  }

  function cycleLutBack() {
    const currentIndex = available_luts.indexOf(current_lut);
    const prevIndex = (currentIndex - 1 + available_luts.length) % available_luts.length;
    dispatch('select', { name: available_luts[prevIndex] });
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
    isDragging = true;
    dragStopIndex = index;
    selectedStopIndex = index;

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging) return;
      const rect = (e.target as HTMLElement).getBoundingClientRect();
      const position = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      gradientStops[dragStopIndex].position = position;
      gradientStops = [...gradientStops].sort((a, b) => a.position - b.position);
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
      // Convert gradient stops to LUT format and send to backend
      const colors = gradientStops.map(stop => {
        const r = parseInt(stop.color.slice(1, 3), 16) / 255;
        const g = parseInt(stop.color.slice(3, 5), 16) / 255;
        const b = parseInt(stop.color.slice(5, 7), 16) / 255;
        return [r, g, b];
      });
      await invoke('update_gradient_preview', { colors });
    } catch (e) {
      console.error('Failed to update gradient preview:', e);
    }
  }

  async function saveCustomLut() {
    if (!custom_lut_name.trim()) return;
    try {
      await invoke('save_custom_lut', { 
        name: custom_lut_name,
        colors: gradientStops.map(stop => {
          const r = parseInt(stop.color.slice(1, 3), 16) / 255;
          const g = parseInt(stop.color.slice(3, 5), 16) / 255;
          const b = parseInt(stop.color.slice(5, 7), 16) / 255;
          return [r, g, b];
        })
      });
      show_gradient_editor = false;
      custom_lut_name = '';
      // Refresh available LUTs
      available_luts = await invoke('get_available_luts');
    } catch (e) {
      console.error('Failed to save custom LUT:', e);
    }
  }
</script>

<div class="lut-selector">
  <div class="lut-controls">
    <button type="button" class="control-btn" on:click={cycleLutBack}>◀</button>
    <select 
      value={current_lut}
      on:change={handleSelect}
      class="lut-select"
    >
      {#each available_luts as lut}
        <option value={lut}>{lut}</option>
      {/each}
    </select>
    <button type="button" class="control-btn" on:click={cycleLutForward}>▶</button>
    <button type="button" class="control-btn reverse-btn" class:reversed on:click={handleReverse} title="Reverse LUT">
      Reverse
    </button>
    <button 
      type="button"
      class="control-btn gradient-btn"
      on:click={() => show_gradient_editor = true}
      title="Create Custom LUT"
    >
      🎨
    </button>
  </div>
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
          class="text-input"
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
        <button 
          type="button"
          class="secondary-button"
          on:click={async () => {
            try {
              await invoke('apply_lut_by_name', { lutName: current_lut });
            } catch (e) {
              console.error('Failed to restore original LUT:', e);
            }
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