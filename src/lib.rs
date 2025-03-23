pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub mod ies;
pub mod ipf;
pub mod tosreader;
pub mod xac;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
