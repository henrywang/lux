//! Recipe registry for the lux agent.
//!
//! A recipe is an opinionated, multi-step setup (e.g. "zsh-popular",
//! "ai-dev-cpu") described declaratively in YAML. The registry loads bundled
//! recipes at compile time; `lux-tools::apply_recipe` expands a recipe into
//! concrete side effects (package install, shell exec, file write).

mod recipe;
mod registry;

pub use recipe::{Distro, PackageSpec, Recipe, Step, WriteFile};
pub use registry::RecipeRegistry;
