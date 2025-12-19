//! Patchwork: Blend deterministic Rust code with LLM-powered reasoning.
//!
//! This library provides a builder-style API for constructing prompts that can
//! invoke Rust closures as MCP tools, enabling seamless interleaving of structured
//! code and natural language processing.
//!
//! # Example
//!
//! ```rust,ignore
//! use patchwork::Patchwork;
//! use sacp::Component;
//!
//! let patchwork = Patchwork::new(component).await?;
//!
//! let name = "Alice";
//! let result: String = patchwork.think()
//!     .text("Say hello to")
//!     .display(&name)
//!     .text("in a friendly way.")
//!     .run()
//!     .await?;
//! ```

mod error;
mod patchwork;
mod think;

pub use error::Error;
pub use patchwork::Patchwork;
pub use think::ThinkBuilder;
