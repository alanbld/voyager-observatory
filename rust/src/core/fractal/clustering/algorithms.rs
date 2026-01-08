//! Clustering Algorithms - K-means and DBSCAN
//!
//! Pure Rust implementations of clustering algorithms for semantic analysis.
//! No external ML dependencies required.

// Note: HashMap/HashSet reserved for future clustering extensions (e.g., cluster metadata)

use thiserror::Error;

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Error)]
pub enum ClusteringError {
    #[error("Empty dataset")]
    EmptyDataset,

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Clustering algorithm failed: {0}")]
    AlgorithmError(String),

    #[error("Not converged after {0} iterations")]
    NotConverged(usize),
}

pub type ClusteringResult<T> = Result<T, ClusteringError>;

// =============================================================================
// Cluster Result
// =============================================================================

/// Result of a clustering operation.
#[derive(Debug, Clone)]
pub struct ClusterResult {
    /// Cluster label for each data point (-1 for noise/outliers)
    pub labels: Vec<i32>,
    /// Cluster centroids (for algorithms that compute them)
    pub centroids: Vec<Vec<f32>>,
    /// Number of clusters found
    pub n_clusters: usize,
    /// Inertia (within-cluster sum of squares) - for K-means
    pub inertia: f32,
    /// Silhouette score (-1 to 1, higher is better)
    pub silhouette_score: f32,
    /// Number of noise points (for DBSCAN)
    pub n_noise: usize,
    /// Size of each cluster
    pub cluster_sizes: Vec<usize>,
}

impl ClusterResult {
    /// Get indices of points in a specific cluster.
    pub fn get_cluster_indices(&self, cluster_id: i32) -> Vec<usize> {
        self.labels
            .iter()
            .enumerate()
            .filter(|(_, &label)| label == cluster_id)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get outlier indices (label == -1).
    pub fn get_outlier_indices(&self) -> Vec<usize> {
        self.get_cluster_indices(-1)
    }
}

// =============================================================================
// K-Means Clustering
// =============================================================================

/// K-means clustering algorithm.
pub struct KMeans {
    /// Number of clusters
    pub k: usize,
    /// Maximum iterations
    pub max_iter: usize,
    /// Convergence tolerance
    pub tolerance: f32,
    /// Random seed for reproducibility
    pub seed: u64,
}

impl Default for KMeans {
    fn default() -> Self {
        Self {
            k: 5,
            max_iter: 100,
            tolerance: 1e-4,
            seed: 42,
        }
    }
}

impl KMeans {
    pub fn new(k: usize) -> Self {
        Self {
            k,
            ..Default::default()
        }
    }

    pub fn with_max_iter(mut self, max_iter: usize) -> Self {
        self.max_iter = max_iter;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Perform K-means clustering.
    pub fn fit(&self, data: &[Vec<f32>]) -> ClusteringResult<ClusterResult> {
        if data.is_empty() {
            return Err(ClusteringError::EmptyDataset);
        }

        let n_samples = data.len();
        let n_features = data[0].len();

        if self.k > n_samples {
            return Err(ClusteringError::InvalidParameters(format!(
                "k={} cannot exceed sample count={}",
                self.k, n_samples
            )));
        }

        if self.k == 0 {
            return Err(ClusteringError::InvalidParameters(
                "k must be at least 1".to_string(),
            ));
        }

        // Validate dimensions
        for point in data.iter() {
            if point.len() != n_features {
                return Err(ClusteringError::DimensionMismatch {
                    expected: n_features,
                    actual: point.len(),
                });
            }
        }

        // Initialize centroids using k-means++ initialization
        let mut centroids = self.initialize_centroids_plusplus(data);
        let mut labels = vec![0i32; n_samples];
        let mut prev_inertia = f32::INFINITY;

        for _iteration in 0..self.max_iter {
            // Assign points to nearest centroid
            let mut inertia = 0.0;
            for (i, point) in data.iter().enumerate() {
                let (nearest, dist) = self.find_nearest_centroid(point, &centroids);
                labels[i] = nearest as i32;
                inertia += dist * dist;
            }

            // Check for convergence
            if (prev_inertia - inertia).abs() < self.tolerance {
                break;
            }
            prev_inertia = inertia;

            // Update centroids
            let new_centroids = self.compute_centroids(data, &labels, n_features);

            // Handle empty clusters
            centroids = new_centroids
                .into_iter()
                .enumerate()
                .map(|(i, c)| {
                    if c.iter().all(|&v| v == 0.0) {
                        // Empty cluster - reinitialize with random point
                        data[self.simple_random(i, n_samples)].clone()
                    } else {
                        c
                    }
                })
                .collect();
        }

        // Calculate final inertia
        let inertia = self.calculate_inertia(data, &labels, &centroids);

        // Calculate cluster sizes
        let mut cluster_sizes = vec![0usize; self.k];
        for &label in &labels {
            if label >= 0 && (label as usize) < self.k {
                cluster_sizes[label as usize] += 1;
            }
        }

        // Calculate silhouette score
        let silhouette_score = self.calculate_silhouette(data, &labels);

        Ok(ClusterResult {
            labels,
            centroids,
            n_clusters: self.k,
            inertia,
            silhouette_score,
            n_noise: 0,
            cluster_sizes,
        })
    }

    /// K-means++ initialization for better starting centroids.
    fn initialize_centroids_plusplus(&self, data: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let n_samples = data.len();
        let mut centroids = Vec::with_capacity(self.k);

        // First centroid: random point
        let first_idx = self.simple_random(0, n_samples);
        centroids.push(data[first_idx].clone());

        // Remaining centroids: weighted by distance squared
        for c in 1..self.k {
            let mut distances: Vec<f32> = data
                .iter()
                .map(|point| {
                    centroids
                        .iter()
                        .map(|centroid| euclidean_distance(point, centroid))
                        .fold(f32::INFINITY, f32::min)
                })
                .collect();

            // Square distances for weighting
            for d in distances.iter_mut() {
                *d = d.powi(2);
            }

            // Select point with probability proportional to squared distance
            let total: f32 = distances.iter().sum();
            if total == 0.0 {
                // All points are centroids already
                centroids.push(data[self.simple_random(c, n_samples)].clone());
                continue;
            }

            let threshold = (self.simple_random(c * 100, 1000) as f32 / 1000.0) * total;
            let mut cumsum = 0.0;
            let mut selected = 0;

            for (i, &dist) in distances.iter().enumerate() {
                cumsum += dist;
                if cumsum >= threshold {
                    selected = i;
                    break;
                }
            }

            centroids.push(data[selected].clone());
        }

        centroids
    }

    fn find_nearest_centroid(&self, point: &[f32], centroids: &[Vec<f32>]) -> (usize, f32) {
        centroids
            .iter()
            .enumerate()
            .map(|(i, c)| (i, euclidean_distance(point, c)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or((0, f32::INFINITY))
    }

    fn compute_centroids(
        &self,
        data: &[Vec<f32>],
        labels: &[i32],
        n_features: usize,
    ) -> Vec<Vec<f32>> {
        let mut sums = vec![vec![0.0f32; n_features]; self.k];
        let mut counts = vec![0usize; self.k];

        for (point, &label) in data.iter().zip(labels.iter()) {
            if label >= 0 {
                let cluster = label as usize;
                counts[cluster] += 1;
                for (i, &val) in point.iter().enumerate() {
                    sums[cluster][i] += val;
                }
            }
        }

        sums.into_iter()
            .zip(counts.iter())
            .map(|(sum, &count)| {
                if count > 0 {
                    sum.into_iter().map(|v| v / count as f32).collect()
                } else {
                    vec![0.0; n_features]
                }
            })
            .collect()
    }

    fn calculate_inertia(&self, data: &[Vec<f32>], labels: &[i32], centroids: &[Vec<f32>]) -> f32 {
        data.iter()
            .zip(labels.iter())
            .map(|(point, &label)| {
                if label >= 0 && (label as usize) < centroids.len() {
                    let dist = euclidean_distance(point, &centroids[label as usize]);
                    dist * dist
                } else {
                    0.0
                }
            })
            .sum()
    }

    fn calculate_silhouette(&self, data: &[Vec<f32>], labels: &[i32]) -> f32 {
        if data.len() <= 1 || self.k <= 1 {
            return 0.0;
        }

        let mut total_score = 0.0;
        let mut valid_points = 0;

        for i in 0..data.len() {
            let label_i = labels[i];
            if label_i < 0 {
                continue;
            }

            // a(i): average distance to same cluster
            let mut a_sum = 0.0;
            let mut a_count = 0;
            for j in 0..data.len() {
                if i != j && labels[j] == label_i {
                    a_sum += euclidean_distance(&data[i], &data[j]);
                    a_count += 1;
                }
            }
            let a_i = if a_count > 0 {
                a_sum / a_count as f32
            } else {
                0.0
            };

            // b(i): minimum average distance to other clusters
            let mut b_i = f32::INFINITY;
            for cluster in 0..self.k as i32 {
                if cluster == label_i {
                    continue;
                }

                let mut b_sum = 0.0;
                let mut b_count = 0;
                for j in 0..data.len() {
                    if labels[j] == cluster {
                        b_sum += euclidean_distance(&data[i], &data[j]);
                        b_count += 1;
                    }
                }

                if b_count > 0 {
                    let avg = b_sum / b_count as f32;
                    if avg < b_i {
                        b_i = avg;
                    }
                }
            }

            if b_i == f32::INFINITY {
                continue;
            }

            let s_i = (b_i - a_i) / a_i.max(b_i);
            if !s_i.is_nan() {
                total_score += s_i;
                valid_points += 1;
            }
        }

        if valid_points > 0 {
            total_score / valid_points as f32
        } else {
            0.0
        }
    }

    fn simple_random(&self, iter: usize, max: usize) -> usize {
        // Simple deterministic "random" for reproducibility
        let hash = self
            .seed
            .wrapping_mul(iter as u64 + 1)
            .wrapping_add(0x9e3779b97f4a7c15);
        (hash % max as u64) as usize
    }
}

// =============================================================================
// DBSCAN Clustering
// =============================================================================

/// DBSCAN (Density-Based Spatial Clustering of Applications with Noise).
pub struct DBSCAN {
    /// Maximum distance between two samples to be considered neighbors
    pub eps: f32,
    /// Minimum number of samples in a neighborhood to form a core point
    pub min_samples: usize,
}

impl Default for DBSCAN {
    fn default() -> Self {
        Self {
            eps: 0.5,
            min_samples: 5,
        }
    }
}

impl DBSCAN {
    pub fn new(eps: f32, min_samples: usize) -> Self {
        Self { eps, min_samples }
    }

    /// Perform DBSCAN clustering.
    pub fn fit(&self, data: &[Vec<f32>]) -> ClusteringResult<ClusterResult> {
        if data.is_empty() {
            return Err(ClusteringError::EmptyDataset);
        }

        if self.eps <= 0.0 {
            return Err(ClusteringError::InvalidParameters(
                "eps must be positive".to_string(),
            ));
        }

        if self.min_samples == 0 {
            return Err(ClusteringError::InvalidParameters(
                "min_samples must be at least 1".to_string(),
            ));
        }

        let n_samples = data.len();
        let n_features = data[0].len();

        // Validate dimensions
        for point in data.iter() {
            if point.len() != n_features {
                return Err(ClusteringError::DimensionMismatch {
                    expected: n_features,
                    actual: point.len(),
                });
            }
        }

        // Labels: -1 = noise, -2 = unvisited
        let mut labels = vec![-2i32; n_samples];
        let mut current_cluster = 0i32;

        for i in 0..n_samples {
            if labels[i] != -2 {
                continue; // Already processed
            }

            // Find neighbors
            let neighbors = self.region_query(data, i);

            if neighbors.len() < self.min_samples {
                labels[i] = -1; // Mark as noise
            } else {
                // Expand cluster
                self.expand_cluster(data, i, &neighbors, current_cluster, &mut labels);
                current_cluster += 1;
            }
        }

        // Calculate statistics
        let n_clusters = current_cluster as usize;
        let n_noise = labels.iter().filter(|&&l| l == -1).count();

        // Cluster sizes
        let mut cluster_sizes = vec![0usize; n_clusters];
        for &label in &labels {
            if label >= 0 && (label as usize) < n_clusters {
                cluster_sizes[label as usize] += 1;
            }
        }

        // Compute centroids
        let centroids = self.compute_centroids(data, &labels, n_clusters, n_features);

        // Silhouette score (skip if only one cluster or too many noise)
        let silhouette_score = if n_clusters >= 2 && n_noise < n_samples / 2 {
            self.calculate_silhouette(data, &labels, n_clusters)
        } else {
            0.0
        };

        Ok(ClusterResult {
            labels,
            centroids,
            n_clusters,
            inertia: 0.0, // DBSCAN doesn't use inertia
            silhouette_score,
            n_noise,
            cluster_sizes,
        })
    }

    fn region_query(&self, data: &[Vec<f32>], point_idx: usize) -> Vec<usize> {
        let point = &data[point_idx];
        data.iter()
            .enumerate()
            .filter(|(i, other)| *i != point_idx && euclidean_distance(point, other) <= self.eps)
            .map(|(i, _)| i)
            .collect()
    }

    fn expand_cluster(
        &self,
        data: &[Vec<f32>],
        point_idx: usize,
        neighbors: &[usize],
        cluster_id: i32,
        labels: &mut [i32],
    ) {
        labels[point_idx] = cluster_id;

        let mut seeds: Vec<usize> = neighbors.to_vec();
        let mut i = 0;

        while i < seeds.len() {
            let q = seeds[i];

            if labels[q] == -1 {
                // Change noise to border point
                labels[q] = cluster_id;
            }

            if labels[q] == -2 {
                // Not yet visited
                labels[q] = cluster_id;

                let q_neighbors = self.region_query(data, q);
                if q_neighbors.len() >= self.min_samples {
                    // Core point - add its neighbors to seeds
                    for neighbor in q_neighbors {
                        if !seeds.contains(&neighbor) {
                            seeds.push(neighbor);
                        }
                    }
                }
            }

            i += 1;
        }
    }

    fn compute_centroids(
        &self,
        data: &[Vec<f32>],
        labels: &[i32],
        n_clusters: usize,
        n_features: usize,
    ) -> Vec<Vec<f32>> {
        let mut sums = vec![vec![0.0f32; n_features]; n_clusters];
        let mut counts = vec![0usize; n_clusters];

        for (point, &label) in data.iter().zip(labels.iter()) {
            if label >= 0 && (label as usize) < n_clusters {
                let cluster = label as usize;
                counts[cluster] += 1;
                for (i, &val) in point.iter().enumerate() {
                    sums[cluster][i] += val;
                }
            }
        }

        sums.into_iter()
            .zip(counts.iter())
            .map(|(sum, &count)| {
                if count > 0 {
                    sum.into_iter().map(|v| v / count as f32).collect()
                } else {
                    vec![0.0; n_features]
                }
            })
            .collect()
    }

    fn calculate_silhouette(&self, data: &[Vec<f32>], labels: &[i32], n_clusters: usize) -> f32 {
        if n_clusters <= 1 {
            return 0.0;
        }

        let mut total_score = 0.0;
        let mut valid_points = 0;

        for i in 0..data.len() {
            let label_i = labels[i];
            if label_i < 0 {
                continue; // Skip noise
            }

            // a(i): average distance to same cluster
            let mut a_sum = 0.0;
            let mut a_count = 0;
            for j in 0..data.len() {
                if i != j && labels[j] == label_i {
                    a_sum += euclidean_distance(&data[i], &data[j]);
                    a_count += 1;
                }
            }
            let a_i = if a_count > 0 {
                a_sum / a_count as f32
            } else {
                0.0
            };

            // b(i): minimum average distance to other clusters
            let mut b_i = f32::INFINITY;
            for cluster in 0..n_clusters as i32 {
                if cluster == label_i {
                    continue;
                }

                let mut b_sum = 0.0;
                let mut b_count = 0;
                for j in 0..data.len() {
                    if labels[j] == cluster {
                        b_sum += euclidean_distance(&data[i], &data[j]);
                        b_count += 1;
                    }
                }

                if b_count > 0 {
                    let avg = b_sum / b_count as f32;
                    if avg < b_i {
                        b_i = avg;
                    }
                }
            }

            if b_i == f32::INFINITY {
                continue;
            }

            let s_i = (b_i - a_i) / a_i.max(b_i);
            if !s_i.is_nan() {
                total_score += s_i;
                valid_points += 1;
            }
        }

        if valid_points > 0 {
            total_score / valid_points as f32
        } else {
            0.0
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate Euclidean distance between two vectors.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Calculate cosine distance between two vectors.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

/// Calculate cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

/// Calculate Manhattan distance between two vectors.
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_clustered_data() -> Vec<Vec<f32>> {
        // Create 3 clear clusters
        let mut data = Vec::new();

        // Cluster 0: around (0, 0)
        data.push(vec![0.0, 0.1]);
        data.push(vec![0.1, 0.0]);
        data.push(vec![0.0, 0.0]);
        data.push(vec![0.1, 0.1]);
        data.push(vec![0.05, 0.05]);

        // Cluster 1: around (5, 5)
        data.push(vec![5.0, 5.1]);
        data.push(vec![5.1, 5.0]);
        data.push(vec![5.0, 5.0]);
        data.push(vec![5.1, 5.1]);
        data.push(vec![5.05, 5.05]);

        // Cluster 2: around (0, 5)
        data.push(vec![0.0, 5.1]);
        data.push(vec![0.1, 5.0]);
        data.push(vec![0.0, 5.0]);
        data.push(vec![0.1, 5.1]);
        data.push(vec![0.05, 5.05]);

        data
    }

    // -------------------------------------------------------------------------
    // K-means Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_kmeans_basic() {
        let data = create_clustered_data();
        let kmeans = KMeans::new(3);

        let result = kmeans.fit(&data).unwrap();

        assert_eq!(result.n_clusters, 3);
        assert_eq!(result.labels.len(), 15);
        assert_eq!(result.centroids.len(), 3);
    }

    #[test]
    fn test_kmeans_finds_clusters() {
        let data = create_clustered_data();
        let kmeans = KMeans::new(3).with_seed(42);

        let result = kmeans.fit(&data).unwrap();

        // Points in same cluster should have same label
        // First 5 points should be in same cluster
        let first_cluster = result.labels[0];
        assert!(result.labels[0..5].iter().all(|&l| l == first_cluster));

        // Points 5-10 should be in same cluster
        let second_cluster = result.labels[5];
        assert!(result.labels[5..10].iter().all(|&l| l == second_cluster));

        // Different clusters should have different labels
        assert_ne!(first_cluster, second_cluster);
    }

    #[test]
    fn test_kmeans_empty_data() {
        let data: Vec<Vec<f32>> = Vec::new();
        let kmeans = KMeans::new(3);

        let result = kmeans.fit(&data);
        assert!(matches!(result, Err(ClusteringError::EmptyDataset)));
    }

    #[test]
    fn test_kmeans_k_too_large() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let kmeans = KMeans::new(5);

        let result = kmeans.fit(&data);
        assert!(matches!(result, Err(ClusteringError::InvalidParameters(_))));
    }

    #[test]
    fn test_kmeans_silhouette() {
        let data = create_clustered_data();
        let kmeans = KMeans::new(3);

        let result = kmeans.fit(&data).unwrap();

        // Well-separated clusters should have positive silhouette
        assert!(result.silhouette_score > 0.0);
    }

    #[test]
    fn test_kmeans_cluster_sizes() {
        let data = create_clustered_data();
        let kmeans = KMeans::new(3);

        let result = kmeans.fit(&data).unwrap();

        // Each cluster should have 5 points
        assert_eq!(result.cluster_sizes.iter().sum::<usize>(), 15);
    }

    // -------------------------------------------------------------------------
    // DBSCAN Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_dbscan_basic() {
        let data = create_clustered_data();
        let dbscan = DBSCAN::new(0.5, 2);

        let result = dbscan.fit(&data).unwrap();

        // Should find 3 clusters
        assert_eq!(result.n_clusters, 3);
        assert_eq!(result.labels.len(), 15);
    }

    #[test]
    fn test_dbscan_with_noise() {
        let mut data = create_clustered_data();
        // Add outlier
        data.push(vec![10.0, 10.0]);

        let dbscan = DBSCAN::new(0.5, 2);
        let result = dbscan.fit(&data).unwrap();

        // Outlier should be marked as noise
        assert!(result.labels.iter().any(|&l| l == -1));
        assert!(result.n_noise > 0);
    }

    #[test]
    fn test_dbscan_empty_data() {
        let data: Vec<Vec<f32>> = Vec::new();
        let dbscan = DBSCAN::new(0.5, 2);

        let result = dbscan.fit(&data);
        assert!(matches!(result, Err(ClusteringError::EmptyDataset)));
    }

    #[test]
    fn test_dbscan_invalid_eps() {
        let data = vec![vec![1.0, 2.0]];
        let dbscan = DBSCAN::new(-1.0, 2);

        let result = dbscan.fit(&data);
        assert!(matches!(result, Err(ClusteringError::InvalidParameters(_))));
    }

    #[test]
    fn test_dbscan_get_cluster_indices() {
        let data = create_clustered_data();
        let dbscan = DBSCAN::new(0.5, 2);

        let result = dbscan.fit(&data).unwrap();

        // Get indices for cluster 0
        let indices = result.get_cluster_indices(0);
        assert!(!indices.is_empty());

        // All indices should have label 0
        for &idx in &indices {
            assert_eq!(result.labels[idx], 0);
        }
    }

    // -------------------------------------------------------------------------
    // Distance Function Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];

        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];

        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];

        let dist = manhattan_distance(&a, &b);
        assert!((dist - 7.0).abs() < 1e-6);
    }

    // -------------------------------------------------------------------------
    // Edge Cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_kmeans_single_point() {
        let data = vec![vec![1.0, 2.0]];
        let kmeans = KMeans::new(1);

        let result = kmeans.fit(&data).unwrap();

        assert_eq!(result.n_clusters, 1);
        assert_eq!(result.labels, vec![0]);
    }

    #[test]
    fn test_dbscan_all_same_point() {
        let data = vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]];
        let dbscan = DBSCAN::new(0.5, 2);

        let result = dbscan.fit(&data).unwrap();

        // All points should be in same cluster
        assert_eq!(result.n_clusters, 1);
    }

    #[test]
    fn test_dimension_mismatch() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0, 5.0]];
        let kmeans = KMeans::new(2);

        let result = kmeans.fit(&data);
        assert!(matches!(
            result,
            Err(ClusteringError::DimensionMismatch { .. })
        ));
    }
}
