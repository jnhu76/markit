//! Provider-internal implementation modules: the normalized result
//! vocabulary, the test-only conformance validator, and the
//! restart-convergence mechanism. Nothing here is product API.

pub(crate) mod normalize;
pub(crate) mod restart;

#[cfg(test)]
pub(crate) mod validate;
