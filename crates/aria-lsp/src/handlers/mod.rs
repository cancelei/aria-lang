//! LSP Request and Notification Handlers
//!
//! This module contains handlers for all LSP protocol messages.
//! Each submodule handles a specific category of LSP functionality.
//!
//! # Handler Categories
//!
//! - [`initialize`]: Server initialization and shutdown
//! - [`shutdown`]: Server shutdown handling
//! - [`document`]: Text document synchronization
//! - [`hover`]: Hover information
//! - [`completion`]: Code completion
//! - [`definition`]: Go-to-definition and related navigation
//! - [`document_symbol`]: Document outline and symbol navigation

pub mod completion;
pub mod definition;
pub mod document;
pub mod document_symbol;
pub mod hover;
pub mod initialize;
pub mod shutdown;
