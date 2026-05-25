//! Audit logging middleware for pdftract CLI.

pub mod audit;

pub use audit::{audit_middleware, AuditState};
