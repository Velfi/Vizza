<SimulationLayout
  simulationName="Voronoi CA"
  {running}
  loading={loading || !settings}
  {showUI}
  {currentFps}
  {controlsVisible}
  {menuPosition}
  on:back={returnToMenu}
  on:toggleUI={toggleBackendGui}
  on:pause={stopSimulation}
  on:resume={resumeSimulation}
  on:userInteraction={handleUserInteraction}
  on:mouseEvent={handleMouseEvent}
>
  {#if settings}
    <form on:submit|preventDefault>
      <fieldset>
        <legend>Rules</legend>
        <div class="control-group">
          <label>Rule (e.g. B3/S23)</label>
          <input type="text" bind:value={settings.rulestring} on:change={(e) => updateSetting('rulestring', (e.target as HTMLInputElement).value)} />
        </div>
        <div class="control-group">
          <label>Steps per frame</label>
          <NumberDragBox value={settings.steps_per_frame} min={1} max={64} step={1} precision={0} on:change={(e) => updateSetting('steps_per_frame', e.detail)} />
        </div>
        <div class="control-group">
          <label>Timestep</label>
          <NumberDragBox value={settings.timestep} min={0.01} max={5} step={0.01} precision={2} on:change={(e) => updateSetting('timestep', e.detail)} />
        </div>
      </fieldset>

      <fieldset>
        <legend>Painting</legend>
        <CursorConfig {cursorSize} {cursorStrength} sizeMin={1} sizeMax={200} sizeStep={1} strengthMin={0} strengthMax={5} strengthStep={0.1} on:sizechange={(e) => updateCursorSize(e.detail)} on:strengthchange={(e) => updateCursorStrength(e.detail)} />
        <div class="control-group">
          <Button on:click={seedRandom}>Seed Random</Button>
        </div>
        <div class="control-group">
          <label class="checkbox">
            <input type="checkbox" bind:checked={settings.auto_reseed_enabled} on:change={(e) => updateSetting('auto_reseed_enabled', (e.target as HTMLInputElement).checked)} />
            Auto Reseed
          </label>
        </div>
        <div class="control-group">
          <label>Reseed Interval (s)</label>
          <NumberDragBox value={settings.auto_reseed_interval_secs} min={0.5} max={120} step={0.5} precision={1} on:change={(e) => updateSetting('auto_reseed_interval_secs', e.detail)} />
        </div>
      </fieldset>

      <fieldset>
        <legend>Colors</legend>
        <LutSelector bind:available_luts current_lut={lut_name} reversed={lut_reversed} on:select={({ detail }) => updateLutName(detail.name)} on:reverse={() => updateLutReversed()} />
      </fieldset>

      <PostProcessingMenu simulationType="flow" />
    </form>
  {/if}
</SimulationLayout>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import NumberDragBox from './components/inputs/NumberDragBox.svelte';
  import LutSelector from './components/shared/LutSelector.svelte';
  import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
  import Button from './components/shared/Button.svelte';

  export let menuPosition: string = 'middle';

  type Settings = {
    rulestring: string;
    timestep: number;
    steps_per_frame: number;
    random_seed: number;
    brush_radius: number;
    brush_strength: number;
    auto_reseed_enabled: boolean;
    auto_reseed_interval_secs: number;
    lut_name: string;
    lut_reversed: boolean;
  };

  let settings: Settings | undefined;
  let loading = false;
  let running = false;
  let showUI = true;
  let controlsVisible = true;
  let currentFps = 0;

  let cursorSize = 10.0;
  let cursorStrength = 1.0;

  let available_luts: string[] = [];
  let lut_name = 'MATPLOTLIB_Blues';
  let lut_reversed = false;

  function returnToMenu() {
    dispatch('navigate', 'menu');
  }

  async function start() {
    loading = true;
    await invoke('start_simulation', { simulationType: 'voronoi_ca' });
    running = true;
    await syncSettings();
    loading = false;
  }

  async function stopSimulation() {
    await invoke('pause_simulation');
  }
  async function resumeSimulation() {
    await invoke('resume_simulation');
  }

  async function toggleBackendGui() {
    await invoke('toggle_gui');
  }

  async function updateSetting(name: string, value: any) {
    await invoke('update_simulation_setting', { settingName: name, value });
    await syncSettings();
  }

  async function syncSettings() {
    const s = (await invoke('get_current_settings')) as Settings;
    settings = s;
    cursorSize = settings.brush_radius;
    cursorStrength = settings.brush_strength;
    lut_name = settings.lut_name;
    lut_reversed = settings.lut_reversed;
  }

  async function updateCursorSize(size: number) {
    cursorSize = size;
    await invoke('update_cursor_size', { size });
  }
  async function updateCursorStrength(strength: number) {
    cursorStrength = strength;
    await invoke('update_cursor_strength', { strength });
  }

  async function seedRandom() {
    await invoke('reset_simulation');
  }

  async function updateLutName(name: string) {
    await invoke('apply_lut_by_name', { lutName: name });
    await syncSettings();
  }
  async function updateLutReversed() {
    await invoke('toggle_lut_reversed');
    await syncSettings();
  }

  async function handleUserInteraction() {}

  async function handleMouseEvent(e: CustomEvent) {
    const event = e.detail as MouseEvent;
    const dpr = window.devicePixelRatio || 1;
    const x = event.clientX * dpr;
    const y = event.clientY * dpr;

    if (event.type === 'mousedown' || event.type === 'mousemove' || event.type === 'contextmenu') {
      await invoke('handle_mouse_interaction_screen', { x, y, mouseButton: event.button });
    } else if (event.type === 'mouseup') {
      await invoke('handle_mouse_release', { mouseButton: event.button });
    }
  }

  async function loadLuts() {
    available_luts = (await invoke('get_available_luts')) as string[];
  }

  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();

  onMount(async () => {
    await start();
    await loadLuts();
  });

  onDestroy(async () => {
    await invoke('destroy_simulation');
  });
</script>

<style>
  @import './shared-theme.css';
</style>