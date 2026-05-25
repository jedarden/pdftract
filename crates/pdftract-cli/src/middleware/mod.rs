//! Audit logging middleware for pdftract CLI.

pub mod audit;

pub use audit::{AuditState, audit_middleware};
