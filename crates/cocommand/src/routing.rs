//! Capability routing — maps intents to tools and apps (Core-6).

pub mod metadata;
pub mod router;

pub use metadata::RoutingMetadata;
pub use router::{RouteCandidate, Router, RoutingResult};
