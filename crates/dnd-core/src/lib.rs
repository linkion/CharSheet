//! D&D 5e (2024 rules) character model and rules engine. No Qt dependencies.

pub mod abilities;
pub mod armor;
pub mod character;
pub mod classes;
pub mod dice;
pub mod edit;
pub mod effects;
pub mod play;
pub mod storage;
pub mod view;

pub use character::Character;
pub use storage::Library;
