#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use libc::c_void;
use nalgebra_sparse::{CsrMatrix, na::DVector};

unsafe extern "C" fn matvec_callback(
    x: *mut c_void,
    _id_x: *mut i64,
    y: *mut c_void,
    _id_y: *mut i64,
    _block_size: *mut i32,
    primme: *mut primme_params,
    ierr: *mut i32,
) {
    // Recover matrix
    let A = unsafe { &*((*primme).matrix as *const CsrMatrix<f64>) };
    let n = A.nrows();

    // Convert x and y to DVector
    let x_slice = unsafe { std::slice::from_raw_parts(x as *const f64, n) };
    let y_slice = unsafe { std::slice::from_raw_parts_mut(y as *mut f64, n) };

    let x_vec = DVector::from_column_slice(x_slice);
    let result = A * &x_vec;

    // Write result into y
    for i in 0..n {
        y_slice[i] = result[i];
    }
    // Indicate success
    unsafe { *ierr = 0 };
}

pub fn smallest_nonzero_eigenvalues(
    A: &CsrMatrix<f64>,
    nev: usize,
    above_zero: f64,
) -> Result<Vec<f64>, i32> {
    // Prepare storage
    let n = A.nrows() as i64;
    let mut evals = vec![0.0; nev];
    let mut res_norms = vec![0.0; nev];
    let mut evecs = vec![0.0; (n as usize) * nev];

    let mut shifts = [above_zero];

    // Init PRIMME
    let mut primme = unsafe { std::mem::zeroed::<primme_params>() };
    unsafe { primme_initialize(&mut primme) };
    primme.n = n;
    primme.numEvals = nev as i32;
    primme.target = primme_target_primme_closest_geq;
    primme.matrix = A as *const _ as *mut c_void;
    primme.targetShifts = shifts.as_mut_ptr();
    primme.numTargetShifts = 1 as i32;
    primme.matrixMatvec = Some(matvec_callback);
    primme.maxMatvecs = n * n as i64;
    primme.eps = above_zero * 1e-4 as f64;
    let ret_method =
        unsafe { primme_set_method(primme_preset_method_PRIMME_DEFAULT_MIN_TIME, &mut primme) };
    if ret_method != 0 {
        return Err(ret_method);
    }

    let ret = unsafe {
        dprimme(
            evals.as_mut_ptr(),
            evecs.as_mut_ptr(),
            res_norms.as_mut_ptr(),
            &mut primme,
        )
    };
    unsafe { primme_free(&mut primme) };
    if ret == 0 { Ok(evals) } else { Err(ret) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra_sparse::{CooMatrix, CsrMatrix};
    use rand::{SeedableRng, rngs::StdRng};
    #[test]
    fn test_smallest_eigenvalues_2x2() {
        // Matrix: [2.0, 0.0]
        //         [0.0, 3.0]
        // Eigenvalues: 2.0, 3.0

        let mut matrix = CooMatrix::zeros(2, 2);
        matrix.push(0, 0, 2.0);
        matrix.push(1, 1, 3.0);
        let matrix = CsrMatrix::from(&matrix);

        let evals = smallest_nonzero_eigenvalues(&matrix, 2, 1e-6).expect("PRIMME failed");
        let mut sorted = evals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        assert!((sorted[0] - 2.0).abs() < 1e-8);
        assert!((sorted[1] - 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_smallest_eigenvalues_random_500x500() {
        use nalgebra::{DMatrix, SymmetricEigen};
        use nalgebra_sparse::{CooMatrix, CsrMatrix};
        use rand::Rng;

        let n = 100;
        let mut rng = StdRng::seed_from_u64(42);
        let mut coo = CooMatrix::zeros(n, n);

        // Generate symmetric sparse matrix
        for i in 0..n {
            for j in i..n {
                if rng.random_bool(0.01) {
                    let val: f64 = rng.sample(rand::distr::StandardUniform);
                    coo.push(i, j, val);
                    if i != j {
                        coo.push(j, i, val);
                    }
                }
            }
        }

        let csr = CsrMatrix::from(&coo);
        let evals = smallest_nonzero_eigenvalues(&csr, 5, 1e-6).expect("PRIMME failed");
        println!("Evals: {:?}", evals);
        // Convert to dense for reference
        let mut dense = DMatrix::<f64>::zeros(n, n);
        for (i, j, val) in coo.triplet_iter() {
            dense[(i, j)] = *val;
        }

        let expected = {
            let mut e = SymmetricEigen::new(dense).eigenvalues.data.as_vec().clone();
            e.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let e: Vec<f64> = e.into_iter().filter(|x| x > &1e-6).collect();
            e
        };
        for i in 0..5 {
            assert!((evals[i] - expected[i]).abs() < 1e-6);
        }
    }
}
