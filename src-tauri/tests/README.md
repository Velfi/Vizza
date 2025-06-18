# Snapshot Testing for Simulations

This directory contains snapshot tests for the simulation systems. These tests run the simulations without the Tauri app for a specified number of iterations and capture screenshots to ensure visual consistency.

## Running Tests

To run the snapshot tests:

```bash
cd src-tauri
cargo test --test snapshot_tests -- --test-threads=1
```

Note: We use `--test-threads=1` because GPU operations may not work well with parallel test execution.

## How It Works

1. Each test initializes a GPU context without a window/surface
2. Creates a simulation with default settings
3. Runs the simulation for 1000 iterations (configurable)
4. Captures the final rendered frame
5. Saves/compares the image against a reference snapshot

## Managing Snapshots

### First Run

When you run tests for the first time, the tests will fail and create new snapshot images in:
```
src-tauri/tests/snapshots/images/
```

Run the tests again to verify they pass with the saved snapshots.

### Updating Snapshots

If you intentionally change the rendering and need to update snapshots:

1. Delete the old snapshot image(s) in `src-tauri/tests/snapshots/images/`
2. Run the tests to generate new snapshots
3. Verify the new snapshots look correct
4. Commit the new snapshot images to version control

### Image Comparison

The snapshot tests use pixel-by-pixel comparison with a small tolerance (5 color values per channel) to account for minor GPU rendering differences across different hardware. The tests will fail if more than 0.1% of pixels differ beyond the tolerance.

## Test Configuration

The `SnapshotTestConfig` struct allows you to configure:
- `width`: Width of the rendered image (default: 800)
- `height`: Height of the rendered image (default: 600)
- `iterations`: Number of simulation steps before capturing (default: 1000)

## Adding New Tests

To add a new snapshot test:

1. Create a new test function in `snapshot_tests.rs`
2. Use `run_simulation_snapshot` with your simulation setup
3. Use `compare_image_snapshot` with a descriptive name
4. Run the test twice - first to create the snapshot, second to verify

Example:
```rust
#[tokio::test]
async fn test_my_simulation() {
    let config = SnapshotTestConfig::default();
    let lut_manager = LutManager::new();
    
    let raw_data = run_simulation_snapshot(&config, |device, queue, surface_config, adapter_info| {
        // Create your simulation here
        MySimulation::new(device, queue, surface_config, ...)
    })
    .await
    .expect("Failed to run simulation");
    
    let img = bgra_to_image(&raw_data, config.width, config.height);
    compare_image_snapshot("my_simulation_snapshot", &img);
}
```

## CI Integration

For CI/CD pipelines:

```bash
# Run tests
cargo test --test snapshot_tests -- --test-threads=1
```

Make sure to commit the snapshot files in `src-tauri/tests/snapshots/images/` to your repository. The CI will compare against these committed snapshots.

## Troubleshooting

### GPU Not Available
If running in a headless environment, ensure software rendering is available:
```bash
export WGPU_BACKEND=gl
export LIBGL_ALWAYS_SOFTWARE=1
```

### Snapshot Differences
If tests fail due to minor rendering differences:
1. Check if the differences are visually significant
2. Adjust the tolerance in `compare_image_snapshot` if needed
3. Consider environmental factors (different GPU, drivers, etc.)