use serde::{Deserialize, Serialize};

/// A declarative multi-step setup loaded from YAML.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Recipe {
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub packages: PackageSpec,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Per-distro native packages plus a flatpak list that always applies.
/// Empty fields are skipped silently: a recipe that only targets Fedora
/// leaves `apt`/`pacman` empty, not an error.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PackageSpec {
    #[serde(default)]
    pub dnf: Vec<String>,
    #[serde(default)]
    pub apt: Vec<String>,
    #[serde(default)]
    pub pacman: Vec<String>,
    #[serde(default)]
    pub flatpak: Vec<String>,
}

impl PackageSpec {
    pub fn native_for(&self, distro: Distro) -> &[String] {
        match distro {
            Distro::Dnf => &self.dnf,
            Distro::Apt => &self.apt,
            Distro::Pacman => &self.pacman,
        }
    }
}

/// The package manager family we target for install steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    Dnf,
    Apt,
    Pacman,
}

/// One step within a recipe. Untagged so YAML stays terse: the presence of
/// `shell` vs `write_file` disambiguates.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Step {
    Shell {
        shell: String,
        #[serde(default)]
        describe: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    Write {
        write_file: WriteFile,
        #[serde(default)]
        describe: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WriteFile {
    pub path: String,
    pub content: String,
}

impl Step {
    pub fn describe(&self) -> String {
        match self {
            Step::Shell {
                describe, shell, ..
            } => describe
                .clone()
                .unwrap_or_else(|| format!("run: {}", first_line(shell))),
            Step::Write {
                describe,
                write_file,
            } => describe
                .clone()
                .unwrap_or_else(|| format!("write {}", write_file.path)),
        }
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}
