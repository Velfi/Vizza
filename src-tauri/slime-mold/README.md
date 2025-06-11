# Slime Mold

A GPU-accelerated slime mold simulation written in Rust using WGPU. This project simulates the emergent behavior of slime mold colonies using compute shaders. It supports millions of agents on modern hardware.

![Slime mold simulation example 1](./example_1.png)
![Slime mold simulation example 2](./example_2.png)
![Slime mold simulation example 3](./example_3.png)
![Slime mold simulation example 4](./example_4.png)
![Slime mold simulation example 5](./example_5.png)

## Features

- Real-time GPU-accelerated simulation using WGPU
- Interactive parameter customization for slime behavior
- Multiple presets for different simulation patterns
- Custom LUT (Look-Up Table) support for color visualization
- Configurable Gaussian blur for trail diffusion

## Quickstart

Run the simulation with `cargo`:

```sh
cargo run --release
```

## Key Bindings

- `R`: Randomize settings
- `/`: Toggle sidebar

## License

This project is open source and available under the MIT License.
