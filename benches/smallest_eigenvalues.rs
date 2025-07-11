use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use nalgebra_sparse::{CooMatrix, CsrMatrix};
use primme::smallest_nonzero_eigenvalues;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn generate_random_symmetric_matrix(n: usize, d: f64) -> CsrMatrix<f64> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut coo = CooMatrix::zeros(n, n);
    // Generate symmetric sparse matrix
    for i in 0..n {
        for j in i..n {
            if rng.random_bool(d) {
                let val: f64 = rng.sample(rand::distr::StandardUniform);
                coo.push(i, j, val);
                if i != j {
                    coo.push(j, i, val);
                }
            }
        }
    }
    let matrix = CsrMatrix::from(&coo);
    matrix
}

fn benchmark_smallest_eigenvalues(c: &mut Criterion) {
    // Sparse eigensolve benchmarks
    let sizes = [100usize, 500, 1000];

    let mut inv_group = c.benchmark_group("sparse eigensolve");
    inv_group.measurement_time(std::time::Duration::from_secs(10));
    inv_group.sample_size(10);

    let d = 0.01;
    for &n in &sizes {
        let id = BenchmarkId::from_parameter(format!("{}x{:.2}", n, d));
        inv_group.bench_with_input(id, &(n, d), |b, &(n, d)| {
            b.iter_batched(
                || generate_random_symmetric_matrix(n, d),
                |matrix| {
                    if let Err(e) =
                        std::hint::black_box(smallest_nonzero_eigenvalues(&matrix, 1, 1e-4))
                    {
                        println!("Error in computing eigenvalues: {e}")
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
    inv_group.finish();
}

criterion_group!(benches, benchmark_smallest_eigenvalues);
criterion_main!(benches);
