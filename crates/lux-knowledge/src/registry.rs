use anyhow::{Context, Result};

use crate::recipe::Recipe;

/// Bundled recipes, compiled in at build time via `include_str!`. Each entry
/// is `(file-stem-used-as-id, raw-yaml)`. The name inside the YAML must match
/// the id — a debug_assert in `RecipeRegistry::new` enforces this.
const BUNDLED: &[(&str, &str)] = &[
    ("zsh-popular", include_str!("../recipes/zsh-popular.yaml")),
    (
        "ghostty-default",
        include_str!("../recipes/ghostty-default.yaml"),
    ),
    ("ai-dev-cpu", include_str!("../recipes/ai-dev-cpu.yaml")),
    ("ai-dev-cuda", include_str!("../recipes/ai-dev-cuda.yaml")),
    (
        "editor-vscodium",
        include_str!("../recipes/editor-vscodium.yaml"),
    ),
    ("editor-zed", include_str!("../recipes/editor-zed.yaml")),
];

pub struct RecipeRegistry {
    recipes: Vec<Recipe>,
}

impl RecipeRegistry {
    pub fn new() -> Result<Self> {
        let mut recipes = Vec::with_capacity(BUNDLED.len());
        for (id, yaml) in BUNDLED {
            let recipe: Recipe = serde_yaml::from_str(yaml)
                .with_context(|| format!("parsing bundled recipe `{id}`"))?;
            debug_assert_eq!(
                recipe.name, *id,
                "recipe file stem must match `name:` field"
            );
            recipes.push(recipe);
        }
        Ok(Self { recipes })
    }

    pub fn get(&self, name: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.name == name)
    }

    pub fn list(&self) -> impl Iterator<Item = &Recipe> {
        self.recipes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Step;

    #[test]
    fn bundled_recipes_load() {
        let reg = RecipeRegistry::new().expect("bundled recipes must parse");
        let names: Vec<&str> = reg.list().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"zsh-popular"));
        assert!(names.contains(&"ai-dev-cpu"));
    }

    #[test]
    fn get_returns_recipe_by_name() {
        let reg = RecipeRegistry::new().unwrap();
        assert!(reg.get("zsh-popular").is_some());
        assert!(reg.get("does-not-exist").is_none());
    }

    #[test]
    fn no_recipe_uses_flatpak_install() {
        // After v2: recipes install via each tool's canonical method
        // (vendor script or native repo), not Flathub repackages. Config
        // paths stay at ~/.config/<app>/ — no sandbox redirection needed.
        let reg = RecipeRegistry::new().unwrap();
        for r in reg.list() {
            assert!(
                r.packages.flatpak.is_empty(),
                "recipe {} still lists flatpak packages; migrate to canonical install",
                r.name
            );
        }
    }

    #[test]
    fn editor_zed_writes_unsandboxed_settings_path() {
        let reg = RecipeRegistry::new().unwrap();
        let zed = reg.get("editor-zed").unwrap();
        let write = zed
            .steps
            .iter()
            .find_map(|s| match s {
                Step::Write { write_file, .. } => Some(write_file),
                _ => None,
            })
            .expect("editor-zed has a write_file step");
        assert_eq!(write.path, "~/.config/zed/settings.json");
    }
}
