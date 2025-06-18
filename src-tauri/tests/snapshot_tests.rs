#[cfg(test)]
mod tests {
    use sim_pix::snapshot_test_runner::{
        bgra_to_image, run_simulation_snapshot, SnapshotTestConfig,
    };
    use sim_pix::simulations::gray_scott::{self, GrayScottModel};
    use sim_pix::simulations::particle_life::{self, ParticleLifeModel};
    use sim_pix::simulations::shared::LutManager;
    use sim_pix::simulations::slime_mold::{self, SlimeMoldModel};
    use std::path::PathBuf;
    use image::ImageFormat;

    fn get_snapshot_path(name: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("snapshots");
        path.push("images");
        std::fs::create_dir_all(&path).unwrap();
        path.push(format!("{}.png", name));
        path
    }

    fn compare_image_snapshot(name: &str, img: &image::RgbaImage) {
        let path = get_snapshot_path(name);
        
        // Check if reference snapshot exists
        if path.exists() {
            // Load reference image
            let reference = image::open(&path).unwrap().to_rgba8();
            
            // Compare dimensions
            assert_eq!(
                (reference.width(), reference.height()),
                (img.width(), img.height()),
                "Image dimensions mismatch for snapshot '{}'", name
            );
            
            // Compare pixels (with some tolerance for GPU rendering differences)
            let mut differences = 0;
            let tolerance = 5; // Allow small differences due to GPU variations
            
            for (p1, p2) in reference.pixels().zip(img.pixels()) {
                for i in 0..4 {
                    let diff = (p1[i] as i32 - p2[i] as i32).abs();
                    if diff > tolerance {
                        differences += 1;
                    }
                }
            }
            
            let total_components = (img.width() * img.height() * 4) as usize;
            let diff_percentage = (differences as f32 / total_components as f32) * 100.0;
            
            assert!(
                diff_percentage < 0.1, // Allow up to 0.1% pixel differences
                "Image snapshot '{}' differs by {:.2}% ({} components out of {})",
                name, diff_percentage, differences, total_components
            );
        } else {
            // Save new snapshot
            img.save_with_format(&path, ImageFormat::Png).unwrap();
            panic!(
                "New snapshot created at {:?}. Run the test again to compare against it.",
                path
            );
        }
    }

    #[tokio::test]
    async fn test_slime_mold_snapshot() {
        let config = SnapshotTestConfig::default();
        let lut_manager = LutManager::new();

        let raw_data = run_simulation_snapshot(&config, |device, queue, surface_config, adapter_info| {
            let settings = slime_mold::settings::Settings::default();
            let default_lut_name = "MATPLOTLIB_bone_r";

            SlimeMoldModel::new(
                device,
                queue,
                surface_config,
                adapter_info,
                1_000_000, // Reduced agent count for tests
                settings,
                &lut_manager,
                default_lut_name.to_owned(),
                false,
            )
        })
        .await
        .expect("Failed to run slime mold simulation");

        // Convert to image
        let img = bgra_to_image(&raw_data, config.width, config.height);
        
        // Compare with snapshot
        compare_image_snapshot("slime_mold_1000_iterations", &img);
    }

    #[tokio::test]
    async fn test_gray_scott_snapshot() {
        let config = SnapshotTestConfig::default();
        let lut_manager = LutManager::new();

        let raw_data = run_simulation_snapshot(&config, |device, queue, surface_config, _adapter_info| {
            let settings = gray_scott::settings::Settings::default();
            let default_lut_name = "MATPLOTLIB_prism";

            GrayScottModel::new(
                device,
                queue,
                surface_config,
                surface_config.width,
                surface_config.height,
                settings,
                &lut_manager,
                default_lut_name.to_owned(),
                false,
            )
        })
        .await
        .expect("Failed to run Gray-Scott simulation");

        // Convert to image
        let img = bgra_to_image(&raw_data, config.width, config.height);
        
        // Compare with snapshot
        compare_image_snapshot("gray_scott_1000_iterations", &img);
    }

    #[tokio::test]
    async fn test_particle_life_snapshot() {
        let config = SnapshotTestConfig::default();
        let lut_manager = LutManager::new();

        let raw_data = run_simulation_snapshot(&config, |device, queue, surface_config, _adapter_info| {
            let settings = particle_life::settings::Settings::default();
            let default_lut_name = "MATPLOTLIB_inferno";

            ParticleLifeModel::new(
                device,
                queue,
                surface_config,
                settings,
                &lut_manager,
                default_lut_name.to_owned(),
                false,
            )
        })
        .await
        .expect("Failed to run particle life simulation");

        // Convert to image
        let img = bgra_to_image(&raw_data, config.width, config.height);
        
        // Compare with snapshot
        compare_image_snapshot("particle_life_1000_iterations", &img);
    }

    #[tokio::test]
    async fn test_slime_mold_with_different_settings() {
        let mut config = SnapshotTestConfig::default();
        config.iterations = 500; // Test with fewer iterations
        let lut_manager = LutManager::new();

        let raw_data = run_simulation_snapshot(&config, |device, queue, surface_config, adapter_info| {
            let mut settings = slime_mold::settings::Settings::default();
            // Modify settings for testing
            settings.move_speed = 2.0;
            settings.sensor_distance = 15.0;
            let default_lut_name = "MATPLOTLIB_hot";

            SlimeMoldModel::new(
                device,
                queue,
                surface_config,
                adapter_info,
                500_000,
                settings,
                &lut_manager,
                default_lut_name.to_owned(),
                false,
            )
        })
        .await
        .expect("Failed to run slime mold simulation with custom settings");

        // Convert to image
        let img = bgra_to_image(&raw_data, config.width, config.height);
        
        // Compare with snapshot
        compare_image_snapshot("slime_mold_500_iterations_custom", &img);
    }

    #[test]
    fn test_snapshot_directory_creation() {
        let path = get_snapshot_path("test");
        assert!(path.parent().unwrap().exists());
    }
}