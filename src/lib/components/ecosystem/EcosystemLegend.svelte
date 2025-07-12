<div class="ecosystem-legend">
  <h3>Ecosystem Legend</h3>

  <div class="legend-section">
    <h4>Population Status</h4>
    <div class="population-summary">
      <div class="total-population">
        Total Population: <span class="population-count"
          >{getTotalPopulation().toLocaleString()}</span
        >
      </div>
      <div class="role-breakdown">
        <div class="role-item">
          <span class="role-name">Recyclers:</span>
          <span class="role-count">{getRolePopulation(0).toLocaleString()}</span>
        </div>
        <div class="role-item">
          <span class="role-name">Producers:</span>
          <span class="role-count">{getRolePopulation(1).toLocaleString()}</span>
        </div>
        <div class="role-item">
          <span class="role-name">Predators:</span>
          <span class="role-count">{getRolePopulation(2).toLocaleString()}</span>
        </div>
      </div>
    </div>
  </div>

  <div class="legend-section">
    <h4>Ecological Roles & Species</h4>
    <div class="visibility-instructions">
      💡 Click on any species icon to toggle its visibility in the simulation
    </div>

    <!-- Recyclers -->
    <div class="role-section">
      <h5 class="role-title recycler">🔄 Recyclers</h5>
      <div class="species-grid">
        {#each speciesInfo.slice(0, 3) as species, i}
          <div
            class="species-item"
            class:visible={speciesVisibility[i]}
            class:hidden={!speciesVisibility[i]}
          >
            <button
              class="species-toggle"
              on:click={() => toggleSpeciesVisibility(i)}
              title="Click to toggle visibility"
            >
              <div class="species-shape" data-species={i}>
                <div class="shape-preview"></div>
              </div>
              <div class="visibility-indicator">
                {speciesVisibility[i] ? '👁️' : '🙈'}
              </div>
            </button>
            <div class="species-info">
              <div class="species-header">
                <div class="species-name">{species.name}</div>
                <div class="species-population">
                  Pop: <span class="population-count">{getPopulationCount(i).toLocaleString()}</span
                  >
                </div>
              </div>
              <div class="species-description">{species.description}</div>
              <div class="species-details">
                <span class="behavior-label">Behavior: {species.behavior}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- Producers -->
    <div class="role-section">
      <h5 class="role-title producer">🌱 Producers</h5>
      <div class="species-grid">
        {#each speciesInfo.slice(3, 6) as species, i}
          <div
            class="species-item"
            class:visible={speciesVisibility[i + 3]}
            class:hidden={!speciesVisibility[i + 3]}
          >
            <button
              class="species-toggle"
              on:click={() => toggleSpeciesVisibility(i + 3)}
              title="Click to toggle visibility"
            >
              <div class="species-shape" data-species={i + 3}>
                <div class="shape-preview"></div>
              </div>
              <div class="visibility-indicator">
                {speciesVisibility[i + 3] ? '👁️' : '🙈'}
              </div>
            </button>
            <div class="species-info">
              <div class="species-header">
                <div class="species-name">{species.name}</div>
                <div class="species-population">
                  Pop: <span class="population-count"
                    >{getPopulationCount(i + 3).toLocaleString()}</span
                  >
                </div>
              </div>
              <div class="species-description">{species.description}</div>
              <div class="species-details">
                <span class="behavior-label">Behavior: {species.behavior}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- Predators -->
    <div class="role-section">
      <h5 class="role-title predator">🦠 Predators</h5>
      <div class="species-grid">
        {#each speciesInfo.slice(6, 9) as species, i}
          <div
            class="species-item"
            class:visible={speciesVisibility[i + 6]}
            class:hidden={!speciesVisibility[i + 6]}
          >
            <button
              class="species-toggle"
              on:click={() => toggleSpeciesVisibility(i + 6)}
              title="Click to toggle visibility"
            >
              <div class="species-shape" data-species={i + 6}>
                <div class="shape-preview"></div>
              </div>
              <div class="visibility-indicator">
                {speciesVisibility[i + 6] ? '👁️' : '🙈'}
              </div>
            </button>
            <div class="species-info">
              <div class="species-header">
                <div class="species-name">{species.name}</div>
                <div class="species-population">
                  Pop: <span class="population-count"
                    >{getPopulationCount(i + 6).toLocaleString()}</span
                  >
                </div>
              </div>
              <div class="species-description">{species.description}</div>
              <div class="species-details">
                <span class="behavior-label">Behavior: {species.behavior}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let syncTrigger = 0; // Prop to trigger visibility sync

  interface PopulationData {
    current?: {
      species_counts?: number[];
      total_population?: number;
    };
  }

  let populationData: PopulationData = {};
  let speciesVisibility: boolean[] = Array(9).fill(true);
  let updateInterval: number | null = null;

  const speciesInfo = [
    // Recyclers (Role 0)
    {
      name: 'Bacteria',
      role: 'Recycler',
      description: 'Fast decomposers, swarm behavior',
      shape: 'Circle',
      behavior: 'Chemical-seeking, rapid movement',
      color: 'Green',
    },
    {
      name: 'Fungi',
      role: 'Recycler',
      description: 'Network builders, slow decomposers',
      shape: 'Star',
      behavior: 'Network-forming, biofilm creation',
      color: 'Purple',
    },
    {
      name: 'Decomposer Protozoans',
      role: 'Recycler',
      description: 'Selective decomposers, moderate speed',
      shape: 'Square',
      behavior: 'Selective feeding, moderate movement',
      color: 'Brown',
    },

    // Producers (Role 1)
    {
      name: 'Algae',
      role: 'Producer',
      description: 'Photosynthetic, biofilm formation',
      shape: 'Circle',
      behavior: 'Light-seeking, slow movement',
      color: 'Cyan',
    },
    {
      name: 'Cyanobacteria',
      role: 'Producer',
      description: 'Mobile photosynthetic colonies',
      shape: 'Diamond',
      behavior: 'Light-seeking, moderate speed',
      color: 'Blue',
    },
    {
      name: 'Photosynthetic Protists',
      role: 'Producer',
      description: 'Complex movement patterns',
      shape: 'Triangle',
      behavior: 'Complex navigation, moderate speed',
      color: 'Teal',
    },

    // Predators (Role 2)
    {
      name: 'Predatory Bacteria',
      role: 'Predator',
      description: 'Coordinated group hunters',
      shape: 'Diamond',
      behavior: 'Pack hunting, fast movement',
      color: 'Red',
    },
    {
      name: 'Viruses',
      role: 'Predator',
      description: 'Extremely fast, inject into hosts',
      shape: 'Small Circle',
      behavior: 'Rapid infection, host targeting',
      color: 'Orange',
    },
    {
      name: 'Predatory Protozoans',
      role: 'Predator',
      description: 'Slow but powerful engulfers',
      shape: 'Large Circle',
      behavior: 'Engulfing prey, slow movement',
      color: 'Dark Red',
    },
  ];

  // Toggle visibility for a specific species variant
  async function toggleSpeciesVisibility(speciesIndex: number) {
    try {
      const ecologicalRole = Math.floor(speciesIndex / 3);
      const variant = speciesIndex % 3;

      await invoke('toggle_species_visibility', {
        ecologicalRole: ecologicalRole,
        variant: variant,
      });

      // Update local visibility state
      speciesVisibility[speciesIndex] = !speciesVisibility[speciesIndex];

      console.log(
        `Toggled visibility for species ${speciesIndex} (role: ${ecologicalRole}, variant: ${variant})`
      );
    } catch (error) {
      console.error('Failed to toggle species visibility:', error);
    }
  }

  // Fetch population data from backend
  async function fetchPopulationData() {
    try {
      const data = await invoke('get_ecosystem_population_data');
      populationData = data as PopulationData;
    } catch (error) {
      // Silently fail if ecosystem simulation isn't running
      populationData = {};
    }
  }

  // Sync visibility state with backend
  async function syncVisibilityState() {
    try {
      const visibilityFlags = (await invoke('get_species_visibility_state')) as number[];
      if (visibilityFlags && visibilityFlags.length === 9) {
        speciesVisibility = visibilityFlags.map((flag) => flag === 1);
      }
    } catch {
      // Silently fail if ecosystem simulation isn't running
      console.debug('Could not sync visibility state');
    }
  }

  // Get population count for a specific species
  function getPopulationCount(speciesIndex: number): number {
    return populationData.current?.species_counts?.[speciesIndex] || 0;
  }

  // Get total population
  function getTotalPopulation(): number {
    return populationData.current?.total_population || 0;
  }

  // Get role-based population totals
  function getRolePopulation(roleIndex: number): number {
    if (!populationData.current?.species_counts) return 0;
    let total = 0;
    for (let i = roleIndex * 3; i < (roleIndex + 1) * 3; i++) {
      total += populationData.current.species_counts[i] || 0;
    }
    return total;
  }

  onMount(() => {
    // Initial fetch
    fetchPopulationData();
    syncVisibilityState();

    // Update every 500ms for real-time data
    updateInterval = setInterval(fetchPopulationData, 500);
  });

  // Watch for sync trigger changes
  $: if (syncTrigger > 0) {
    syncVisibilityState();
  }

  onDestroy(() => {
    if (updateInterval) {
      clearInterval(updateInterval);
    }
  });
</script>

<style>
  .ecosystem-legend {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .ecosystem-legend h3 {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1.1rem;
  }

  .legend-section {
    margin-bottom: 1.5rem;
  }

  .legend-section:last-child {
    margin-bottom: 0;
  }

  .legend-section h4 {
    margin: 0 0 0.75rem 0;
    color: rgba(255, 255, 255, 0.8);
    font-size: 1rem;
    font-weight: 500;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    padding-bottom: 0.25rem;
  }

  .visibility-instructions {
    background: rgba(255, 193, 7, 0.1);
    border: 1px solid rgba(255, 193, 7, 0.3);
    border-radius: 4px;
    padding: 0.5rem;
    margin-bottom: 1rem;
    color: rgba(255, 193, 7, 0.9);
    font-size: 0.85rem;
    text-align: center;
  }

  .population-summary {
    background: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    text-align: center;
  }

  .total-population {
    color: rgba(255, 255, 255, 0.9);
    font-weight: 500;
    font-size: 1rem;
    margin-bottom: 0.5rem;
  }

  .role-breakdown {
    display: flex;
    justify-content: space-around;
    gap: 1rem;
    font-size: 0.9rem;
  }

  .role-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }

  .role-name {
    color: rgba(255, 255, 255, 0.7);
  }

  .role-count {
    color: #4caf50;
    font-weight: 600;
  }

  .population-count {
    color: #4caf50;
    font-weight: 700;
    font-size: 1.1em;
    text-shadow: 0 0 4px rgba(76, 175, 80, 0.3);
  }

  .role-section {
    margin-bottom: 1.5rem;
  }

  .role-section:last-child {
    margin-bottom: 0;
  }

  .role-title {
    margin: 0 0 0.75rem 0;
    font-size: 0.95rem;
    font-weight: 600;
    padding: 0.5rem;
    border-radius: 4px;
    text-align: center;
  }

  .role-title.recycler {
    background: rgba(76, 175, 80, 0.2);
    color: #4caf50;
    border: 1px solid rgba(76, 175, 80, 0.3);
  }

  .role-title.producer {
    background: rgba(33, 150, 243, 0.2);
    color: #2196f3;
    border: 1px solid rgba(33, 150, 243, 0.3);
  }

  .role-title.predator {
    background: rgba(244, 67, 54, 0.2);
    color: #f44336;
    border: 1px solid rgba(244, 67, 54, 0.3);
  }

  .species-grid {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .species-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    transition: background-color 0.2s ease;
  }

  .species-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .species-item.hidden {
    opacity: 0.4;
    filter: grayscale(0.5);
  }

  .species-item.visible {
    opacity: 1;
    filter: none;
  }

  .species-toggle {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    transition: transform 0.2s ease;
  }

  .species-toggle:hover {
    transform: scale(1.05);
  }

  .species-toggle:active {
    transform: scale(0.95);
  }

  .visibility-indicator {
    font-size: 0.8rem;
    opacity: 0.8;
    transition: opacity 0.2s ease;
  }

  .species-toggle:hover .visibility-indicator {
    opacity: 1;
  }

  .species-shape {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none; /* Prevent shape from interfering with button clicks */
  }

  .shape-preview {
    width: 24px;
    height: 24px;
    border: 2px solid currentColor;
  }

  /* Recyclers - all circles with green shades */
  .species-shape[data-species='0'] .shape-preview {
    border-radius: 50%;
    background: rgba(76, 175, 80, 0.3);
    border-color: #4caf50;
  }

  .species-shape[data-species='1'] .shape-preview {
    border-radius: 50%;
    background: rgba(33, 150, 34, 0.3);
    border-color: #219622;
  }

  .species-shape[data-species='2'] .shape-preview {
    border-radius: 50%;
    background: rgba(156, 216, 80, 0.3);
    border-color: #9cd850;
  }

  /* Producers - all diamonds with blue shades */
  .species-shape[data-species='3'] .shape-preview {
    width: 20px;
    height: 20px;
    transform: rotate(45deg);
    background: rgba(33, 150, 243, 0.3);
    border-color: #2196f3;
  }

  .species-shape[data-species='4'] .shape-preview {
    width: 20px;
    height: 20px;
    transform: rotate(45deg);
    background: rgba(0, 150, 216, 0.3);
    border-color: #0096d8;
  }

  .species-shape[data-species='5'] .shape-preview {
    width: 20px;
    height: 20px;
    transform: rotate(45deg);
    background: rgba(156, 216, 243, 0.3);
    border-color: #9cd8f3;
  }

  /* Predators - all triangles with red shades */
  .species-shape[data-species='6'] .shape-preview {
    width: 0;
    height: 0;
    border-left: 12px solid transparent;
    border-right: 12px solid transparent;
    border-bottom: 20px solid rgba(244, 67, 54, 0.3);
    border-color: transparent transparent #f44336 transparent;
  }

  .species-shape[data-species='7'] .shape-preview {
    width: 0;
    height: 0;
    border-left: 12px solid transparent;
    border-right: 12px solid transparent;
    border-bottom: 20px solid rgba(139, 0, 0, 0.3);
    border-color: transparent transparent #8b0000 transparent;
  }

  .species-shape[data-species='8'] .shape-preview {
    width: 0;
    height: 0;
    border-left: 12px solid transparent;
    border-right: 12px solid transparent;
    border-bottom: 20px solid rgba(255, 152, 152, 0.3);
    border-color: transparent transparent #ff9898 transparent;
  }

  .species-info {
    flex: 1;
  }

  .species-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .species-name {
    color: rgba(255, 255, 255, 0.9);
    font-weight: 500;
    font-size: 0.9rem;
  }

  .species-population {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
  }

  .species-description {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
    margin-bottom: 0.25rem;
  }

  .species-details {
    display: flex;
    gap: 1rem;
    font-size: 0.75rem;
  }

  .behavior-label {
    color: rgba(255, 255, 255, 0.6);
  }
</style>
