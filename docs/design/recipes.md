# Recipes and the install philosophy

This doc explains the design of `lux-knowledge` (recipe registry) and
`lux-tools::apply_recipe`, and the principles that guide how lux installs
software — both other tools (via recipes) and itself.

## The motivating problem

A real user request:

> I want to install ghostty, zsh and configure them with the most popular
> plugin installed and popular theme.

The small (1.7B) local model can't reliably answer this. "Popular" is a
moving target, exact install commands require specific URLs and flags, and
configuration lives across several files. Vector RAG over man pages
wouldn't help — man pages don't rank popularity.

What the request *actually* asks for is an **opinionated, multi-step
setup**: a named bundle of packages + shell commands + config files that
someone with taste has already assembled.

## Two mechanisms, one principle

lux has **two curated knowledge bases** for installing software. They look
different because the shapes of install requests differ, but both
are **human-maintained**, not model-generated.

| Mechanism | Shape of install | Example |
|---|---|---|
| `GUI_APPS` table in `lux-agent/src/intent.rs` | Single package, known distro mapping | `install firefox` → `dnf install firefox` (or flatpak on image-mode) |
| Recipe YAML in `lux-knowledge/recipes/` | Multi-step opinionated setup (≥3 steps that go together) | `set up zsh nicely` → install zsh + oh-my-zsh + plugins + theme + write `.zshrc` |

Rule of thumb: if the answer is one package-manager call, put it in
`GUI_APPS`. If it's "run X, clone Y, write file Z, maybe run W," write a
recipe.

### Principle: recipes are cross-distro; the rest of lux is Fedora-first

The recipe layer and the core tool layer have **different distro
support stories**, and that split is intentional.

| Layer | Fedora/RHEL | Arch | Debian/Ubuntu |
|---|---|---|---|
| Core tools (`install_package`, `manage_firewall`, `bootc_*`, `rpm -q` shortcuts) | Supported | Not yet | Not yet |
| Recipes (`apply_recipe`) | Primary target | Best-effort | Per-recipe |

**Why split:** the install-method dispatch each recipe needs (`command -v
dnf/apt/pacman`) is local to one shell step — adding the Arch and
Debian branches alongside the Fedora one is marginal extra work, and
the per-distro paths come straight from the upstream tool's docs.
Generalising the *core tools* the same way is a much larger project:
package-manager backends, firewall abstraction, distro-specific service
names. Until that lands, recipes can ship cross-distro value without
waiting for it.

**What it means for a recipe author:**

- Always provide the Fedora path. It will be tested first.
- Provide Arch and Debian paths when upstream supports them. If they
  don't, fail explicitly (`echo "..." >&2; exit 1`) rather than silently
  skipping.
- Don't assume any other lux tool works on the user's distro. Recipes
  should be self-contained: install + configure, not "install then call
  `manage_firewall`."

**What it means for a user on Arch/Debian:** recipes will mostly work;
asking the agent "is nginx running?" or "open port 8080" will fail until
the core tools grow per-distro backends.

### Principle: install the way the tool's own docs say to

**Do not uniformly install everything as a Flatpak.** The temptation is
real — one ID works across distros — but it has costs:

- **Sandbox redirection**: flatpak'd apps can't read host
  `~/.config/<app>/`, so configs must be written to
  `~/.var/app/<id>/config/<app>/`. Every recipe that configures a flatpak
  app needs this redirect. Schema grows to work around the sandbox.
- **Unofficial repackages**: Flathub listings for Zed, Ghostty, VSCodium,
  and many others are community repackages, not official builds. Version
  lag and provenance varies.
- **Feature gaps**: shell integration in flatpak'd terminals,
  GPU acceleration, native font rendering — all have historical friction.
- **Not the path users will find when they Google the tool.** The Zed
  docs' first recommendation is `curl https://zed.dev/install.sh | sh`,
  not Flathub. Recipes should match user expectations.

Instead, **each recipe installs using the method that tool's own
documentation recommends first**:

- **Zed** → official install script (`curl https://zed.dev/install.sh`)
- **Ghostty** → Fedora COPR `pgdev/ghostty`, Arch `ghostty` in extra,
  Debian/Ubuntu: declared unsupported until an official package exists
- **VSCodium** → VSCodium's own dnf/apt repos

Flatpak stays available as a cross-distro fallback in `PackageSpec.flatpak`
for tools where no better option exists (e.g. closed-source apps in
`GUI_APPS`). It is not the default for tools we can install natively.

### Principle: the LLM routes, it doesn't know install methods

A 1.7B local model is **exactly the wrong place** to store install
knowledge:

- It hallucinates URLs, repo names, and GPG fingerprints.
- Install methods churn; fine-tuning can't keep up. Weight updates are
  expensive and the knowledge goes stale.
- Weights are opaque — users can't audit "why does lux want me to `curl X
  | sh`?"

So: **install knowledge lives in YAML**, reviewable via `git diff` and
editable by anyone with a PR. The LLM's job is narrow:

1. Map natural-language input → recipe name via the intent matcher
   (pattern matching, no LLM call) or the general LLM fallback.
2. Compose multiple recipes for compound requests ("set up zsh and
   ghostty") — the agent emits two `apply_recipe` tool calls.
3. Fall back to `install_package` (generic) when no recipe exists —
   explicitly, not by bluffing.

This is the same pattern Homebrew, AUR, and nixpkgs use: human-curated
formulas, no AI in the install-knowledge path.

## Recipe schema

A recipe is a YAML file with the fields defined in
[`crates/lux-knowledge/src/recipe.rs`](../../crates/lux-knowledge/src/recipe.rs):

```yaml
name: zsh-popular                    # must match file stem
summary: "One-sentence description shown in list_recipes and apply_recipe plans"

packages:
  dnf: [zsh, git, curl]              # installed via `dnf install -y`
  apt: [zsh, git, curl]              # installed via `apt-get install -y`
  pacman: [zsh, git, curl]           # installed via `pacman -S --noconfirm --needed`
  flatpak: []                        # cross-distro fallback (avoid if native works)

steps:
  - shell: |                          # runs via `sh -c`; sudo is allowed
      command -v uv >/dev/null || curl -LsSf https://astral.sh/uv/install.sh | sh
    describe: Install uv              # short human description (appears in the plan)
    optional: true                    # optional steps log the failure and continue

  - write_file:                       # creates parent dirs, expands leading ~/
      path: ~/.zshrc
      content: |
        export ZSH="$HOME/.oh-my-zsh"
        ...
    describe: Write ~/.zshrc
```

Notes:

- **Step ordering**: `packages.flatpak` and `packages.<native>` always run
  first (in that order), then `steps` in the listed order.
- **Shell steps** run in a plain `sh -c` — they can use `sudo` and should
  guard against re-runs (e.g. `command -v X >/dev/null ||`, `test -d ... ||`).
- **Distro dispatch**: for tools where the install method differs per
  distro beyond a package name (COPR enablement, repo setup), the recipe
  uses `command -v dnf/apt/pacman` inside a single shell step. Keep each
  branch small; if logic grows, split into multiple steps with
  per-distro guards.
- **Unsupported distros**: fail cleanly with a clear message and non-zero
  exit. Silent skipping hides a broken recipe.

### Why YAML, not TOML or code

- Multi-line shell scripts are readable with `|` block scalars.
- Cross-distro package maps are more ergonomic than TOML's nested tables.
- YAML is the format users expect for Ansible/CI/Kubernetes recipes, so
  the learning curve is nearly zero.

## Confirmation UX

`apply_recipe` is a tool the LLM or intent matcher can invoke. Because
recipes install software and edit dotfiles, blind execution would be a
trust violation.

**Flow:**

1. Tool prints the plan to stderr (recipe name, summary, packages, steps).
2. If stdin is a TTY: prints `Apply this recipe? [y/N]` and blocks for input.
3. If stdin is not a TTY (scripted use): refuses unless
   `assume_yes: true` is passed explicitly.

The confirmation lives inside the tool, not the CLI, so scripted callers
and the REPL behave consistently. Moving it to a higher layer would
require a state machine in the (currently stateless) intent matcher.

## Intent matcher routing

`lux-agent/src/intent.rs::match_intent` tries recipe dispatch *before*
generic install, so "install zsh with popular plugins" picks
`zsh-popular` rather than interpreting the whole sentence as a package
list. Specifically:

- **`try_list_recipes`**: "list/what/which recipes" → `list_recipes`.
- **`try_recipe`**: direct (`apply the X recipe`), AI-dev-env heuristics,
  and per-tool patterns.
- **Compound requests** (multiple recipe-owned tools mentioned) fall
  through to the LLM, which emits several `apply_recipe` tool calls in
  one turn.
- **Tools owned by a recipe** are pruned from `GUI_APPS` so "install
  zed" / "install vscode" routes through the recipe's canonical path, not
  a Flathub repackage. (`install vscode` specifically becomes VSCodium —
  lux doesn't install Microsoft's VSCode.)

## Contributing a recipe

1. Decide: is this really ≥3 steps that always go together with a
   defensible opinion? If no, it belongs in `GUI_APPS` (simple install)
   or as a standalone tool (one-off command).
2. Author `crates/lux-knowledge/recipes/<name>.yaml`. The file stem must
   match the `name:` field.
3. Add the file to the `BUNDLED` array in
   `crates/lux-knowledge/src/registry.rs` (`include_str!` bakes it into
   the binary).
4. If the recipe should route from common phrasing, add a pattern in
   `crates/lux-agent/src/intent.rs::try_recipe` and a matching test.
5. For each supported distro: actually run the recipe end-to-end on a
   fresh install. Unit tests catch schema errors, not runtime ones.

## Future phases

- **v2 — user overrides**: load recipes from
  `~/.config/lux/recipes/*.yaml` in addition to bundled ones. Users can
  add team-internal or personal recipes without patching lux.
- **Later — signed remote registry**: a `lux-linux/recipes` repo that
  ships signed release bundles; `lux recipes update` syncs. Think `brew
  update` for formulas.
- **Later — vector RAG over docs**: the *original* idea for
  `lux-knowledge` was retrieval over man pages / distro wikis for
  *explanatory* queries ("why is my wifi flaky?"). This is a separate
  feature — a different data shape, different UX. Not tangled with
  recipes.

## How lux installs itself

The same principles apply to lux's own distribution:

- **Canonical**: `curl -fsSL ... | sh` installs a prebuilt binary to
  `~/.local/bin/lux` from GitHub releases. No sudo, includes the bundled
  `llama-server` for portable mode.
- **Native packages**: Fedora COPR `lux-linux/lux`, Arch AUR `lux-bin`,
  `.deb` on GitHub releases / later a proper apt repo.
- **Developers**: `cargo install lux-cli`.
- **Explicitly not**: Flatpak, Snap. lux is a system agent — it needs
  unsandboxed subprocess, filesystem, and network access to do its job.
  Sandboxing a system tool is a contradiction.

The lux installer should print its plan and wait for confirmation, the
same way `apply_recipe` does, so users running `curl | sh` see exactly
what will happen before it happens. Consistency with the recipes is
intentional: we teach users to expect a plan-and-confirm flow everywhere
lux touches their system.
