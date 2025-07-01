<script lang="ts">
  import { onDestroy } from 'svelte';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let showUI = false;
  export let autoHideDelay = 5000; // 5 seconds

  let isVisible = false;
  let hideTimeout: number | null = null;
  let lastInteractionTime = Date.now();
  let wasAutoHidden = false; // Flag to prevent re-showing after auto-hide
  let progressPercent = 100; // Progress bar percentage
  let progressInterval: number | null = null; // Interval for progress updates

  // Set up event listeners immediately
  const events = ['mousedown', 'mousemove', 'keydown', 'wheel', 'touchstart'];
  
  const handleEvent = () => {
    lastInteractionTime = Date.now();
    wasAutoHidden = false; // Reset auto-hidden flag
    if (isVisible) {
      startHideTimer();
      startProgressTimer(); // Restart progress timer
    }
  };

  // Add event listeners
  events.forEach(event => {
    document.addEventListener(event, handleEvent, { passive: true });
  });

  // Function to start the progress bar timer
  function startProgressTimer() {
    if (progressInterval) {
      clearInterval(progressInterval);
    }
    
    const startTime = Date.now();
    
    progressInterval = window.setInterval(() => {
      const currentTime = Date.now();
      const elapsed = currentTime - startTime;
      const remaining = Math.max(0, autoHideDelay - elapsed);
      
      progressPercent = (remaining / autoHideDelay) * 100;
      
      if (remaining <= 0) {
        stopProgressTimer();
      }
    }, 50); // Update every 50ms for smooth animation
  }

  // Function to stop the progress bar timer
  function stopProgressTimer() {
    if (progressInterval) {
      clearInterval(progressInterval);
      progressInterval = null;
    }
    progressPercent = 100;
  }

  // Function to show the indicator
  function showIndicator() {
    isVisible = true;
    wasAutoHidden = false; // Reset auto-hidden flag
    startHideTimer();
    startProgressTimer();
  }

  // Function to hide the indicator
  function hideIndicator() {
    isVisible = false;
    wasAutoHidden = true; // Mark as auto-hidden
    stopProgressTimer();
    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  }

  // Function to start the auto-hide timer
  function startHideTimer() {
    if (hideTimeout) {
      clearTimeout(hideTimeout);
    }
    
    hideTimeout = window.setTimeout(() => {
      // Only hide if no interaction in the last 5 seconds
      const timeSinceLastInteraction = Date.now() - lastInteractionTime;
      if (timeSinceLastInteraction >= autoHideDelay) {
        hideIndicator();
      } else {
        // Restart timer for remaining time
        const remainingTime = autoHideDelay - timeSinceLastInteraction;
        hideTimeout = window.setTimeout(hideIndicator, remainingTime);
      }
    }, autoHideDelay);
  }

  // Function to handle user interaction
  function handleInteraction() {
    lastInteractionTime = Date.now();
    wasAutoHidden = false; // Reset auto-hidden flag on user interaction
    if (isVisible) {
      startHideTimer();
      startProgressTimer(); // Restart progress timer
    }
  }

  // Function to toggle UI
  function toggleUI() {
    handleInteraction();
    dispatch('toggle');
  }

  // Watch for showUI changes
  $: {
    if (!showUI && !isVisible && !wasAutoHidden) {
      showIndicator();
    } else if (showUI && isVisible) {
      hideIndicator();
    }
  }

  // Initialize: if UI is hidden when component mounts, show indicator
  if (!showUI) {
    showIndicator();
  }

  onDestroy(() => {
    // Remove event listeners
    events.forEach(event => {
      document.removeEventListener(event, handleEvent);
    });
    
    if (hideTimeout) {
      clearTimeout(hideTimeout);
    }
    
    if (progressInterval) {
      clearInterval(progressInterval);
    }
  });
</script>

<div class="ui-hidden-indicator" class:visible={isVisible} on:click={handleInteraction} on:mouseenter={handleInteraction}>
  <div class="ui-hidden-content">
    <span>UI Hidden - Press <kbd>/</kbd> to toggle</span>
    <button class="ui-toggle-button" on:click={toggleUI}>Show UI</button>
  </div>
  <div class="progress-bar">
    <div class="progress-fill" style="width: {progressPercent}%"></div>
  </div>
</div>

<style>
  /* UI Hidden Indicator Styles */
  .ui-hidden-indicator {
    position: fixed;
    top: 10px;
    right: 10px;
    background: rgba(0, 0, 0, 0.8);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    color: white;
    z-index: 1000;
    backdrop-filter: blur(4px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    transition: opacity 0.3s ease, transform 0.3s ease;
    cursor: pointer;
    opacity: 0;
    pointer-events: none;
  }

  .ui-hidden-indicator.visible {
    opacity: 1;
    pointer-events: auto;
  }

  .ui-hidden-indicator:hover {
    background: rgba(0, 0, 0, 0.8);
    transform: translateY(-1px);
  }

  .ui-hidden-content {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.9rem;
  }

  .ui-hidden-content kbd {
    background: rgba(255, 255, 255, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.4);
    border-radius: 3px;
    padding: 0.2rem 0.4rem;
    font-family: monospace;
    font-size: 0.8rem;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .ui-toggle-button {
    padding: 0.4rem 0.8rem;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 4px;
    color: white;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s ease;
  }

  .ui-toggle-button:hover {
    background: rgba(255, 255, 255, 0.25);
    border-color: rgba(255, 255, 255, 0.5);
    transform: translateY(-1px);
  }

  .ui-toggle-button:active {
    transform: translateY(0);
  }

  /* Progress Bar Styles */
  .progress-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 0 0 6px 6px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, rgba(255, 255, 255, 0.6), rgba(255, 255, 255, 0.3));
    transition: width 0.1s ease;
    border-radius: 0 0 6px 6px;
  }

  /* Responsive design for UI hidden indicator */
  @media (max-width: 600px) {
    .ui-hidden-indicator {
      top: 5px;
      right: 5px;
      padding: 0.5rem 0.75rem;
    }
    
    .ui-hidden-content {
      font-size: 0.8rem;
      gap: 0.5rem;
    }
    
    .ui-toggle-button {
      padding: 0.3rem 0.6rem;
      font-size: 0.8rem;
    }
  }
</style> 