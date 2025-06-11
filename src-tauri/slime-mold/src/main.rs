use slime_mold::app::App;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for performance test mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--buffer-pool-test" {
        return run_buffer_pool_performance_test();
    }

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create event loop and app
    let event_loop = EventLoop::new()?;
    let mut app = App::new();

    // Run the event loop
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn run_buffer_pool_performance_test() -> Result<(), Box<dyn std::error::Error>> {
    use slime_mold::buffer_pool::BufferPool;
    use std::time::Instant;
    use wgpu::{BufferUsages, Instance};

    println!("🧪 Running Buffer Pool Performance Test");
    println!("======================================");

    // Setup GPU device for testing
    let (device, _queue) = pollster::block_on(async {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("No compatible adapter found");

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    memory_hints: wgpu::MemoryHints::default(),
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device")
    });

    let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
    let iterations = 1000;
    let buffer_size = 1024 * 1024; // 1MB buffers

    // Test 1: Direct allocation performance
    println!("\n📊 Test 1: Direct Buffer Allocation");
    let start = Instant::now();
    for _ in 0..iterations {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test Buffer"),
            size: buffer_size,
            usage,
            mapped_at_creation: false,
        });
        std::hint::black_box(buffer); // Prevent optimization, buffer is dropped here
    }
    let direct_duration = start.elapsed();
    println!("   Time: {:?} ({:.2} μs per allocation)", direct_duration, direct_duration.as_micros() as f64 / iterations as f64);

    // Test 2: Pooled allocation performance (first-time, no reuse)
    println!("\n📊 Test 2: Pooled Buffer Allocation (Cold Pool)");
    let start = Instant::now();
    {
        let mut pool = BufferPool::new();
        for _ in 0..iterations {
            let buffer = pool.get_buffer(&device, Some("Pooled Buffer"), buffer_size, usage);
            pool.return_buffer(buffer, buffer_size, usage);
        }
    }
    let pooled_cold_duration = start.elapsed();
    println!("   Time: {:?} ({:.2} μs per allocation)", pooled_cold_duration, pooled_cold_duration.as_micros() as f64 / iterations as f64);

    // Test 3: Pooled allocation with reuse (warm pool)
    println!("\n📊 Test 3: Pooled Buffer Allocation (Warm Pool)");
    let mut pool = BufferPool::new();
    
    // Pre-populate the pool
    let mut buffers = Vec::new();
    for _ in 0..10 {
        let buffer = pool.get_buffer(&device, Some("Warm Buffer"), buffer_size, usage);
        buffers.push((buffer, buffer_size));
    }
    for (buffer, size) in buffers {
        pool.return_buffer(buffer, size, usage);
    }
    
    let start = Instant::now();
    for _ in 0..iterations {
        let buffer = pool.get_buffer(&device, Some("Warm Buffer"), buffer_size, usage);
        pool.return_buffer(buffer, buffer_size, usage);
    }
    let pooled_warm_duration = start.elapsed();
    println!("   Time: {:?} ({:.2} μs per allocation)", pooled_warm_duration, pooled_warm_duration.as_micros() as f64 / iterations as f64);

    // Test 4: Resize simulation pattern
    println!("\n📊 Test 4: Resize Pattern Simulation");
    let resize_pattern = [
        (10_000, 800, 600),   // Initial state
        (15_000, 800, 600),   // Increase agents
        (15_000, 1200, 800),  // Increase window size
        (8_000, 1200, 800),   // Decrease agents
        (8_000, 800, 600),    // Decrease window size
        (20_000, 1600, 900),  // Large increase
    ];

    // Direct allocation pattern
    let start = Instant::now();
    for _ in 0..100 { // Repeat the pattern 100 times
        for &(agent_count, width, height) in &resize_pattern {
            let agent_size = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
            let trail_size = (width * height * std::mem::size_of::<u32>()) as u64;

            let agent_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Agent Buffer"),
                size: agent_size,
                usage,
                mapped_at_creation: false,
            });

            let trail_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Trail Buffer"),
                size: trail_size,
                usage,
                mapped_at_creation: false,
            });

            std::hint::black_box((&agent_buffer, &trail_buffer));
        }
    }
    let direct_resize_duration = start.elapsed();
    println!("   Direct resize pattern: {:?}", direct_resize_duration);

    // Pooled allocation pattern
    let start = Instant::now();
    {
        let mut pool = BufferPool::new();
        for _ in 0..100 { // Repeat the pattern 100 times
            let mut old_buffers = Vec::new();
            
            for &(agent_count, width, height) in &resize_pattern {
                let agent_size = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
                let trail_size = (width * height * std::mem::size_of::<u32>()) as u64;

                // Return old buffers
                for (buffer, size) in old_buffers.drain(..) {
                    pool.return_buffer(buffer, size, usage);
                }

                // Get new buffers
                let agent_buffer = pool.get_buffer(&device, Some("Agent Buffer"), agent_size, usage);
                let trail_buffer = pool.get_buffer(&device, Some("Trail Buffer"), trail_size, usage);

                std::hint::black_box((&agent_buffer, &trail_buffer));

                old_buffers.push((agent_buffer, agent_size));
                old_buffers.push((trail_buffer, trail_size));
            }

            // Return final buffers
            for (buffer, size) in old_buffers {
                pool.return_buffer(buffer, size, usage);
            }
        }
    }
    let pooled_resize_duration = start.elapsed();
    println!("   Pooled resize pattern: {:?}", pooled_resize_duration);

    // Print summary
    println!("\n🎯 Performance Summary");
    println!("=====================");
    println!("Direct allocation:     {:.2} μs per operation", direct_duration.as_micros() as f64 / iterations as f64);
    println!("Pooled (cold):         {:.2} μs per operation", pooled_cold_duration.as_micros() as f64 / iterations as f64);
    println!("Pooled (warm):         {:.2} μs per operation", pooled_warm_duration.as_micros() as f64 / iterations as f64);
    
    let direct_resize_per_op = direct_resize_duration.as_micros() as f64 / (100.0 * resize_pattern.len() as f64);
    let pooled_resize_per_op = pooled_resize_duration.as_micros() as f64 / (100.0 * resize_pattern.len() as f64);
    
    println!("Direct resize pattern: {:.2} μs per resize operation", direct_resize_per_op);
    println!("Pooled resize pattern: {:.2} μs per resize operation", pooled_resize_per_op);

    // Calculate improvements
    if pooled_warm_duration < direct_duration {
        let improvement = (direct_duration.as_micros() as f64 / pooled_warm_duration.as_micros() as f64 - 1.0) * 100.0;
        println!("\n✨ Buffer pool shows {:.1}% performance improvement for warm allocations!", improvement);
    }

    if pooled_resize_duration < direct_resize_duration {
        let improvement = (direct_resize_duration.as_micros() as f64 / pooled_resize_duration.as_micros() as f64 - 1.0) * 100.0;
        println!("✨ Buffer pool shows {:.1}% performance improvement for resize operations!", improvement);
    }

    Ok(())
}
