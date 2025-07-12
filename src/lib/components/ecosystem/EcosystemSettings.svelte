<div class="ecosystem-settings">
  <h3>Ecosystem Settings</h3>

  <div class="settings-section">
    <h4>Population</h4>
    <div class="setting-row">
      <label for="agent-count">Agent Count</label>
      <Selector
        id="agent-count"
        options={agentCountOptions}
        value={String(settings.agent_count || 1500)}
        on:change={(e) => handleSettingChange('agent_count', getAgentCountValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="carrying-capacity">Carrying Capacity</label>
      <NumberDragBox
        id="carrying-capacity"
        value={settings.carrying_capacity || 2000.0}
        min={500.0}
        max={5000.0}
        step={100.0}
        on:change={(e) => handleSettingChange('carrying_capacity', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Movement & Sensing</h4>
    <div class="setting-row">
      <label for="speed-min">Speed Min</label>
      <NumberDragBox
        id="speed-min"
        value={settings.agent_speed_min || 0.005}
        min={0.001}
        max={0.05}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('agent_speed_min', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="speed-max">Speed Max</label>
      <NumberDragBox
        id="speed-max"
        value={settings.agent_speed_max || 0.03}
        min={0.005}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('agent_speed_max', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="sensor-range">Sensor Range</label>
      <NumberDragBox
        id="sensor-range"
        value={settings.sensor_range || 0.15}
        min={0.01}
        max={0.5}
        step={0.01}
        on:change={(e) => handleSettingChange('sensor_range', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="sensor-count">Sensor Count</label>
      <NumberDragBox
        id="sensor-count"
        value={settings.sensor_count || 3}
        min={2}
        max={6}
        step={1}
        precision={0}
        on:change={(e) => handleSettingChange('sensor_count', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="sensor-angle">Sensor Angle</label>
      <NumberDragBox
        id="sensor-angle"
        value={settings.sensor_angle || 0.4}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('sensor_angle', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="chemotaxis-sensitivity">Chemotaxis Sensitivity</label>
      <NumberDragBox
        id="chemotaxis-sensitivity"
        value={settings.chemotaxis_sensitivity || 0.5}
        min={0.1}
        max={2.0}
        step={0.1}
        on:change={(e) => handleSettingChange('chemotaxis_sensitivity', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="brownian-motion">Brownian Motion</label>
      <NumberDragBox
        id="brownian-motion"
        value={settings.brownian_motion_strength || 0.002}
        min={0.0}
        max={0.01}
        step={0.0001}
        precision={4}
        on:change={(e) => handleSettingChange('brownian_motion_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="run-duration-min">Run Duration Min</label>
      <NumberDragBox
        id="run-duration-min"
        value={settings.run_duration_min || 0.5}
        min={0.1}
        max={2.0}
        step={0.1}
        on:change={(e) => handleSettingChange('run_duration_min', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="run-duration-max">Run Duration Max</label>
      <NumberDragBox
        id="run-duration-max"
        value={settings.run_duration_max || 2.0}
        min={1.0}
        max={5.0}
        step={0.1}
        on:change={(e) => handleSettingChange('run_duration_max', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Ecological Roles</h4>
    <div class="setting-row">
      <label for="recycler-efficiency">Recycler Efficiency</label>
      <NumberDragBox
        id="recycler-efficiency"
        value={settings.recycler_efficiency || 0.7}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('recycler_efficiency', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="producer-photosynthesis">Producer Photosynthesis</label>
      <NumberDragBox
        id="producer-photosynthesis"
        value={settings.producer_photosynthesis_rate || 0.8}
        min={0.1}
        max={2.0}
        step={0.05}
        on:change={(e) => handleSettingChange('producer_photosynthesis_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="predator-hunting">Predator Hunting Efficiency</label>
      <NumberDragBox
        id="predator-hunting"
        value={settings.predator_hunting_efficiency || 0.6}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('predator_hunting_efficiency', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Hunting Mechanics</h4>
    <div class="setting-row">
      <label for="min-predation-size">Min Predation Size Ratio</label>
      <NumberDragBox
        id="min-predation-size"
        value={settings.min_predation_size_ratio || 0.8}
        min={0.5}
        max={2.0}
        step={0.1}
        on:change={(e) => handleSettingChange('min_predation_size_ratio', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="predation-contact-range">Predation Contact Range</label>
      <NumberDragBox
        id="predation-contact-range"
        value={settings.predation_contact_range || 0.05}
        min={0.01}
        max={0.2}
        step={0.005}
        precision={3}
        on:change={(e) => handleSettingChange('predation_contact_range', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="predation-success-rate">Predation Success Rate</label>
      <NumberDragBox
        id="predation-success-rate"
        value={settings.predation_success_rate || 0.3}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('predation_success_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="pack-hunting-bonus">Pack Hunting Bonus</label>
      <NumberDragBox
        id="pack-hunting-bonus"
        value={settings.pack_hunting_bonus || 1.5}
        min={1.0}
        max={3.0}
        step={0.1}
        on:change={(e) => handleSettingChange('pack_hunting_bonus', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Energy & Metabolism</h4>
    <div class="setting-row">
      <label for="energy-consumption">Energy Consumption</label>
      <NumberDragBox
        id="energy-consumption"
        value={settings.energy_consumption_rate || 0.004}
        min={0.001}
        max={0.01}
        step={0.0001}
        precision={4}
        on:change={(e) => handleSettingChange('energy_consumption_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="energy-from-food">Energy from Food</label>
      <NumberDragBox
        id="energy-from-food"
        value={settings.energy_gain_from_food || 25.0}
        min={5.0}
        max={50.0}
        step={1.0}
        on:change={(e) => handleSettingChange('energy_gain_from_food', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="reproduction-threshold">Reproduction Threshold</label>
      <NumberDragBox
        id="reproduction-threshold"
        value={settings.reproduction_energy_threshold || 65.0}
        min={30.0}
        max={100.0}
        step={5.0}
        on:change={(e) => handleSettingChange('reproduction_energy_threshold', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="reproduction-probability">Reproduction Probability</label>
      <NumberDragBox
        id="reproduction-probability"
        value={settings.reproduction_probability || 0.015}
        min={0.001}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('reproduction_probability', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="mutation-rate">Mutation Rate</label>
      <NumberDragBox
        id="mutation-rate"
        value={settings.mutation_rate || 0.02}
        min={0.001}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('mutation_rate', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Chemical Environment</h4>
    <div class="setting-row">
      <label for="diffusion-rate">Diffusion Rate</label>
      <NumberDragBox
        id="diffusion-rate"
        value={settings.chemical_diffusion_rate || 0.08}
        min={0.01}
        max={0.5}
        step={0.01}
        on:change={(e) => handleSettingChange('chemical_diffusion_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="decay-rate">Decay Rate</label>
      <NumberDragBox
        id="decay-rate"
        value={settings.chemical_decay_rate || 0.015}
        min={0.001}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('chemical_decay_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="deposition-rate">Deposition Rate</label>
      <NumberDragBox
        id="deposition-rate"
        value={settings.chemical_deposition_rate || 0.3}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('chemical_deposition_rate', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Fluid Dynamics</h4>
    <div class="setting-row">
      <label for="enable-fluid-dynamics">Enable Fluid Dynamics</label>
      <Selector
        id="enable-fluid-dynamics"
        options={booleanOptions}
        value={getBooleanString(
          settings.enable_fluid_dynamics !== undefined ? settings.enable_fluid_dynamics : true
        )}
        on:change={(e) =>
          handleSettingChange('enable_fluid_dynamics', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="fluid-viscosity">Fluid Viscosity</label>
      <NumberDragBox
        id="fluid-viscosity"
        value={settings.fluid_viscosity || 0.1}
        min={0.01}
        max={1.0}
        step={0.01}
        on:change={(e) => handleSettingChange('fluid_viscosity', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="fluid-density">Fluid Density</label>
      <NumberDragBox
        id="fluid-density"
        value={settings.fluid_density || 1.0}
        min={0.1}
        max={3.0}
        step={0.1}
        on:change={(e) => handleSettingChange('fluid_density', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="biological-current">Biological Current Strength</label>
      <NumberDragBox
        id="biological-current"
        value={settings.biological_current_strength || 0.05}
        min={0.0}
        max={0.2}
        step={0.01}
        on:change={(e) => handleSettingChange('biological_current_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="chemical-current">Chemical Current Strength</label>
      <NumberDragBox
        id="chemical-current"
        value={settings.chemical_current_strength || 0.03}
        min={0.0}
        max={0.2}
        step={0.01}
        on:change={(e) => handleSettingChange('chemical_current_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="flow-update-frequency">Flow Update Frequency</label>
      <NumberDragBox
        id="flow-update-frequency"
        value={settings.flow_update_frequency || 4}
        min={1}
        max={20}
        step={1}
        precision={0}
        on:change={(e) => handleSettingChange('flow_update_frequency', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Advanced Movement</h4>
    <div class="setting-row">
      <label for="tumble-angle-range">Tumble Angle Range</label>
      <NumberDragBox
        id="tumble-angle-range"
        value={settings.tumble_angle_range || 3.14}
        min={0.1}
        max={6.28}
        step={0.1}
        on:change={(e) => handleSettingChange('tumble_angle_range', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="flagella-strength">Flagella Strength</label>
      <NumberDragBox
        id="flagella-strength"
        value={settings.flagella_strength || 1.0}
        min={0.1}
        max={3.0}
        step={0.1}
        on:change={(e) => handleSettingChange('flagella_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="receptor-saturation">Receptor Saturation Threshold</label>
      <NumberDragBox
        id="receptor-saturation"
        value={settings.receptor_saturation_threshold || 5.0}
        min={1.0}
        max={20.0}
        step={0.5}
        on:change={(e) => handleSettingChange('receptor_saturation_threshold', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Spatial Organization</h4>
    <div class="setting-row">
      <label for="nutrient-stream-threshold">Nutrient Stream Threshold</label>
      <NumberDragBox
        id="nutrient-stream-threshold"
        value={settings.nutrient_stream_threshold || 2.0}
        min={0.5}
        max={10.0}
        step={0.1}
        on:change={(e) => handleSettingChange('nutrient_stream_threshold', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="territory-establishment">Territory Establishment Range</label>
      <NumberDragBox
        id="territory-establishment"
        value={settings.territory_establishment_range || 0.3}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('territory_establishment_range', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Population Dynamics</h4>
    <div class="setting-row">
      <label for="population-oscillation">Population Oscillation Damping</label>
      <NumberDragBox
        id="population-oscillation"
        value={settings.population_oscillation_damping || 0.05}
        min={0.01}
        max={1.0}
        step={0.01}
        on:change={(e) => handleSettingChange('population_oscillation_damping', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="resource-competition">Resource Competition Strength</label>
      <NumberDragBox
        id="resource-competition"
        value={settings.resource_competition_strength || 0.25}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('resource_competition_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="succession-pattern">Succession Pattern Strength</label>
      <NumberDragBox
        id="succession-pattern"
        value={settings.succession_pattern_strength || 0.12}
        min={0.0}
        max={1.0}
        step={0.01}
        on:change={(e) => handleSettingChange('succession_pattern_strength', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Environmental Factors</h4>
    <div class="setting-row">
      <label for="toxin-accumulation">Toxin Accumulation Rate</label>
      <NumberDragBox
        id="toxin-accumulation"
        value={settings.toxin_accumulation_rate || 0.01}
        min={0.0}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('toxin_accumulation_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="dead-zone-threshold">Dead Zone Threshold</label>
      <NumberDragBox
        id="dead-zone-threshold"
        value={settings.dead_zone_threshold || 8.0}
        min={1.0}
        max={20.0}
        step={0.5}
        on:change={(e) => handleSettingChange('dead_zone_threshold', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Nutrient Architecture</h4>
    <div class="setting-row">
      <label for="max-particles">Max Particles</label>
      <NumberDragBox
        id="max-particles"
        value={settings.max_particles || 600}
        min={100}
        max={2000}
        step={50}
        precision={0}
        on:change={(e) => handleSettingChange('max_particles', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="particle-decomposition">Particle Decomposition Rate</label>
      <NumberDragBox
        id="particle-decomposition"
        value={settings.particle_decomposition_rate || 0.01}
        min={0.001}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('particle_decomposition_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="particle-decay">Particle Decay Rate</label>
      <NumberDragBox
        id="particle-decay"
        value={settings.particle_decay_rate || 0.005}
        min={0.001}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('particle_decay_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="matter-to-chemical">Matter to Chemical Ratio</label>
      <NumberDragBox
        id="matter-to-chemical"
        value={settings.matter_to_chemical_ratio || 0.75}
        min={0.1}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('matter_to_chemical_ratio', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Light Gradient</h4>
    <div class="setting-row">
      <label for="enable-light-gradient">Enable Light Gradient</label>
      <Selector
        id="enable-light-gradient"
        options={booleanOptions}
        value={getBooleanString(
          settings.enable_light_gradient !== undefined ? settings.enable_light_gradient : true
        )}
        on:change={(e) =>
          handleSettingChange('enable_light_gradient', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="base-light-intensity">Base Light Intensity</label>
      <NumberDragBox
        id="base-light-intensity"
        value={settings.base_light_intensity || 0.3}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('base_light_intensity', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="light-gradient-strength">Light Gradient Strength</label>
      <NumberDragBox
        id="light-gradient-strength"
        value={settings.light_gradient_strength || 0.7}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('light_gradient_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="light-rotation-speed">Light Rotation Speed</label>
      <NumberDragBox
        id="light-rotation-speed"
        value={settings.light_rotation_speed || 0.1}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('light_rotation_speed', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Biofilm Formation</h4>
    <div class="setting-row">
      <label for="enable-biofilm">Enable Biofilm Formation</label>
      <Selector
        id="enable-biofilm"
        options={booleanOptions}
        value={getBooleanString(
          settings.enable_biofilm_formation !== undefined ? settings.enable_biofilm_formation : true
        )}
        on:change={(e) =>
          handleSettingChange('enable_biofilm_formation', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="biofilm-growth-rate">Biofilm Growth Rate</label>
      <NumberDragBox
        id="biofilm-growth-rate"
        value={settings.biofilm_growth_rate || 0.02}
        min={0.001}
        max={0.1}
        step={0.001}
        precision={3}
        on:change={(e) => handleSettingChange('biofilm_growth_rate', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="biofilm-persistence">Biofilm Persistence</label>
      <NumberDragBox
        id="biofilm-persistence"
        value={settings.biofilm_persistence || 0.95}
        min={0.8}
        max={1.0}
        step={0.01}
        on:change={(e) => handleSettingChange('biofilm_persistence', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Environment</h4>
    <div class="setting-row">
      <label for="wrap-edges">Wrap Edges</label>
      <Selector
        id="wrap-edges"
        options={booleanOptions}
        value={getBooleanString(settings.wrap_edges !== undefined ? settings.wrap_edges : false)}
        on:change={(e) => handleSettingChange('wrap_edges', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="temperature-gradient">Temperature Gradient</label>
      <NumberDragBox
        id="temperature-gradient"
        value={settings.temperature_gradient_strength || 0.1}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('temperature_gradient_strength', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="ph-gradient">pH Gradient</label>
      <NumberDragBox
        id="ph-gradient"
        value={settings.ph_gradient_strength || 0.05}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('ph_gradient_strength', e.detail)}
      />
    </div>
  </div>

  <div class="settings-section">
    <h4>Visualization</h4>

    <!-- Chemical Field Controls -->
    <div class="setting-row">
      <label for="show-chemical-fields">Show Chemical Fields</label>
      <Selector
        id="show-chemical-fields"
        options={booleanOptions}
        value={getBooleanString(
          settings.show_chemical_fields !== undefined ? settings.show_chemical_fields : false
        )}
        on:change={(e) =>
          handleSettingChange('show_chemical_fields', getBooleanValue(e.detail.value))}
      />
    </div>

    {#if settings.show_chemical_fields}
      <div class="sub-section">
        <div class="setting-row">
          <label for="show-oxygen">Show Oxygen (O₂)</label>
          <Selector
            id="show-oxygen"
            options={booleanOptions}
            value={getBooleanString(
              settings.show_oxygen !== undefined ? settings.show_oxygen : false
            )}
            on:change={(e) => handleSettingChange('show_oxygen', getBooleanValue(e.detail.value))}
          />
        </div>

        <div class="setting-row">
          <label for="show-co2">Show CO₂</label>
          <Selector
            id="show-co2"
            options={booleanOptions}
            value={getBooleanString(settings.show_co2 !== undefined ? settings.show_co2 : false)}
            on:change={(e) => handleSettingChange('show_co2', getBooleanValue(e.detail.value))}
          />
        </div>

        <div class="setting-row">
          <label for="show-nitrogen">Show Nitrogen</label>
          <Selector
            id="show-nitrogen"
            options={booleanOptions}
            value={getBooleanString(
              settings.show_nitrogen !== undefined ? settings.show_nitrogen : false
            )}
            on:change={(e) => handleSettingChange('show_nitrogen', getBooleanValue(e.detail.value))}
          />
        </div>

        <div class="setting-row">
          <label for="show-pheromones">Show Pheromones</label>
          <Selector
            id="show-pheromones"
            options={booleanOptions}
            value={getBooleanString(
              settings.show_pheromones !== undefined ? settings.show_pheromones : false
            )}
            on:change={(e) =>
              handleSettingChange('show_pheromones', getBooleanValue(e.detail.value))}
          />
        </div>

        <div class="setting-row">
          <label for="show-toxins">Show Toxins</label>
          <Selector
            id="show-toxins"
            options={booleanOptions}
            value={getBooleanString(
              settings.show_toxins !== undefined ? settings.show_toxins : false
            )}
            on:change={(e) => handleSettingChange('show_toxins', getBooleanValue(e.detail.value))}
          />
        </div>

        <div class="setting-row">
          <label for="show-attractants">Show Attractants</label>
          <Selector
            id="show-attractants"
            options={booleanOptions}
            value={getBooleanString(
              settings.show_attractants !== undefined ? settings.show_attractants : false
            )}
            on:change={(e) =>
              handleSettingChange('show_attractants', getBooleanValue(e.detail.value))}
          />
        </div>

        <div class="setting-row">
          <label for="chemical-field-opacity">Chemical Field Opacity</label>
          <NumberDragBox
            id="chemical-field-opacity"
            value={settings.chemical_field_opacity || 0.5}
            min={0.0}
            max={1.0}
            step={0.05}
            on:change={(e) => handleSettingChange('chemical_field_opacity', e.detail)}
          />
        </div>
      </div>
    {/if}

    <!-- Environmental Overlays -->
    <div class="setting-row">
      <label for="show-light-gradient">Show Light Gradient</label>
      <Selector
        id="show-light-gradient"
        options={booleanOptions}
        value={getBooleanString(
          settings.show_light_gradient !== undefined ? settings.show_light_gradient : false
        )}
        on:change={(e) =>
          handleSettingChange('show_light_gradient', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="show-temperature-zones">Show Temperature Zones</label>
      <Selector
        id="show-temperature-zones"
        options={booleanOptions}
        value={getBooleanString(
          settings.show_temperature_zones !== undefined ? settings.show_temperature_zones : false
        )}
        on:change={(e) =>
          handleSettingChange('show_temperature_zones', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="show-ph-zones">Show pH Zones</label>
      <Selector
        id="show-ph-zones"
        options={booleanOptions}
        value={getBooleanString(
          settings.show_ph_zones !== undefined ? settings.show_ph_zones : false
        )}
        on:change={(e) => handleSettingChange('show_ph_zones', getBooleanValue(e.detail.value))}
      />
    </div>

    <div class="setting-row">
      <label for="environmental-opacity">Environmental Opacity</label>
      <NumberDragBox
        id="environmental-opacity"
        value={settings.environmental_opacity || 0.3}
        min={0.0}
        max={1.0}
        step={0.05}
        on:change={(e) => handleSettingChange('environmental_opacity', e.detail)}
      />
    </div>

    <div class="setting-row">
      <label for="show-energy-as-size">Show Energy as Size</label>
      <Selector
        id="show-energy-as-size"
        options={booleanOptions}
        value={getBooleanString(
          settings.show_energy_as_size !== undefined ? settings.show_energy_as_size : false
        )}
        on:change={(e) =>
          handleSettingChange('show_energy_as_size', getBooleanValue(e.detail.value))}
      />
    </div>
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import NumberDragBox from '../inputs/NumberDragBox.svelte';
  import Selector from '../inputs/Selector.svelte';

  interface EcosystemSettings {
    agent_count?: number;
    species_count?: number;
    show_pheromones?: boolean;
    show_toxins?: boolean;
    show_attractants?: boolean;
    chemical_field_opacity?: number;
    show_light_gradient?: boolean;
    show_temperature_zones?: boolean;
    show_ph_zones?: boolean;
    environmental_opacity?: number;
    show_energy_as_size?: boolean;
    [key: string]: unknown;
  }

  export let settings: EcosystemSettings = {};

  const dispatch = createEventDispatcher();

  function handleSettingChange(setting: string, value: number | boolean) {
    dispatch('settingChange', { setting, value });
  }

  // Agent behavior options
  const agentCountOptions = ['500', '1000', '1500', '2000', '3000'];
  const booleanOptions = ['On', 'Off'];

  function getAgentCountValue(stringValue: string): number {
    return parseInt(stringValue);
  }

  function getBooleanValue(stringValue: string): boolean {
    return stringValue === 'On';
  }

  function getBooleanString(boolValue: boolean): string {
    return boolValue ? 'On' : 'Off';
  }
</script>

<style>
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

  .sub-section {
    margin-top: 1rem;
    padding-left: 1.5rem;
    border-left: 1px solid rgba(255, 255, 255, 0.1);
  }
</style>
