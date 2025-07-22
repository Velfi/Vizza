/// Matrix operations for the Particle Life simulation
/// This module contains all the matrix transformation operations that can be applied
/// to the force interaction matrix.
/// Scale all matrix values by a given factor
pub fn scale_force_matrix(force_matrix: &mut [Vec<f32>], scale_factor: f32) {
    for row in force_matrix {
        for element in row {
            *element = (*element * scale_factor).clamp(-1.0, 1.0);
        }
    }
}

/// Flip the matrix horizontally
pub fn flip_horizontal(force_matrix: &mut [Vec<f32>]) {
    let n = force_matrix.len();
    for row in force_matrix.iter_mut().take(n) {
        for j in 0..n / 2 {
            row.swap(j, n - 1 - j);
        }
    }
}

/// Flip the matrix vertically
pub fn flip_vertical(force_matrix: &mut [Vec<f32>]) {
    let n = force_matrix.len();
    for i in 0..n / 2 {
        for j in 0..n {
            let temp = force_matrix[i][j];
            force_matrix[i][j] = force_matrix[n - 1 - i][j];
            force_matrix[n - 1 - i][j] = temp;
        }
    }
}

/// Rotate the matrix clockwise (90 degrees)
#[allow(clippy::needless_range_loop)]
pub fn rotate_clockwise(force_matrix: &mut Vec<Vec<f32>>) {
    let n = force_matrix.len();
    let mut new_matrix = vec![vec![0.0; n]; n];

    // Rotate all elements according to the transformation
    for i in 0..n {
        for j in 0..n {
            new_matrix[j][n - 1 - i] = force_matrix[i][j];
        }
    }

    *force_matrix = new_matrix;
}

/// Rotate the matrix counterclockwise (90 degrees)
#[allow(clippy::needless_range_loop)]
pub fn rotate_counterclockwise(force_matrix: &mut Vec<Vec<f32>>) {
    let n = force_matrix.len();
    let mut new_matrix = vec![vec![0.0; n]; n];

    // Rotate all elements according to the transformation
    for i in 0..n {
        for j in 0..n {
            new_matrix[n - 1 - j][i] = force_matrix[i][j];
        }
    }

    *force_matrix = new_matrix;
}

/// Shift the force matrix left (circular shift of columns)
pub fn shift_left(force_matrix: &mut [Vec<f32>]) {
    let n = force_matrix.len();
    for row in force_matrix.iter_mut().take(n) {
        let temp = row[0];
        for j in 0..n - 1 {
            row[j] = row[j + 1];
        }
        row[n - 1] = temp;
    }
}

/// Shift the force matrix right (circular shift of columns)
pub fn shift_right(force_matrix: &mut [Vec<f32>]) {
    let n = force_matrix.len();
    for row in force_matrix.iter_mut().take(n) {
        let temp = row[n - 1];
        for j in (1..n).rev() {
            row[j] = row[j - 1];
        }
        row[0] = temp;
    }
}

/// Shift the force matrix up (circular shift of rows)
pub fn shift_up(force_matrix: &mut [Vec<f32>]) {
    let n = force_matrix.len();
    let temp_row = force_matrix[0].clone();
    for i in 0..n - 1 {
        force_matrix[i] = force_matrix[i + 1].clone();
    }
    force_matrix[n - 1] = temp_row;
}

/// Shift the force matrix down (circular shift of rows)
pub fn shift_down(force_matrix: &mut [Vec<f32>]) {
    let n = force_matrix.len();
    let temp_row = force_matrix[n - 1].clone();
    for i in (1..n).rev() {
        force_matrix[i] = force_matrix[i - 1].clone();
    }
    force_matrix[0] = temp_row;
}

/// Set all matrix values to zero
pub fn zero_matrix(force_matrix: &mut [Vec<f32>]) {
    for row in force_matrix {
        for element in row {
            *element = 0.0;
        }
    }
}

/// Flip the sign of all matrix values (multiply by -1)
pub fn flip_sign(force_matrix: &mut [Vec<f32>]) {
    for row in force_matrix {
        for element in row {
            *element = -*element;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a test matrix with known values
    fn create_test_matrix(size: usize) -> Vec<Vec<f32>> {
        let mut matrix = vec![vec![0.0; size]; size];

        // Fill with test values: diagonal = -0.1, others = small sequential values
        for (i, row) in matrix.iter_mut().enumerate().take(size) {
            for (j, cell) in row.iter_mut().enumerate().take(size) {
                if i == j {
                    *cell = -0.1; // Self-repulsion
                } else {
                    // Use smaller values that won't get clamped when scaled
                    *cell = (i * size + j) as f32 * 0.01; // Sequential values
                }
            }
        }

        matrix
    }

    /// Helper function to compare matrices with tolerance for floating point errors
    fn matrices_equal(a: &[Vec<f32>], b: &[Vec<f32>], tolerance: f32) -> bool {
        if a.len() != b.len() {
            return false;
        }

        for i in 0..a.len() {
            if a[i].len() != b[i].len() {
                return false;
            }

            for j in 0..a[i].len() {
                if (a[i][j] - b[i][j]).abs() > tolerance {
                    return false;
                }
            }
        }

        true
    }

    #[test]
    fn test_scale_force_matrix() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test scaling up
        scale_force_matrix(&mut matrix, 2.0);
        for i in 0..3 {
            for j in 0..3 {
                // All elements should be scaled
                assert!((matrix[i][j] - original_matrix[i][j] * 2.0).abs() < 0.001);
            }
        }

        // Test scaling down
        scale_force_matrix(&mut matrix, 0.5);
        for i in 0..3 {
            for j in 0..3 {
                // All elements should be scaled back down
                assert!((matrix[i][j] - original_matrix[i][j]).abs() < 0.001);
            }
        }

        // Test clamping behavior
        matrix[0][1] = 0.8;
        scale_force_matrix(&mut matrix, 2.0);
        assert!(matrix[0][1] <= 1.0); // Should be clamped to 1.0

        matrix[0][1] = -0.8;
        scale_force_matrix(&mut matrix, 2.0);
        assert!(matrix[0][1] >= -1.0); // Should be clamped to -1.0
    }

    #[test]
    fn test_flip_horizontal() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test horizontal flip
        flip_horizontal(&mut matrix);

        // Test that flipping twice returns to original
        flip_horizontal(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_flip_vertical() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test vertical flip
        flip_vertical(&mut matrix);

        // Test that flipping twice returns to original
        flip_vertical(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_rotate_clockwise() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test clockwise rotation
        rotate_clockwise(&mut matrix);

        // Check specific rotations for 3x3 matrix
        // Original matrix:
        // [-0.1, 0.01, 0.02]
        // [0.03, -0.1, 0.05]
        // [0.06, 0.07, -0.1]
        //
        // After clockwise rotation (new[j][n-1-i] = original[i][j]):
        // [0.06, 0.03, -0.1]
        // [0.07, -0.1, 0.01]
        // [-0.1, 0.05, 0.02]
        assert!((matrix[0][0] - 0.06).abs() < 0.001);
        assert!((matrix[0][1] - 0.03).abs() < 0.001);
        assert!((matrix[0][2] - (-0.1)).abs() < 0.001);
        assert!((matrix[1][0] - 0.07).abs() < 0.001);
        assert!((matrix[1][1] - (-0.1)).abs() < 0.001);
        assert!((matrix[1][2] - 0.01).abs() < 0.001);
        assert!((matrix[2][0] - (-0.1)).abs() < 0.001);
        assert!((matrix[2][1] - 0.05).abs() < 0.001);
        assert!((matrix[2][2] - 0.02).abs() < 0.001);

        // Test that rotating 4 times returns to original
        rotate_clockwise(&mut matrix);
        rotate_clockwise(&mut matrix);
        rotate_clockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_rotate_counterclockwise() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test counterclockwise rotation
        rotate_counterclockwise(&mut matrix);

        // Test that rotating 4 times returns to original
        rotate_counterclockwise(&mut matrix);
        rotate_counterclockwise(&mut matrix);
        rotate_counterclockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_rotate_clockwise_and_counterclockwise() {
        let mut matrix = create_test_matrix(4);
        let original_matrix = matrix.clone();

        // Test that clockwise + counterclockwise = identity
        rotate_clockwise(&mut matrix);
        rotate_counterclockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test that counterclockwise + clockwise = identity
        rotate_counterclockwise(&mut matrix);
        rotate_clockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_shift_left_and_right() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test left shift
        shift_left(&mut matrix);

        // Check that elements are shifted left (circular)
        // Original: [0][0]=-0.1, [0][1]=0.01, [0][2]=0.02
        // After left shift: [0][0]=0.01, [0][1]=0.02, [0][2]=-0.1
        assert!((matrix[0][0] - 0.01).abs() < 0.001);
        assert!((matrix[0][1] - 0.02).abs() < 0.001);
        assert!((matrix[0][2] - (-0.1)).abs() < 0.001);

        // Test that shifting left then right returns to original
        shift_right(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test that shifting right then left returns to original
        shift_right(&mut matrix);
        shift_left(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_shift_up_and_down() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test up shift
        shift_up(&mut matrix);

        // Check that elements are shifted up (circular)
        // Original row 0: [-0.1, 0.01, 0.02]
        // Original row 1: [0.03, -0.1, 0.05]
        // Original row 2: [0.06, 0.07, -0.1]
        // After up shift:
        // Row 0 should be [0.03, -0.1, 0.05] (was row 1)
        // Row 1 should be [0.06, 0.07, -0.1] (was row 2)
        // Row 2 should be [-0.1, 0.01, 0.02] (was row 0)
        assert!((matrix[0][0] - 0.03).abs() < 0.001);
        assert!((matrix[0][1] - (-0.1)).abs() < 0.001);
        assert!((matrix[0][2] - 0.05).abs() < 0.001);

        assert!((matrix[1][0] - 0.06).abs() < 0.001);
        assert!((matrix[1][1] - 0.07).abs() < 0.001);
        assert!((matrix[1][2] - (-0.1)).abs() < 0.001);

        assert!((matrix[2][0] - (-0.1)).abs() < 0.001);
        assert!((matrix[2][1] - 0.01).abs() < 0.001);
        assert!((matrix[2][2] - 0.02).abs() < 0.001);

        // Test that shifting up then down returns to original
        shift_down(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test that shifting down then up returns to original
        shift_down(&mut matrix);
        shift_up(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_complex_operation_sequences() {
        let mut matrix = create_test_matrix(4);
        let original_matrix = matrix.clone();

        // Test simpler sequence: scale -> scale back
        scale_force_matrix(&mut matrix, 2.0);
        scale_force_matrix(&mut matrix, 0.5);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test rotation sequence: rotate -> rotate back
        rotate_clockwise(&mut matrix);
        rotate_counterclockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test flip sequence: flip -> flip back
        flip_horizontal(&mut matrix);
        flip_horizontal(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_operation_edge_cases() {
        let mut matrix = create_test_matrix(2);
        let original_matrix = matrix.clone();

        // Test operations on 2x2 matrix (minimum size)
        rotate_clockwise(&mut matrix);
        rotate_counterclockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        flip_horizontal(&mut matrix);
        flip_horizontal(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        flip_vertical(&mut matrix);
        flip_vertical(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        shift_left(&mut matrix);
        shift_right(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        shift_up(&mut matrix);
        shift_down(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_scale_edge_cases() {
        let mut matrix = create_test_matrix(3);

        // Test scaling by 0 (should zero out all elements)
        scale_force_matrix(&mut matrix, 0.0);
        for row in &matrix {
            for &value in row {
                assert!((value - 0.0).abs() < 0.001);
            }
        }

        // Test scaling by negative value
        matrix = create_test_matrix(3);
        scale_force_matrix(&mut matrix, -1.0);
        for (i, row) in matrix.iter().enumerate().take(3) {
            for (j, &value) in row.iter().enumerate().take(3) {
                // All elements should be negated
                let expected = if i == j {
                    -(-0.1) // Diagonal elements were -0.1, so negated becomes 0.1
                } else {
                    -((i * 3 + j) as f32) * 0.01
                };
                assert!((value - expected).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_matrix_operations_with_different_sizes() {
        // Test operations on different matrix sizes
        for size in 2..=6 {
            let mut matrix = create_test_matrix(size);
            let original_matrix = matrix.clone();

            // Test rotation
            rotate_clockwise(&mut matrix);
            rotate_clockwise(&mut matrix);
            rotate_clockwise(&mut matrix);
            rotate_clockwise(&mut matrix);
            assert!(matrices_equal(&matrix, &original_matrix, 0.001));

            // Test shifts
            for _ in 0..size {
                shift_left(&mut matrix);
            }
            assert!(matrices_equal(&matrix, &original_matrix, 0.001));

            for _ in 0..size {
                shift_up(&mut matrix);
            }
            assert!(matrices_equal(&matrix, &original_matrix, 0.001));
        }
    }

    #[test]
    fn test_operation_invariants() {
        let mut matrix = create_test_matrix(4);

        // Test that operations preserve matrix dimensions
        let original_size = matrix.len();

        rotate_clockwise(&mut matrix);
        assert_eq!(matrix.len(), original_size);
        assert_eq!(matrix[0].len(), original_size);

        flip_horizontal(&mut matrix);
        assert_eq!(matrix.len(), original_size);
        assert_eq!(matrix[0].len(), original_size);

        shift_left(&mut matrix);
        assert_eq!(matrix.len(), original_size);
        assert_eq!(matrix[0].len(), original_size);

        // Test that all values remain in valid range [-1.0, 1.0]
        for row in &matrix {
            for &value in row {
                assert!(
                    (-1.0..=1.0).contains(&value),
                    "Value {} is outside valid range",
                    value
                );
            }
        }
    }

    #[test]
    fn test_rotation_inverses() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test that rotating left and then right gives the original matrix
        rotate_counterclockwise(&mut matrix);
        rotate_clockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test that rotating right and then left gives the original matrix
        rotate_clockwise(&mut matrix);
        rotate_counterclockwise(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));
    }

    #[test]
    fn test_zero_matrix() {
        let mut matrix = create_test_matrix(3);

        // Test that zeroing sets all values to 0
        zero_matrix(&mut matrix);
        for row in &matrix {
            for &value in row {
                assert!((value - 0.0).abs() < 0.001);
            }
        }

        // Test on different matrix sizes
        for size in 2..=6 {
            let mut matrix = create_test_matrix(size);
            zero_matrix(&mut matrix);
            for row in &matrix {
                for &value in row {
                    assert!((value - 0.0).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn test_flip_sign() {
        let mut matrix = create_test_matrix(3);
        let original_matrix = matrix.clone();

        // Test that flipping sign negates all values
        flip_sign(&mut matrix);
        for (i, row) in matrix.iter().enumerate().take(3) {
            for (j, &value) in row.iter().enumerate().take(3) {
                assert!((value - (-original_matrix[i][j])).abs() < 0.001);
            }
        }

        // Test that flipping sign twice returns to original
        flip_sign(&mut matrix);
        assert!(matrices_equal(&matrix, &original_matrix, 0.001));

        // Test with different values
        let mut test_matrix = vec![
            vec![1.0, -0.5, 0.0],
            vec![-1.0, 0.3, -0.8],
            vec![0.0, 0.7, -0.2],
        ];
        let expected = vec![
            vec![-1.0, 0.5, 0.0],
            vec![1.0, -0.3, 0.8],
            vec![0.0, -0.7, 0.2],
        ];

        flip_sign(&mut test_matrix);
        assert!(matrices_equal(&test_matrix, &expected, 0.001));
    }
}
