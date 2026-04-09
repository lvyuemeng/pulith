//! Target resolver abstraction for shim binary.
//!
//! # Architecture
//!
//! This module defines the `TargetResolver` trait - the only contract
//! between shim binary and user-defined resolution policy.
//!
//! Shim is a mechanism, not policy. The resolver implements policy.

use std::path::PathBuf;

pub trait TargetResolver {
    fn resolve(&self, command: &str) -> Option<PathBuf>;
}

#[derive(Clone)]
pub struct PairResolver<R1, R2> {
    primary: R1,
    fallback: R2,
}

impl<R1, R2> PairResolver<R1, R2>
where
    R1: TargetResolver,
    R2: TargetResolver,
{
    pub fn new(primary: R1, fallback: R2) -> Self {
        Self { primary, fallback }
    }
}

impl<R1, R2> TargetResolver for PairResolver<R1, R2>
where
    R1: TargetResolver,
    R2: TargetResolver,
{
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        self.primary
            .resolve(command)
            .or_else(|| self.fallback.resolve(command))
    }
}

#[derive(Clone)]
pub struct TripleResolver<R1, R2, R3> {
    first: R1,
    second: R2,
    third: R3,
}

impl<R1, R2, R3> TripleResolver<R1, R2, R3>
where
    R1: TargetResolver,
    R2: TargetResolver,
    R3: TargetResolver,
{
    pub fn new(first: R1, second: R2, third: R3) -> Self {
        Self {
            first,
            second,
            third,
        }
    }
}

impl<R1, R2, R3> TargetResolver for TripleResolver<R1, R2, R3>
where
    R1: TargetResolver,
    R2: TargetResolver,
    R3: TargetResolver,
{
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        self.first
            .resolve(command)
            .or_else(|| self.second.resolve(command))
            .or_else(|| self.third.resolve(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockResolver(&'static str, Option<PathBuf>);

    impl TargetResolver for MockResolver {
        fn resolve(&self, command: &str) -> Option<PathBuf> {
            if command == self.0 {
                self.1.clone()
            } else {
                None
            }
        }
    }

    #[test]
    fn test_pair_resolver_fallback() {
        let primary = MockResolver("cmd1", Some(PathBuf::from("/primary/cmd1")));
        let fallback = MockResolver("cmd2", Some(PathBuf::from("/fallback/cmd2")));

        let resolver = PairResolver::new(primary, fallback);

        assert_eq!(
            resolver.resolve("cmd1"),
            Some(PathBuf::from("/primary/cmd1"))
        );
        assert_eq!(
            resolver.resolve("cmd2"),
            Some(PathBuf::from("/fallback/cmd2"))
        );
        assert_eq!(resolver.resolve("cmd3"), None);
    }

    #[test]
    fn test_triple_resolver_chain() {
        let first = MockResolver("cmd1", Some(PathBuf::from("/first/cmd1")));
        let second = MockResolver("cmd2", Some(PathBuf::from("/second/cmd2")));
        let third = MockResolver("cmd3", Some(PathBuf::from("/third/cmd3")));

        let resolver = TripleResolver::new(first, second, third);

        assert_eq!(resolver.resolve("cmd1"), Some(PathBuf::from("/first/cmd1")));
        assert_eq!(
            resolver.resolve("cmd2"),
            Some(PathBuf::from("/second/cmd2"))
        );
        assert_eq!(resolver.resolve("cmd3"), Some(PathBuf::from("/third/cmd3")));
        assert_eq!(resolver.resolve("cmd4"), None);
    }
}
