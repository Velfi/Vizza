# Microbial Ecosystem Simulation TODO

## Current State Analysis

### ✅ Already Implemented
- Basic ecosystem simulation framework
- Agent struct with emotional states, neural networks, memory systems
- 6 chemical types (O2, CO2, N, pheromones, toxins, attractants)
- 4 species types (cyanobacteria, heterotrophs, predators, fungi)
- Enhanced neural networks (12 inputs, 2 hidden layers, 6 outputs)
- Chemical diffusion and reaction systems
- GPU compute shaders for agent updates
- Basic UI components for settings

### 🔄 Partially Implemented
- Neural network weights structure (200 weights broken into chunks)
- Memory systems (short-term and long-term)
- Species-specific parameters in settings
- Chemical field visualization

### ❌ Not Yet Implemented
- Lotka-Volterra population dynamics
- Default preset with stable oscillations
- Population tracking and visualization
- Species-specific behaviors
- Environmental gradients and obstacles
- Advanced collective behaviors

## Phase 1: Lotka-Volterra Foundation

### Core Population Dynamics
- [ ] **Implement Lotka-Volterra equations** in agent behavior
  - [ ] Add population tracking per species
  - [ ] Implement exponential growth for prey (α parameter)
  - [ ] Add predation pressure proportional to predator density (β parameter)
  - [ ] Implement natural death rate for predators (γ parameter)
  - [ ] Add predator growth from prey consumption (δ parameter)
  - [ ] Add carrying capacity limits to prevent unbounded growth

### Default Preset Calibration
- [ ] **Create stable default preset**
  - [ ] Set initial populations: 1000 cyanobacteria, 800 heterotrophs, 200 predators
  - [ ] Calibrate α, β, γ, δ parameters for 3-4 year oscillation cycles
  - [ ] Test and adjust for stable coexistence without extinction
  - [ ] Ensure predator peaks lag prey peaks by ~1 year

### Population Visualization
- [ ] **Add real-time population graphs**
  - [ ] Create population tracking system in Rust backend
  - [ ] Implement Lotka-Volterra curves display
  - [ ] Add phase space plot (predator vs prey populations)
  - [ ] Create population statistics panel

### Parameter Controls
- [ ] **Add Lotka-Volterra parameter sliders**
  - [ ] α (prey growth rate) slider
  - [ ] β (predation efficiency) slider
  - [ ] γ (predator death rate) slider
  - [ ] δ (predator growth efficiency) slider
  - [ ] Carrying capacity controls
  - [ ] Stability indicators

## Phase 2: Enhanced Agent Behaviors

### Species-Specific Behaviors
- [ ] **Implement distinct behaviors for each trophic level**
  - [ ] **Cyanobacteria**: Move toward light, form biofilms, produce oxygen
  - [ ] **Heterotrophs**: Follow chemical gradients, consume waste, reproduce rapidly
  - [ ] **Predators**: Hunt in packs, coordinate attacks, show territorial behavior
  - [ ] **Fungi**: Form branching networks, transport nutrients, create symbioses

### Neural Network Implementation
- [ ] **Complete neural network functionality**
  - [ ] Implement 12 inputs: light (3), chemicals (3×3), temperature (3), neighbors (3), energy, age, emotions (4)
  - [ ] Add 2 hidden layers with ReLU activation (16 + 12 neurons)
  - [ ] Expand outputs to 6: movement (x,y), speed, chemical secretion, reproduction, interaction, memory
  - [ ] Add backpropagation for learning from experiences
  - [ ] Implement memory consolidation and retrieval

### Emotional and Memory Systems
- [ ] **Enhance emotional state management**
  - [ ] Fear: increases with predators, triggers fleeing
  - [ ] Hunger: drives food-seeking behavior
  - [ ] Curiosity: drives exploration
  - [ ] Aggression: territorial defense
- [ ] **Complete memory system**
  - [ ] Short-term memory for recent experiences (10-20 frames)
  - [ ] Long-term memory for learned behaviors
  - [ ] Spatial memory for locations of food, danger, mates

## Phase 3: Chemical Ecology Enhancement

### Chemical Communication
- [ ] **Implement pheromone-based communication**
  - [ ] Danger signals for predator warnings
  - [ ] Food alerts for resource discovery
  - [ ] Mating calls for reproduction coordination
  - [ ] Territory markers for boundary definition
- [ ] **Chemical gradient navigation**
- [ ] **Collective decision making through chemical signals**

### Metabolic Networks
- [ ] **Waste-to-food conversion chains**
  - [ ] Oxygen production by cyanobacteria
  - [ ] CO2 consumption by producers
  - [ ] Nitrogen cycling between species
  - [ ] Toxin production and degradation
- [ ] **Energy flow between trophic levels**
- [ ] **Nutrient cycling and recycling**

### Chemical Reactions
- [ ] **Implement chemical reactions**
  - [ ] O2 + organic matter → CO2 + energy
  - [ ] CO2 + light → O2 + organic matter
  - [ ] Nitrogen fixation and cycling
  - [ ] Toxin neutralization reactions

## Phase 4: Environmental Complexity

### Dynamic Gradients
- [ ] **Implement environmental gradients**
  - [ ] Oxygen gradients based on producer distribution
  - [ ] Temperature zones with hot spots near energy sources
  - [ ] pH gradients affecting species growth
  - [ ] Light gradients for photosynthetic species
  - [ ] Nutrient patches with concentrated resources

### Physical Environment
- [ ] **Add obstacles and barriers**
  - [ ] Solid barriers creating microhabitats
  - [ ] Channels and flow paths for chemicals and movement
  - [ ] Pores and small openings for size-based filtering
  - [ ] Surface properties affecting biofilm formation
- [ ] **Dynamic environmental changes**
  - [ ] Seasonal cycles affecting environmental conditions
  - [ ] Catastrophic events testing adaptability
  - [ ] Resource pulses providing periodic abundance

### Environmental Interactions
- [ ] **Agent-environment interactions**
  - [ ] Biofilm formation on surfaces
  - [ ] Tunneling through obstacles
  - [ ] Environmental modification by agents
  - [ ] Habitat creation and destruction

## Phase 5: Collective Behaviors

### Emergent Behaviors
- [ ] **Implement collective behaviors**
  - [ ] Swarming and flocking for similar species
  - [ ] Coordinated hunting for predators
  - [ ] Feeding aggregations around resources
  - [ ] Migration patterns across environment
  - [ ] Territorial formation and defense

### Spatial Patterns
- [ ] **Biofilm formation** by producers
- [ ] **Hunting packs** by predators
- [ ] **Resource corridors** between food sources
- [ ] **Refuge areas** for vulnerable species
- [ ] **Territorial boundaries** between species

### Temporal Patterns
- [ ] **Population cycles** (Lotka-Volterra dynamics)
- [ ] **Succession** changes in dominant species over time
- [ ] **Adaptation waves** evolution of new behaviors
- [ ] **Boom-bust dynamics** with realistic timing

## Phase 6: UI and Visualization

### Population Visualization
- [ ] **Real-time population graphs**
  - [ ] Lotka-Volterra curves for each species
  - [ ] Phase space plots showing predator-prey relationships
  - [ ] Population statistics and trends
  - [ ] Stability indicators and warnings

### Environmental Visualization
- [ ] **Chemical field visualization**
  - [ ] Heat maps for different chemical types
  - [ ] Flow lines showing chemical gradients
  - [ ] Overlay options for multiple chemicals
- [ ] **Environmental overlay**
  - [ ] Gradient visualization (oxygen, temperature, pH)
  - [ ] Obstacle and barrier display
  - [ ] Nutrient patch highlighting

### Interactive Features
- [ ] **Species selection and focus**
  - [ ] Click to focus on specific species
  - [ ] Species-specific statistics display
  - [ ] Individual agent tracking
- [ ] **Environmental controls**
  - [ ] Gradient adjustment sliders
  - [ ] Obstacle placement and removal
  - [ ] Environmental parameter controls
- [ ] **Preset library**
  - [ ] Stable oscillation presets
  - [ ] Chaotic dynamics presets
  - [ ] Extinction scenario presets
  - [ ] Custom preset saving and loading

## Phase 7: Performance and Polish

### Performance Optimization
- [ ] **GPU optimization**
  - [ ] Spatial partitioning for neighbor detection
  - [ ] Efficient memory access patterns
  - [ ] Workgroup optimization for large populations
  - [ ] Chemical field computation optimization
- [ ] **Scalability improvements**
  - [ ] Support for 10,000+ agents at 60 FPS
  - [ ] Multi-resolution rendering for performance
  - [ ] Memory management for large populations

### Settings and Configuration
- [ ] **Expand Settings struct**
  - [ ] Lotka-Volterra parameters (α, β, γ, δ)
  - [ ] Species-specific settings
  - [ ] Environmental parameters
  - [ ] Chemical system parameters
  - [ ] Behavior parameters
- [ ] **Update UI components**
  - [ ] Add new parameter controls
  - [ ] Organize settings into logical groups
  - [ ] Add tooltips and help text

### Testing and Validation
- [ ] **Behavioral complexity validation**
  - [ ] At least 5 distinct emergent behaviors
  - [ ] Realistic population dynamics
  - [ ] Stable species interactions
  - [ ] Environmental adaptation
- [ ] **Performance benchmarking**
- [ ] **Edge case handling**

## Implementation Notes

### Breaking Changes Required
- **Agent struct**: Already expanded, needs behavior implementation
- **Neural network**: Structure exists, needs full implementation
- **Chemical system**: 6 types implemented, needs reactions
- **Species system**: 4 types defined, needs specialized behaviors
- **Settings**: Expanded, needs UI updates

### Migration Strategy
1. **Phase 1**: Implement Lotka-Volterra dynamics and default preset
2. **Phase 2**: Complete neural network and species behaviors
3. **Phase 3**: Enhance chemical ecology and communication
4. **Phase 4**: Add environmental complexity
5. **Phase 5**: Implement collective behaviors
6. **Phase 6**: Polish UI and visualization
7. **Phase 7**: Optimize performance and final polish

### Success Criteria
- **Stable Lotka-Volterra oscillations** in default preset
- **At least 5 distinct emergent behaviors** observed
- **Population dynamics** show realistic boom/bust cycles
- **Species interactions** create stable communities
- **Environmental changes** drive adaptation
- **Support for 10,000+ agents** at 60 FPS
