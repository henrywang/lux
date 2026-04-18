# lux — portable bundle

This tarball contains everything needed to run lux on Linux x86_64, with no
system-wide install and no network access after extraction.

## Contents

```
lux        # interactive CLI
luxd       # background monitoring daemon (optional)
ollama     # local inference server
models/    # pre-pulled henrywang/lux weights
systemd/   # reference unit file for running luxd via systemd --user
```

## Run the CLI

```
./lux
```

On startup, lux notices the sibling `ollama` binary and `models/` directory and
auto-spawns a local inference server on an ephemeral port. The server is
terminated when lux exits. No files are written to `~/.ollama` or any system
location.

### Options

- `./lux -c "install htop"` — one-shot command, no REPL.
- `LUX_NO_PORTABLE=1 ./lux` — skip the sibling-ollama auto-spawn (useful when
  developing against a different server).
- `./lux --ollama-url http://localhost:11434` — target an explicit server;
  sibling-ollama auto-spawn is skipped.

## Run the daemon (optional)

`luxd` watches systemd units, journald, SELinux AVCs, and disk usage, then
writes findings that the REPL's `/findings` command surfaces. It does not use
the LLM.

Foreground:

```
./luxd
```

First run writes `~/.config/lux/luxd.toml`. Edit to change `mode`,
`interval_secs`, or which detectors are active.

### systemd user unit

If you want luxd to start at login on a system with systemd:

```
install -m 755 ./luxd ~/.local/bin/luxd
install -m 644 systemd/luxd.service ~/.config/systemd/user/luxd.service
systemctl --user daemon-reload
systemctl --user enable --now luxd.service
```

The unit is a user unit, not a system unit — it runs as you, not as root.

## Licensing

- `lux` / `luxd`: Apache-2.0 (see the source repository for the full license).
- `ollama`: MIT, bundled unmodified from https://ollama.com.
- `models/`: the `henrywang/lux` weights are redistributed under Apache-2.0,
  inherited from the Qwen3 base model.
