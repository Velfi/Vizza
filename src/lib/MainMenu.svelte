<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();
  let renderLoopId: number | null = null;

  function selectSimulation(simulation: string) {
    dispatch('navigate', simulation);
  }

  async function startIdleRenderLoop() {
    async function renderLoop() {
      if (renderLoopId === null) return; // Check if we should stop
      
      try {
        await invoke('render_frame');
      } catch (e) {
        console.error('Idle render failed:', e);
      }
      
      if (renderLoopId !== null) {
        renderLoopId = requestAnimationFrame(renderLoop);
      }
    }

    renderLoopId = requestAnimationFrame(renderLoop);
  }

  function stopIdleRenderLoop() {
    if (renderLoopId !== null) {
      cancelAnimationFrame(renderLoopId);
      renderLoopId = null;
    }
  }

  onMount(() => {
    startIdleRenderLoop();
  });

  onDestroy(() => {
    stopIdleRenderLoop();
  });
</script>

<div class="menu-container">
  <h1>Sim-Pix</h1>
  <p>Select a simulation:</p>
  
  <div class="simulation-grid">
    <button 
      class="simulation-card" 
      on:click={() => selectSimulation('slime-mold')}
    >
      <h3>Slime Mold</h3>
      <p>Cellular automaton simulation</p>
    </button>
    
    <div class="simulation-card disabled">
      <h3>Coming Soon</h3>
      <p>More simulations</p>
    </div>
  </div>
</div>

<style>
  .menu-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
    text-align: center;
    background: rgba(0, 0, 0, 0.5);
  }

  h1 {
    font-size: 3rem;
    margin-bottom: 1rem;
    color: rgba(255, 255, 255, 0.87);
  }

  p {
    font-size: 1.2rem;
    margin-bottom: 2rem;
    color: rgba(255, 255, 255, 0.7);
  }

  .simulation-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.5rem;
    max-width: 600px;
    width: 100%;
  }

  .simulation-card {
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    padding: 2rem;
    cursor: pointer;
    transition: all 0.3s ease;
    color: inherit;
    font-family: inherit;
    text-align: center;
  }

  .simulation-card:hover:not(.disabled) {
    background: rgba(255, 255, 255, 0.15);
    border-color: rgba(255, 255, 255, 0.4);
    transform: translateY(-2px);
  }

  .simulation-card.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .simulation-card h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.5rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .simulation-card p {
    margin: 0;
    font-size: 1rem;
    color: rgba(255, 255, 255, 0.7);
  }
</style>