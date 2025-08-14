<!-- Control bar for when UI is visible -->
{#if showUI}
  <div class="controls">
    <div class="left-controls">
      <Button variant="default" on:click={handleBackClick}>← Back to Menu</Button>
      <Button variant="default" on:click={handleToggleUI}>
        👁 {showUI ? 'Hide UI' : 'Show UI'}
      </Button>
      {#if controlModeButton}
        {@render controlModeButton()}
      {/if}
    </div>

    {#if showCenterControls}
      <div class="center-controls">
        <Button variant={running ? 'danger' : 'success'} on:click={handlePauseResume}>
          {running ? '⏸ Pause' : '▶ Resume'}
        </Button>
        {#if showStep}
          <Button variant="default" on:click={() => dispatch('step')} disabled={running}>
            ⏭ Step
          </Button>
        {/if}
        <span class="status-text">
          {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
        </span>
      </div>
    {/if}

    {#if showRightControls}
      <div class="right-controls">
        <span class="info-text">
          {simulationName} at {currentFps} FPS
        </span>
      </div>
    {/if}
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
      <Button variant="default" on:click={handleBackClick}>← Back to Menu</Button>
      <Button variant="default" on:click={handleToggleUI}>👁 Show UI</Button>
      {#if controlModeButton}
        {@render controlModeButton()}
      {/if}
    </div>

    {#if showCenterControls}
      <div class="center-controls">
        <Button variant={running ? 'danger' : 'success'} on:click={handlePauseResume}>
          {running ? '⏸ Pause' : '▶ Resume'}
        </Button>
        {#if showStep}
          <Button variant="default" on:click={() => dispatch('step')} disabled={running}>
            ⏭ Step
          </Button>
        {/if}
        <span class="status-text">
          {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
        </span>
      </div>
    {/if}

    {#if showRightControls}
      <div class="right-controls">
        <span class="info-text">
          {simulationName} at {currentFps} FPS
        </span>
      </div>
    {/if}
  </div>
{/if}

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from './Button.svelte';

  const dispatch = createEventDispatcher();

  // Props
  export let running: boolean = false;
  export let loading: boolean = false;
  export let showUI: boolean = true;
  export let currentFps: number = 0;
  export let simulationName: string = 'Simulation';
  export let controlsVisible: boolean = true; // For auto-hide functionality
  export let showCenterControls: boolean = true; // Control center section visibility
  export let showRightControls: boolean = true; // Control right section visibility
  export let controlModeButton: import('svelte').Snippet | undefined = undefined;
  export let showStep: boolean = false; // Show optional step control when paused

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
    padding: 0.5rem;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 60px;
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

    .status-text,
    .info-text {
      font-size: 0.8rem;
    }
  }
</style>
