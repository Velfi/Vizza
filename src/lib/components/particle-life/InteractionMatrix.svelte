<!-- Interaction Matrix Component -->
<div class="interaction-matrix" style="--species-count: {settings.species_count}">
  <!-- Header Row -->
  <div class="matrix-header-row">
    <div class="header-corner"></div>
    {#each Array(settings.species_count) as _, j}
      <div class="header-label" style="color: {speciesColors[j] || '#ffffff'}">S{j + 1}</div>
    {/each}
  </div>

  <div class="matrix-grid">
    {#each Array(settings.species_count) as _, i}
      <div class="matrix-row">
        <div class="row-label" style="color: {speciesColors[i] || '#ffffff'}">S{i + 1}</div>
        {#each Array(settings.species_count) as _, j}
          {@const matrixValue =
            settings.force_matrix &&
            settings.force_matrix[i] &&
            settings.force_matrix[i][j] !== undefined
              ? settings.force_matrix[i][j]
              : 0}
          <div
            class="matrix-cell"
            class:repulsion={matrixValue < 0.0}
            class:neutral={matrixValueIsNeutral(matrixValue)}
            class:weak={matrixValueIsWeak(matrixValue)}
            class:moderate={matrixValueIsModerate(matrixValue)}
            class:strong={matrixValueIsStrong(matrixValue)}
          >
            {#if settings.force_matrix && settings.force_matrix[i] && settings.force_matrix[i][j] !== undefined}
              <NumberDragBox
                value={settings.force_matrix[i][j]}
                min={-1}
                max={1}
                step={0.1}
                precision={2}
                showButtons={false}
                on:change={(e) => updateForceMatrix(i, j, e.detail)}
              />
            {:else}
              <div class="matrix-placeholder">0.00</div>
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  </div>

  <div class="matrix-legend">
    <span class="negative">-1.0 = Repulsion</span>
    <span class="neutral">0.0 = Neutral</span>
    <span class="positive">+1.0 = Attraction</span>
  </div>

  <!-- Matrix Transformation Controls -->
  <div class="icon-button-pair">
    <button
      type="button"
      class="icon-btn scale-down"
      on:click={() => scaleMatrix(0.8)}
      title="Scale down matrix values by 20%"
    >
      ⬇↓
    </button>
    <button
      type="button"
      class="icon-btn scale-up"
      on:click={() => scaleMatrix(1.2)}
      title="Scale up matrix values by 20%"
    >
      ↑⬆
    </button>
  </div>

  <!-- Transformation Controls -->
  <div class="icon-transformation-grid">
    <!-- Rotation Pair -->
    <div class="icon-button-pair">
      <button
        type="button"
        class="icon-btn"
        on:click={rotateMatrixCounterclockwise}
        title="Rotate matrix anticlockwise"
      >
        ↺
      </button>
      <button
        type="button"
        class="icon-btn"
        on:click={rotateMatrixClockwise}
        title="Rotate matrix clockwise"
      >
        ↻
      </button>
    </div>

    <!-- Flip Pair -->
    <div class="icon-button-pair">
      <button
        type="button"
        class="icon-btn"
        on:click={flipMatrixHorizontal}
        title="Flip matrix horizontally"
      >
        ↔
      </button>
      <button
        type="button"
        class="icon-btn"
        on:click={flipMatrixVertical}
        title="Flip matrix vertically"
      >
        ↕
      </button>
    </div>

    <!-- Horizontal Shift Pair -->
    <div class="icon-button-pair">
      <button type="button" class="icon-btn" on:click={shiftMatrixLeft} title="Shift matrix left">
        ←
      </button>
      <button type="button" class="icon-btn" on:click={shiftMatrixRight} title="Shift matrix right">
        →
      </button>
    </div>

    <!-- Vertical Shift Pair -->
    <div class="icon-button-pair">
      <button type="button" class="icon-btn" on:click={shiftMatrixUp} title="Shift matrix up">
        ↑
      </button>
      <button type="button" class="icon-btn" on:click={shiftMatrixDown} title="Shift matrix down">
        ↓
      </button>
    </div>

    <!-- Zero and Sign Flip Pair -->
    <div class="icon-button-pair">
      <button
        type="button"
        class="icon-btn"
        on:click={zeroMatrix}
        title="Set all matrix values to zero"
      >
        0
      </button>
      <button
        type="button"
        class="icon-btn"
        on:click={flipMatrixSign}
        title="Flip the sign of all matrix values"
      >
        ±
      </button>
    </div>
  </div>

  <div class="scaling-info">
    <small>Transformations preserve diagonal (self-repulsion) values</small>
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import NumberDragBox from '../inputs/NumberDragBox.svelte';

  const dispatch = createEventDispatcher();

  // Props
  interface ParticleLifeSettings {
    species_count: number;
    force_matrix: number[][];
  }

  export let settings: ParticleLifeSettings;
  export let speciesColors: string[] = [];

  // Matrix value classification functions
  function matrixValueIsNeutral(value: number): boolean {
    return Math.abs(value) < 0.1;
  }

  function matrixValueIsWeak(value: number): boolean {
    return Math.abs(value) >= 0.1 && Math.abs(value) < 0.3;
  }

  function matrixValueIsModerate(value: number): boolean {
    return Math.abs(value) >= 0.3 && Math.abs(value) < 0.7;
  }

  function matrixValueIsStrong(value: number): boolean {
    return Math.abs(value) >= 0.7;
  }

  // Matrix update function
  function updateForceMatrix(i: number, j: number, value: number) {
    if (!settings.force_matrix) return;

    settings.force_matrix[i][j] = value;
    dispatch('matrixUpdate', { i, j, value });
  }

  // Matrix transformation functions
  function scaleMatrix(scaleFactor: number) {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          // Preserve diagonal values (self-repulsion)
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          // Scale non-diagonal values
          newMatrix[i][j] = Math.max(-1, Math.min(1, settings.force_matrix[i][j] * scaleFactor));
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'scale', factor: scaleFactor, matrix: newMatrix });
  }

  function flipMatrixHorizontal() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          newMatrix[i][j] = settings.force_matrix[i][speciesCount - 1 - j];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'flipHorizontal', matrix: newMatrix });
  }

  function flipMatrixVertical() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          newMatrix[i][j] = settings.force_matrix[speciesCount - 1 - i][j];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'flipVertical', matrix: newMatrix });
  }

  function rotateMatrixClockwise() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          newMatrix[i][j] = settings.force_matrix[speciesCount - 1 - j][i];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'rotateClockwise', matrix: newMatrix });
  }

  function rotateMatrixCounterclockwise() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          newMatrix[i][j] = settings.force_matrix[j][speciesCount - 1 - i];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'rotateCounterclockwise', matrix: newMatrix });
  }

  function shiftMatrixLeft() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          const newJ = (j - 1 + speciesCount) % speciesCount;
          newMatrix[i][j] = settings.force_matrix[i][newJ];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'shiftLeft', matrix: newMatrix });
  }

  function shiftMatrixRight() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          const newJ = (j + 1) % speciesCount;
          newMatrix[i][j] = settings.force_matrix[i][newJ];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'shiftRight', matrix: newMatrix });
  }

  function shiftMatrixUp() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          const newI = (i - 1 + speciesCount) % speciesCount;
          newMatrix[i][j] = settings.force_matrix[newI][j];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'shiftUp', matrix: newMatrix });
  }

  function shiftMatrixDown() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          const newI = (i + 1) % speciesCount;
          newMatrix[i][j] = settings.force_matrix[newI][j];
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'shiftDown', matrix: newMatrix });
  }

  function zeroMatrix() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          newMatrix[i][j] = 0;
        }
      }
    }

    settings.force_matrix = newMatrix;
    dispatch('matrixTransform', { type: 'zero', matrix: newMatrix });
  }

  async function flipMatrixSign() {
    if (!settings.force_matrix) return;

    const speciesCount = settings.species_count;
    const newMatrix = Array(speciesCount)
      .fill(null)
      .map(() => Array(speciesCount).fill(0));

    for (let i = 0; i < speciesCount; i++) {
      for (let j = 0; j < speciesCount; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j];
        } else {
          newMatrix[i][j] = -settings.force_matrix[i][j];
        }
      }
    }

    settings.force_matrix = newMatrix;

    try {
      await invoke('update_particle_life_setting', {
        setting: 'force_matrix',
        value: newMatrix,
      });
      dispatch('matrixTransform', { type: 'flipSign' });
    } catch (error) {
      console.error('Failed to flip matrix sign:', error);
    }
  }
</script>

<style>
  .interaction-matrix {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .matrix-header-row {
    display: grid;
    grid-template-columns: repeat(calc(var(--species-count, 3) + 1), 1fr);
    width: fit-content;
    margin: 0 auto;
  }
  .header-corner {
    width: 50px;
    height: 50px;
  }
  .header-label {
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 0.9rem;
    width: 50px;
    height: 50px;
    padding: 0;
    margin: 0;
  }

  .matrix-grid {
    display: grid;
    grid-template-columns: repeat(calc(var(--species-count, 3) + 1), 1fr);
    width: fit-content;
    margin: 0 auto;
    gap: 0;
    align-items: center;
    justify-items: center;
  }

  .row-label {
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 0.9rem;
    width: 50px;
    height: 50px;
    padding: 0;
    margin: 0;
  }

  .matrix-row {
    display: contents;
  }

  .matrix-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: all 0.2s ease;
    width: 50px;
    height: 50px;
    padding: 0;
    margin: 0;
  }

  .matrix-cell :global(.number-drag-container) {
    background: transparent;
    border: none;
    width: 100%;
    height: 100%;
    padding: 0;
    margin: 0;
  }

  .matrix-cell :global(.number-drag-box) {
    background: transparent;
    border: none;
    padding: 0;
    font-size: 0.8rem;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0;
  }

  .matrix-cell.repulsion.weak {
    background: rgb(59, 130, 246);
    border-color: rgb(59, 130, 246);
  }

  .matrix-cell.repulsion.moderate {
    background: rgb(37, 99, 235);
    border-color: rgb(37, 99, 235);
  }

  .matrix-cell.repulsion.strong {
    background: rgb(29, 78, 216);
    border-color: rgb(29, 78, 216);
  }

  .matrix-cell.weak {
    background: rgb(239, 68, 68);
    border-color: rgb(239, 68, 68);
  }

  .matrix-cell.moderate {
    background: rgb(220, 38, 38);
    border-color: rgb(220, 38, 38);
  }

  .matrix-cell.strong {
    background: rgb(185, 28, 28);
    border-color: rgb(185, 28, 28);
  }

  .matrix-cell.neutral {
    background: rgb(138, 138, 138);
    border-color: rgb(138, 138, 138);
  }

  .matrix-placeholder {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    font-family: monospace;
  }

  .matrix-legend {
    display: flex;
    gap: 1rem;
    justify-content: center;
    margin-bottom: 1rem;
    font-size: 0.8rem;
    flex-wrap: wrap;
  }

  .matrix-legend span {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .matrix-legend .negative {
    color: #336aea;
  }

  .matrix-legend .neutral {
    color: #a4a4a4;
  }

  .matrix-legend .positive {
    color: #c42f1c;
  }

  .icon-btn.scale-down {
    background: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.4);
  }

  .icon-btn.scale-down:hover {
    background: rgba(239, 68, 68, 0.4);
    border-color: rgba(239, 68, 68, 0.6);
  }

  .icon-btn.scale-up {
    background: rgba(34, 197, 94, 0.2);
    border-color: rgba(34, 197, 94, 0.4);
  }

  .icon-btn.scale-up:hover {
    background: rgba(34, 197, 94, 0.4);
    border-color: rgba(34, 197, 94, 0.6);
  }

  .icon-transformation-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .icon-button-pair {
    display: flex;
    gap: 2px;
    justify-content: center;
    align-items: center;
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    padding: 0;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.2);
    color: white;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .icon-btn:hover {
    background: rgba(59, 130, 246, 0.4);
    border-color: rgba(59, 130, 246, 0.6);
    transform: scale(1.1);
  }

  .icon-btn:active {
    transform: scale(0.95);
  }

  .scaling-info {
    text-align: center;
  }

  .scaling-info small {
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.75rem;
  }

  /* Responsive design */
  @media (max-width: 1200px) {
    .matrix-cell {
      width: 50px;
      height: 50px;
    }

    .matrix-cell :global(.number-drag-box) {
      font-size: 0.75rem;
      padding: 0.15rem 0.3rem;
    }
  }

  @media (max-width: 900px) {
    .matrix-cell {
      width: 40px;
      height: 40px;
    }

    .matrix-cell :global(.number-drag-box) {
      font-size: 0.7rem;
      padding: 0.1rem 0.2rem;
    }
  }

  @media (max-width: 800px) {
    .icon-transformation-grid {
      grid-template-columns: 1fr;
      gap: 0.5rem;
    }

    .icon-btn {
      width: 28px;
      height: 28px;
      font-size: 14px;
    }
  }

  @media (max-width: 600px) {
    .icon-transformation-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 0.5rem;
    }

    .icon-btn {
      width: 24px;
      height: 24px;
      font-size: 12px;
    }
  }
</style>
