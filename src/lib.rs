use crate::xac::Mesh;
use pyo3::prelude::*;
use xac::SubMesh;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub mod ies;
pub mod ipf;
pub mod tosreader;
pub mod xac;

// Python bindings function
#[pyfunction]
fn extract_xac_data_py(ipf_path: String, xac_filename: String) -> PyResult<Vec<Mesh>> {
    match xac::extract_xac_data(&ipf_path, &xac_filename) {
        Ok(meshes) => {
            // Convert Rust Vec<Mesh> to Python list
            let py_meshes: Vec<Mesh> = meshes.into_iter().collect();
            Ok(py_meshes)
        }
        Err(err) => Err(PyErr::new::<pyo3::exceptions::PyOSError, _>(
            err.to_string(),
        )),
    }
}

// PyO3 module initialization
#[pymodule]
fn toslib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SubMesh>()?;
    m.add_class::<Mesh>()?;
    m.add_function(wrap_pyfunction!(extract_xac_data_py, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
