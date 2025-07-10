<!-- Control bar for when UI is visible -->
{#if showUI}
  <div class="controls">
    <div class="left-controls">
      <button class="back-button" on:click={handleBackClick}> ← Back to Menu </button>
      <button class="hide-ui-button" on:click={handleToggleUI}> 👁 Hide UI </button>
    </div>

    <div class="center-controls">
      <button class="pause-resume-button" class:running on:click={handlePauseResume}>
        {running ? '⏸ Pause' : '▶ Resume'}
      </button>
      <span class="status-text">
        {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
      </span>
    </div>

    <div class="right-controls">
      <span class="info-text">
        {simulationName} at {currentFps} FPS
      </span>
    </div>
  </div>
{:else}
  <!-- Auto-hiding controls when UI is hidden -->
  <div
    class="controls auto-hiding"
    class:visible={controlsVisible}
    on:click={handleUserInteraction}
    on:mouseenter={handleUserInteraction}
    on:keydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        handleUserInteraction();
      }
    }}
    role="button"
    tabindex="0"
    aria-label="Auto-hiding control bar - interact to show controls"
  >
    <div class="left-controls">
      <button class="back-button" on:click={handleBackClick}> ← Back to Menu </button>
      <button class="hide-ui-button" on:click={handleToggleUI}> 👁 Show UI </button>
    </div>

    <div class="center-controls">
      <button class="pause-resume-button" class:running on:click={handlePauseResume}>
        {running ? '⏸ Pause' : '▶ Resume'}
      </button>
      <span class="status-text">
        {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
      </span>
    </div>

    <div class="right-controls">
      <span class="info-text">
        {simulationName} at {currentFps} FPS
      </span>
    </div>
  </div>
{/if}

<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  // Props
  export let running: boolean = false;
  export let loading: boolean = false;
  export let showUI: boolean = true;
  export let currentFps: number = 0;
  export let simulationName: string = 'Simulation';
  export let controlsVisible: boolean = true; // For auto-hide functionality

  function handleBackClick() {
    dispatch('back');
  }

  function handleToggleUI() {
    dispatch('toggleUI');
  }

  function handlePauseResume() {
    if (running) {
      dispatch('pause');
    } else {
      dispatch('resume');
    }
  }

  function handleUserInteraction() {
    dispatch('userInteraction');
  }
</script>

<style>
  .controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 80px;
    box-sizing: border-box;
  }

  .left-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .center-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .right-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .back-button,
  .hide-ui-button,
  .pause-resume-button {
    padding: 0.5rem 1rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-family: inherit;
    transition: all 0.3s ease;
  }

  .back-button:hover,
  .hide-ui-button:hover,
  .pause-resume-button:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .pause-resume-button.running {
    background: rgba(81, 207, 102, 0.2);
    border-color: rgba(81, 207, 102, 0.4);
    color: #51cf66;
  }

  .pause-resume-button.running:hover {
    background: rgba(81, 207, 102, 0.3);
    border-color: rgba(81, 207, 102, 0.6);
  }

  .status-text,
  .info-text {
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
  }

  /* Auto-hiding controls styles */
  .controls.auto-hiding {
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s ease;
  }

  .controls.auto-hiding.visible {
    opacity: 1;
    pointer-events: auto;
  }

  /* Responsive design for auto-hiding controls */
  @media (max-width: 600px) {
    .controls {
      height: 70px;
      padding: 0.75rem;
    }

    .left-controls,
    .center-controls,
    .right-controls {
      gap: 0.25rem;
    }

    .back-button,
    .hide-ui-button,
    .pause-resume-button {
      padding: 0.25rem 0.5rem;
      font-size: 0.875rem;
    }

    .status-text,
    .info-text {
      font-size: 0.8rem;
    }
  }
</style>
