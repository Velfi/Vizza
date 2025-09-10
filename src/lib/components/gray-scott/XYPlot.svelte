<div class="xy-plot-container" bind:this={container}>
    <h3 class="plot-title">{title}</h3>
    <canvas
        bind:this={canvas}
        class="xy-plot"
        on:pointerdown={handlePointerDown}
        on:pointermove={handlePointerMove}
        on:pointerup={handlePointerUp}
        aria-label="Interactive XY plot with draggable parameter handle"
    >
    </canvas>
</div>

<script lang="ts">
    import { createEventDispatcher, onMount } from 'svelte';

    export let xValue: number = 0;
    export let yValue: number = 0;
    export let xRange: { min: number; max: number } = { min: 0, max: 1 };
    export let yRange: { min: number; max: number } = { min: 0, max: 1 };
    export let xLabel: string = 'X';
    export let yLabel: string = 'Y';
    export let title: string = 'XY Plot';
    export let handleColor: string = '#ef4444';
    export let handleStrokeColor: string = '#dc2626';
    export let valueLabelX: string = 'X';
    export let valueLabelY: string = 'Y';
    export let width: number = 400;
    export let height: number = 300;
    export let margin: number = 40;

    const dispatch = createEventDispatcher();

    // Canvas and context
    let canvas: HTMLCanvasElement;
    let ctx: CanvasRenderingContext2D;
    let container: HTMLDivElement;

    // Dragging state
    let isDragging = false;
    let lastUpdateTime = 0;
    const updateThrottle = 50; // ms between updates

    // Internal state
    let internalXValue = xValue;
    let internalYValue = yValue;
    const lastInternalUpdate = { x: 0, y: 0 };

    // Handle position
    const handle = { x: 0, y: 0 };

    // Plot dimensions
    let plotSize: number;
    let plotX: number;
    let plotY: number;

    function updateLayout() {
        plotSize = Math.min(width - 2 * margin, height - 2 * margin);
        plotX = (width - plotSize) / 2;
        plotY = margin;

        if (canvas) {
            canvas.width = width;
            canvas.height = height;
            draw();
        }
    }

    // Convert coordinates for XY plot
    function toCanvasX(value: number): number {
        const normalized = (value - xRange.min) / (xRange.max - xRange.min);
        return plotX + normalized * plotSize;
    }

    function toCanvasY(value: number): number {
        const normalized = (value - yRange.min) / (yRange.max - yRange.min);
        return plotY + plotSize - normalized * plotSize;
    }

    function fromCanvasX(x: number): number {
        const normalized = (x - plotX) / plotSize;
        return xRange.min + normalized * (xRange.max - xRange.min);
    }

    function fromCanvasY(y: number): number {
        const normalized = (plotY + plotSize - y) / plotSize;
        return yRange.min + normalized * (yRange.max - yRange.min);
    }

    // Update handle position
    function updateHandlePosition() {
        handle.x = toCanvasX(internalXValue);
        handle.y = toCanvasY(internalYValue);
    }

    // Draw the plot
    function draw() {
        if (!ctx) return;

        // Clear canvas
        ctx.clearRect(0, 0, width, height);

        // Update handle position
        updateHandlePosition();

        // Draw background
        ctx.fillStyle = '#1f2937';
        ctx.fillRect(plotX - 10, plotY - 30, plotSize + 20, plotSize + 60);
        ctx.strokeStyle = '#374151';
        ctx.lineWidth = 1;
        ctx.strokeRect(plotX - 10, plotY - 30, plotSize + 20, plotSize + 60);

        // Grid
        ctx.strokeStyle = '#374151';
        ctx.lineWidth = 1;
        ctx.setLineDash([2, 2]);

        // Vertical grid lines
        for (let i = 1; i < 10; i++) {
            const x = plotX + (i / 10) * plotSize;
            ctx.beginPath();
            ctx.moveTo(x, plotY);
            ctx.lineTo(x, plotY + plotSize);
            ctx.stroke();
        }

        // Horizontal grid lines
        for (let i = 1; i < 10; i++) {
            const gridY = plotY + (i / 10) * plotSize;
            ctx.beginPath();
            ctx.moveTo(plotX, gridY);
            ctx.lineTo(plotX + plotSize, gridY);
            ctx.stroke();
        }
        ctx.setLineDash([]);

        // Plot border
        ctx.strokeStyle = '#4b5563';
        ctx.lineWidth = 2;
        ctx.strokeRect(plotX, plotY, plotSize, plotSize);

        // Axis labels
        ctx.fillStyle = '#9ca3af';
        ctx.font = '12px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(xLabel, plotX + plotSize / 2, plotY + plotSize + 20);

        ctx.save();
        ctx.translate(plotX - 15, plotY + plotSize / 2);
        ctx.rotate(-Math.PI / 2);
        ctx.fillText(yLabel, 0, 0);
        ctx.restore();

        // Range labels
        ctx.fillText(xRange.min.toFixed(2), plotX, plotY + plotSize + 20);
        ctx.fillText(xRange.max.toFixed(2), plotX + plotSize, plotY + plotSize + 20);
        ctx.fillText(yRange.min.toFixed(2), plotX - 5, plotY + plotSize);
        ctx.fillText(yRange.max.toFixed(2), plotX - 5, plotY);

        // Handle
        const handleRadius = 8;
        ctx.fillStyle = handleColor;
        ctx.strokeStyle = handleStrokeColor;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(handle.x, handle.y, handleRadius, 0, 2 * Math.PI);
        ctx.fill();
        ctx.stroke();

        // Value labels
        ctx.fillStyle = '#fbbf24';
        ctx.font = '12px monospace';
        ctx.textAlign = 'center';
        ctx.fillText(`${valueLabelX}: ${internalXValue.toFixed(3)}`, handle.x, handle.y - 15);
        ctx.fillText(`${valueLabelY}: ${internalYValue.toFixed(3)}`, handle.x, handle.y + 25);
    }

    // Check if point is near handle
    function isNearHandle(x: number, y: number): boolean {
        const distance = Math.sqrt((x - handle.x) ** 2 + (y - handle.y) ** 2);
        return distance <= 12;
    }

    // Check if point is in plot area
    function isInPlot(x: number, y: number): boolean {
        return x >= plotX && x <= plotX + plotSize && y >= plotY && y <= plotY + plotSize;
    }

    // Handle mouse/touch events
    function handlePointerDown(event: PointerEvent) {
        event.preventDefault();

        const rect = canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        if (isNearHandle(x, y) || isInPlot(x, y)) {
            isDragging = true;
            lastUpdateTime = Date.now();
            canvas.setPointerCapture(event.pointerId);
        }
    }

    function handlePointerMove(event: PointerEvent) {
        if (!isDragging) return;

        const rect = canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        // Throttle updates to prevent feedback loops
        const now = Date.now();
        if (now - lastUpdateTime < updateThrottle) {
            return;
        }
        lastUpdateTime = now;

        const newXValue = Math.max(xRange.min, Math.min(xRange.max, fromCanvasX(x)));
        const newYValue = Math.max(yRange.min, Math.min(yRange.max, fromCanvasY(y)));

        internalXValue = newXValue;
        internalYValue = newYValue;
        lastInternalUpdate.x = newXValue;
        lastInternalUpdate.y = newYValue;

        dispatch('update', { x: newXValue, y: newYValue });
        draw();
    }

    function handlePointerUp(event: PointerEvent) {
        if (isDragging) {
            isDragging = false;
            canvas.releasePointerCapture(event.pointerId);
        }
    }

    // Sync with external props when they change
    $: if (
        xValue !== internalXValue &&
        !isDragging &&
        Math.abs(xValue - lastInternalUpdate.x) > 0.0001
    ) {
        internalXValue = xValue;
        draw();
    }

    $: if (
        yValue !== internalYValue &&
        !isDragging &&
        Math.abs(yValue - lastInternalUpdate.y) > 0.0001
    ) {
        internalYValue = yValue;
        draw();
    }

    // Handle resize
    function handleResize() {
        if (container) {
            width = Math.max(320, container.offsetWidth);
            updateLayout();
        }
    }

    // Initialize on mount
    onMount(() => {
        ctx = canvas.getContext('2d')!;

        // Initialize internal values from props
        internalXValue = xValue;
        internalYValue = yValue;

        // Set up resize observer
        const resizeObserver = new ResizeObserver(() => {
            handleResize();
        });

        if (container) {
            resizeObserver.observe(container);
            handleResize();
        }

        return () => {
            resizeObserver.disconnect();
        };
    });
</script>

<style>
    .xy-plot-container {
        width: 100%;
        max-width: 100%;
        margin: 0;
        padding: 0;
    }

    .plot-title {
        margin: 0 0 0.5rem 0;
        color: #ffffff;
        font-size: 1rem;
        font-weight: 500;
        text-align: center;
    }

    .xy-plot {
        border: 1px solid #374151;
        border-radius: 0.5rem;
        background: #1a1a1a;
        display: block;
        width: 100%;
        height: auto;
        max-width: 100%;
    }
</style>
