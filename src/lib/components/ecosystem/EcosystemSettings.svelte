<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import NumberDragBox from '../inputs/NumberDragBox.svelte';
  import Selector from '../inputs/Selector.svelte';

  export let settings: any = {};

  const dispatch = createEventDispatcher();

  function handleSettingChange(setting: string, value: any) {
    dispatch('settingChange', { setting, value });
  }

  // Agent behavior options
  const agentCountOptions = ['500', '1000', '1500', '2000', '3000'];
  const speciesCountOptions = ['2', '3', '4', '5', '6'];
  const booleanOptions = ['On', 'Off'];

  function getAgentCountValue(stringValue: string): number {
    return parseInt(stringValue);
  }

  function getSpeciesCountValue(stringValue: string): number {
    return parseInt(stringValue);
  }

  function getBooleanValue(stringValue: string): boolean {
    return stringValue === 'On';
  }

  function getBooleanString(boolValue: boolean): string {
    return boolValue ? 'On' : 'Off';
  }
</script>

<div class="ecosystem-settings">
  <h3>Ecosystem Settings</h3>

  <div class="settings-section">
    <h4>Population</h4>
    <div class="setting-row">
      <label>Agent Count</label>
      <Selector
        options={agentCountOptions}
        value={String(settings.agent_count || 1000)}
        on:change={(e) => handleSettingChange('agent_count', getAgentCountValue(e.detail.value))}
      />
    </div>
    
    <div class="setting-row">
      <label>Species Count</label>
      <Selector
        options={speciesCountOptions}
        value={String(settings.species_count || 3)}
        on:change={(e) => handleSettingChange('species_count', getSpeciesCountValue(e.detail.value))}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Agent Behavior</h4>
    <div class="setting-row">
      <label>Speed Min</label>
      <NumberDragBox
        value={settings.agent_speed_min || 0.001}
        min={0.0001}
        max={0.01}
        step={0.0001}
        precision={4}
        on:change={(e) => handleSettingChange('agent_speed_min', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Speed Max</label>
      <NumberDragBox
        value={settings.agent_speed_max || 0.005}
        min={0.0005}
        max={0.02}
        step={0.0001}
        precision={4}
        on:change={(e) => handleSettingChange('agent_speed_max', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Turn Rate</label>
      <NumberDragBox
        value={settings.agent_turn_rate || 2.0}
        min={0.1}
        max={10.0}
        step={0.1}
        on:change={(e) => handleSettingChange('agent_turn_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Sensor Range</label>
      <NumberDragBox
        value={settings.sensor_range || 0.1}
        min={0.01}
        max={0.5}
        step={0.01}
        on:change={(e) => handleSettingChange('sensor_range', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Learning & Evolution</h4>
    <div class="setting-row">
      <label>Learning Rate</label>
      <NumberDragBox
        value={settings.learning_rate || 0.01}
        min={0.001}
        max={0.1}
        step={0.001}
        on:change={(e) => handleSettingChange('learning_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Mutation Rate</label>
      <NumberDragBox
        value={settings.mutation_rate || 0.05}
        min={0.01}
        max={0.2}
        step={0.01}
        on:change={(e) => handleSettingChange('mutation_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Energy Consumption</label>
      <NumberDragBox
        value={settings.energy_consumption_rate || 0.1}
        min={0.01}
        max={1.0}
        step={0.01}
        on:change={(e) => handleSettingChange('energy_consumption_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Energy from Food</label>
      <NumberDragBox
        value={settings.energy_gain_from_food || 10.0}
        min={1.0}
        max={50.0}
        step={1.0}
        on:change={(e) => handleSettingChange('energy_gain_from_food', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Chemical Environment</h4>
    <div class="setting-row">
      <label>Diffusion Rate</label>
      <NumberDragBox
        value={settings.chemical_diffusion_rate || 0.1}
        min={0.01}
        max={1.0}
        step={0.01}
        on:change={(e) => handleSettingChange('chemical_diffusion_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Decay Rate</label>
      <NumberDragBox
        value={settings.chemical_decay_rate || 0.01}
        min={0.001}
        max={0.1}
        step={0.001}
        on:change={(e) => handleSettingChange('chemical_decay_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Deposition Rate</label>
      <NumberDragBox
        value={settings.chemical_deposition_rate || 1.0}
        min={0.1}
        max={5.0}
        step={0.1}
        on:change={(e) => handleSettingChange('chemical_deposition_rate', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Environment</h4>
    <div class="setting-row">
      <label>Food Spawn Rate</label>
      <NumberDragBox
        value={settings.food_spawn_rate || 0.5}
        min={0.1}
        max={2.0}
        step={0.1}
        on:change={(e) => handleSettingChange('food_spawn_rate', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Brownian Motion</label>
      <NumberDragBox
        value={settings.brownian_motion_strength || 0.001}
        min={0.0}
        max={0.01}
        step={0.0001}
        on:change={(e) => handleSettingChange('brownian_motion_strength', e.detail)}
      />
    </div>
    
    <div class="setting-row">
      <label>Wrap Edges</label>
      <Selector
        options={booleanOptions}
        value={getBooleanString(settings.wrap_edges !== undefined ? settings.wrap_edges : true)}
        on:change={(e) => handleSettingChange('wrap_edges', getBooleanValue(e.detail.value))}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Visualization</h4>
    <div class="setting-row">
      <label>Show Chemical Trails</label>
      <Selector
        options={booleanOptions}
        value={getBooleanString(settings.show_chemical_trails !== undefined ? settings.show_chemical_trails : true)}
        on:change={(e) => handleSettingChange('show_chemical_trails', getBooleanValue(e.detail.value))}
      />
    </div>
    
    <div class="setting-row">
      <label>Show Energy as Size</label>
      <Selector
        options={booleanOptions}
        value={getBooleanString(settings.show_energy_as_size !== undefined ? settings.show_energy_as_size : true)}
        on:change={(e) => handleSettingChange('show_energy_as_size', getBooleanValue(e.detail.value))}
      />
    </div>
    
    <div class="setting-row">
      <label>Show Sensors</label>
      <Selector
        options={booleanOptions}
        value={getBooleanString(settings.show_sensors !== undefined ? settings.show_sensors : false)}
        on:change={(e) => handleSettingChange('show_sensors', getBooleanValue(e.detail.value))}
      />
    </div>
    
    <div class="setting-row">
      <label>Trail Opacity</label>
      <NumberDragBox
        value={settings.trail_opacity || 0.3}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('trail_opacity', e.detail)}
      />
    </div>
  </div>
</div>

<style>
  .ecosystem-settings {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .ecosystem-settings h3 {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1.1rem;
  }

  .settings-section {
    margin-bottom: 1.5rem;
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .settings-section h4 {
    margin: 0 0 0.75rem 0;
    color: rgba(255, 255, 255, 0.8);
    font-size: 1rem;
    font-weight: 500;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    padding-bottom: 0.25rem;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    gap: 1rem;
  }

  .setting-row:last-child {
    margin-bottom: 0;
  }

  .setting-row label {
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
    flex: 1;
    text-align: left;
  }

  .setting-row :global(.number-drag-box),
  .setting-row :global(.selector) {
    flex: 0 0 auto;
    min-width: 80px;
  }
</style> 