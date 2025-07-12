# Microbial Ecosystem Simulation - Design Document

## Core Philosophy

This simulation prioritizes **emergent complexity from simple rules** over scientific accuracy. The goal is to create a living, breathing microscopic world where complex behaviors arise naturally from basic agent interactions and environmental physics. Players should feel like they're observing a real ecosystem, not a programmed system.

**Design Principles:**

- **Realistic Movement**: Agents move like real microorganisms using chemotaxis and Brownian motion
- **Matter Conservation**: Nothing appears or disappears - all matter transforms between states
- **Emergent Behavior**: Complex patterns arise from simple individual rules, not top-down programming
- **Observable Causality**: Players can trace cause-and-effect relationships through the system
- **Biological Inspiration**: Loosely based on real microorganisms but simplified for gameplay

## Agent Categories & Variants

The ecosystem uses three fundamental ecological roles, each with multiple variants that provide visual diversity and behavioral nuance without requiring players to understand real microbiology.

### Recyclers

**Ecological Role**: Break down dead matter and waste into usable nutrients, preventing system stagnation

**Core Behavior**: Detect chemical gradients from decomposing matter, move toward sources, consume particles on contact, release nutrients into environment

**Variants**:

- **Bacteria**: Small, fast-moving, swarm behavior around food sources
- **Fungi**: Slower movement, create visible thread networks, specialize in complex matter breakdown
- **Decomposer Protozoans**: Larger, engulf debris particles whole, more selective feeding

**Design Intent**: These agents prevent the ecosystem from choking on its own waste while creating resource hotspots that drive spatial organization.

### Producers

**Ecological Role**: Convert raw nutrients and light energy into biomass, forming the foundation of the food web

**Core Behavior**: Absorb chemicals from environment grid, photosynthesize more efficiently in bright areas, grow and reproduce when well-fed

**Variants**:

- **Algae**: Form biofilm mats, highly light-dependent, create persistent structures
- **Cyanobacteria**: Mobile colonies, moderate light needs, can fix nitrogen from environment
- **Photosynthetic Protists**: Larger individual organisms, complex movement patterns, efficient nutrient use

**Design Intent**: These agents create the primary energy input to the system and establish spatial patterns through their light-seeking behavior.

### Predators

**Ecological Role**: Control population dynamics through consumption, prevent any single species from dominating

**Core Behavior**: Use chemical sensors to detect prey, chase using directed movement, consume on contact based on size relationships

**Variants**:

- **Predatory Bacteria**: Small, fast, hunt in coordinated groups, target other bacteria
- **Viruses**: Inject into hosts, replicate internally, burst out after delay, highly specialized
- **Predatory Protozoans**: Large, engulf smaller organisms, slow but powerful
- **Parasitic Microbes**: Attach to hosts, drain resources over time, create mobile infections

**Design Intent**: These agents create population pressure and spatial heterogeneity, forcing prey to develop survival strategies.

## Environmental Systems

### Unified Nutrient Architecture

The simulation uses a two-layer nutrient system that balances visual clarity with realistic physics.

**Particle Layer (Visible)**:

- Dead matter appears as discrete particles when organisms die
- Waste pellets produced by living organisms
- Particles have size, chemical signature, and spatial location
- Players can directly observe food sources and their distribution

**Chemical Layer (Invisible)**:

- Concentration gradients stored in environment grid
- Particles continuously leak chemicals into surrounding cells
- Producers absorb chemicals directly from grid
- Creates realistic diffusion and flow patterns

**Conversion Process**:

- Dead matter particles gradually decompose, releasing chemicals
- Particle shrinks as it leaks, eventually disappearing when fully converted
- Rate depends on environmental conditions (temperature, pH, recycler activity)
- Maintains matter conservation while enabling natural cleanup

### Fluid Dynamics System

Rather than simple diffusion, nutrients flow through the environment using simplified fluid physics that create realistic mixing patterns.

**Current Generation**:

- **Biological Sources**: Agent movement creates micro-currents, biofilm growth displaces fluid
- **Physical Sources**: Density gradients from nutrient concentration differences
- **Chemical Sources**: Osmotic pressure from chemical gradients, pH variations

**Flow Mechanics**:

- Simple velocity field updated using pressure-based approximation
- Nutrients advect with flow using standard Euler integration
- Diffusion combined with advection creates natural mixing
- Biofilm structures act as flow obstacles, creating downstream effects

**Performance Optimization**:

- Coarse fluid grid with interpolation for fine nutrient resolution
- Update frequency scaling based on activity levels
- Precomputed flow patterns for common configurations

### Light Gradient System

Constant angular gradient simulates sunlight direction while remaining visually clear and predictable.

**Implementation**:

- Light intensity varies smoothly across environment
- Brightest at sun edge, darkest at opposite edge
- Intensity = base_light + (gradient_strength × cos(angle_from_sun))
- Gradient direction rotates slowly over time (day/night cycle)

**Behavioral Effects**:

- Producers naturally concentrate toward bright areas
- Creates persistent ecosystem structure without complex programming
- Slow rotation drives large-scale migration patterns
- Predators follow prey toward light, creating predictable hunting zones

## Movement & Sensing Systems

### Realistic Microbial Movement

Agents move using authentic microorganism locomotion patterns rather than game-like pathfinding.

**Core Mechanics**:

- **Brownian Motion**: Constant random jittering provides baseline movement
- **Chemotaxis**: Bias random movement toward favorable chemical gradients
- **Run-and-Tumble**: Move straight for random duration, then randomly change direction
- **Flagella Simulation**: Forward thrust with rotational drag creates swimming behavior

**Sensing Implementation**:

- Each agent has 3-4 chemical receptors pointing different directions
- Compare receptor strengths to determine movement bias
- Temporal sampling allows gradient following using recent sensor history
- Receptor saturation prevents perfect navigation in strong chemical fields

**Emergent Behaviors**:

- Natural swarming around food sources
- Realistic search patterns when resources are sparse
- Organic-looking movement paths rather than straight lines
- Confusion and dispersion in areas with mixed chemical signals

### Hunting Mechanics

Predation uses realistic pursuit and capture mechanics rather than abstract health systems.

**Target Acquisition**:

- Predators detect prey through chemical signatures
- Different agent types release different chemical profiles
- Sensor range and sensitivity vary by predator variant
- Preference hierarchies target easier prey first

**Attack Resolution**:

- Size-based success: predator must be significant fraction of prey size
- Contact required: physical overlap triggers attack attempt
- Environmental factors affect success (biofilm cover, toxin levels)
- Failed attacks cost energy and trigger prey escape responses

**Pack Hunting**:

- Multiple predators can attack same prey if size threshold requires it
- All participants share biomass reward equally
- Coordination bonus for simultaneous attacks
- Diminishing returns prevent excessive reward splitting

## Emergent Behaviors

### Population Dynamics

Complex population cycles emerge from simple individual interactions without explicit programming.

**Predator-Prey Oscillations**: Classic boom-bust cycles as predators respond to prey availability with reproduction lag

**Resource Competition**: Producers compete for light and nutrients, creating spatial exclusion zones

**Carrying Capacity**: System naturally regulates population based on resource availability and waste accumulation

**Succession Patterns**: Different agent types dominate at different times based on environmental conditions

### Spatial Organization

The environment self-organizes into distinct zones and patterns through agent behavior.

**Biofilm Colonies**: Producers create persistent structures that modify local flow and chemistry

**Nutrient Streams**: Fluid dynamics create flowing channels that concentrate resources

**Hunting Territories**: Predators establish patrol routes around prey concentrations

**Dead Zones**: Areas with poor circulation or high toxin levels become uninhabitable

### Adaptive Responses

Agents develop survival strategies through simple behavioral rules interacting with environmental pressure.

**Predator Avoidance**: Prey naturally avoid areas with high predator activity

**Resource Tracking**: Agents follow chemical gradients to locate food sources

**Seasonal Migration**: Light gradient rotation drives ecosystem-wide movement patterns

**Niche Specialization**: Different variants excel in different environmental conditions

## Player Interaction

### Observation Tools

Players act as scientists studying the ecosystem rather than controlling it directly.

**Multi-Scale Viewing**:

- Zoom from individual organism behavior to ecosystem-wide patterns
- Track specific agents through their lifecycle
- Time-lapse functionality to observe long-term changes

**Data Visualization**:

- Population graphs by category and variant
- Chemical concentration heatmaps
- Flow streamlines showing nutrient movement
- Statistical analysis of ecosystem health

**Overlay Systems**:

- Toggle between particle view and chemical gradients
- Highlight specific agent types or behaviors
- Show interaction networks and food web relationships

### Environmental Manipulation

Players can influence the ecosystem through realistic environmental changes.

**Light Control**:

- Adjust gradient intensity and rotation speed
- Change sun direction to trigger migration
- Create day/night cycles of varying length

**Chemical Introduction**:

- Add nutrients to specific locations
- Introduce toxins to create disturbances
- Adjust pH or salinity to favor certain organisms

**Physical Disruption**:

- Introduce new biofilm structures
- Create flow obstacles or channels
- Disturb established colonies

### Experimental Design

The system supports scientific-style experimentation and hypothesis testing.

**Controlled Variables**: Isolate specific factors to test their effects

**Replication**: Run multiple trials with identical starting conditions

**Data Collection**: Export population and environmental data for analysis

**Hypothesis Testing**: Predict outcomes and observe results

## Technical Implementation

### Agent Architecture

Each agent uses a simple state machine with realistic sensory input processing.

**Core Systems**:

- **Sensor Array**: Chemical receptors provide environmental information
- **Movement Engine**: Brownian motion with chemotactic bias
- **Metabolic System**: Energy consumption, growth, and reproduction
- **Behavioral States**: Feeding, hunting, reproducing, escaping

**Performance Optimization**:

- Spatial partitioning for efficient neighbor queries
- Level-of-detail rendering for large populations
- Predictive caching for common calculations
- Batch processing for similar operations

### Environmental Simulation

The environment uses efficient grid-based systems with interpolation for smooth agent interaction.

**Grid Architecture**:

- Chemical concentrations stored in 2D grid cells
- Fluid velocity field updated using simplified Navier-Stokes
- Biofilm structures represented as persistent grid modifications
- Particle systems for visible matter (dead organisms, waste)

**Update Strategies**:

- Adaptive time-stepping based on system activity
- Spatial subdivision to skip inactive regions
- Asynchronous updates for non-critical systems
- Memory pooling for temporary objects

### Rendering Pipeline

Visual presentation emphasizes clarity and biological authenticity.

**Agent Rendering**:

- Variant-specific sprites with procedural animation
- Trail effects showing movement history and chemical following
- Dynamic sizing based on biomass and health
- Behavioral state indicators (hunting, feeding, reproducing)

**Environmental Visualization**:

- Smooth interpolation of chemical concentration fields
- Flow streamlines with particle advection
- Biofilm structures with organic growth patterns
- Lighting effects for photosynthesis visualization

## Success Metrics

### Ecosystem Health

- **Stability**: Population oscillations remain within sustainable bounds
- **Diversity**: Multiple agent variants coexist without any dominating completely
- **Resilience**: System recovers from environmental disturbances
- **Productivity**: Nutrient cycling maintains ecosystem energy flow

### Player Engagement

- **Predictability**: Players can anticipate system responses to their actions
- **Discovery**: New behaviors and patterns emerge through extended observation
- **Experimentation**: Changes produce measurable, understandable effects
- **Education**: Players develop intuitive understanding of ecological principles

### Technical Performance

- **Scalability**: System maintains performance with thousands of agents
- **Responsiveness**: Real-time interaction without perceptible lag
- **Stability**: No crashes or corruption during extended operation
- **Efficiency**: Reasonable resource usage on target hardware

## Scope and Limitations

### Included Features

- Realistic microbial movement and sensing
- Fluid dynamics for nutrient transport
- Emergent population dynamics
- Biofilm formation and persistence
- Multi-variant agent types
- Environmental manipulation tools

### Excluded Features

- Genetic algorithms or evolutionary computation
- Detailed biochemical simulation
- Species identification or taxonomic accuracy
- Educational content or scientific explanations
- Multiplayer or competitive elements
- Save/load ecosystem states

### Design Constraints

- Prioritize performance over biological accuracy
- Maintain visual clarity at all zoom levels
- Ensure predictable cause-and-effect relationships
- Keep individual agent rules simple and understandable
- Avoid systems that require extensive player micromanagement
