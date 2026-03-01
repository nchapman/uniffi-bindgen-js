pub fn test_harness_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::test_harness_ready;

    #[test]
    fn harness_ready_works() {
        assert!(test_harness_ready());
    }
}
