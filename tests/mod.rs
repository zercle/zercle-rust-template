//! Test module
//
// This is the root of the test hierarchy. Tests are organized into:
// - unit: Unit tests with mocked dependencies
// - integration: Integration tests with real database connections
// - common: Shared test utilities and helpers

pub mod unit;
pub mod integration;
pub mod common;
