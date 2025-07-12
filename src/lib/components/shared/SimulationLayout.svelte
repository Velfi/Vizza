<div
  class="simulation-container"
  on:mousedown={handleMouseEvent}
  on:mousemove={handleMouseEvent}
  on:mouseup={handleMouseEvent}
  on:wheel={handleMouseEvent}
  on:contextmenu={handleContextMenu}
  role="button"
  tabindex="0"
>
  <SimulationControlBar
    {simulationName}
    {running}
    {loading}
    {showUI}
    {currentFps}
    {controlsVisible}
    on:back={handleBack}
    on:toggleUI={handleToggleUI}
    on:pause={handlePause}
    on:resume={handleResume}
    on:reset={handleReset}
    on:randomize={handleRandomize}
    on:userInteraction={handleUserInteraction}
  />

  <SimulationMenuContainer position={menuPosition} {showUI}>
    <slot />
  </SimulationMenuContainer>

  <!-- Loading Screen -->
  {#if loading}
    <div class="loading-overlay">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <h2>Starting Simulation...</h2>
        <p>Initializing GPU resources</p>
      </div>
    </div>
  {/if}
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import SimulationControlBar from './SimulationControlBar.svelte';
  import SimulationMenuContainer from './SimulationMenuContainer.svelte';

  const dispatch = createEventDispatcher();

  export let simulationName: string;
  export let running: boolean = false;
  export let loading: boolean = false;
  export let showUI: boolean = true;
  export let currentFps: number = 0;
  export let controlsVisible: boolean = true;
  export let menuPosition: string = 'middle';

  // Event handlers
  function handleBack() {
    dispatch('back');
  }

  function handleToggleUI() {
    dispatch('toggleUI');
  }

  function handlePause() {
    dispatch('pause');
  }

  function handleResume() {
    dispatch('resume');
  }

  function handleReset() {
    dispatch('reset');
  }

  function handleRandomize() {
    dispatch('randomize');
  }

  function handleUserInteraction() {
    dispatch('userInteraction');
  }

  // Helper function to check if the event occurred directly on the container
  function isDirectTarget(event: Event): boolean {
    return event.target === event.currentTarget;
  }

  // Handle mouse events and forward to parent, but only if not on UI elements
  function handleMouseEvent(event: MouseEvent | WheelEvent) {
    if (isDirectTarget(event)) {
      event.preventDefault();
      dispatch('mouseEvent', event);
    }
  }

  // Handle context menu - only prevent default when clicking on simulation area
  function handleContextMenu(event: MouseEvent) {
    if (isDirectTarget(event)) {
      event.preventDefault();
      dispatch('mouseEvent', event);
    }
  }
</script>

<style>
  .simulation-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
    position: relative;
  }

  .loading-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .loading-content {
    text-align: center;
    color: white;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .loading-content h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1.5rem;
  }

  .loading-content p {
    margin: 0;
    opacity: 0.8;
  }
</style>
