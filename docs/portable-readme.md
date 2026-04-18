# lux — portable bundle

This tarball contains everything needed to run lux on Linux x86_64, with no
system-wide install and no network access after extraction.

**Distro support:** lux is built for the Fedora family (Fedora, RHEL, CentOS
Stream, and bootc images). The binary runs on any glibc Linux, but tools that
wrap `dnf`, `firewall-cmd`, or `bootc` will report the distro as unsupported
on Ubuntu/Debian/Arch until cross-distro backends land. The REPL, `/findings`,
flatpak installs, log reading, and service management work everywhere.

## Contents

```
lux             # interactive CLI
luxd            # background monitoring daemon (optional)
llama-server    # local inference server (llama.cpp)
libggml*.so     # shared libs loaded by llama-server ($ORIGIN rpath)
libllama*.so
libmtmd*.so
models/         # pre-downloaded Q4_K_M GGUF weights (henrywangxf/lux)
systemd/        # reference unit file for running luxd via systemd --user
```

## Run the CLI

```
./lux
```

On startup, lux notices the sibling `llama-server` binary and `models/*.gguf`
and auto-spawns a local inference server on an ephemeral port (with the
model's chat template enabled via `--jinja`). The server is terminated when
lux exits. No files are written outside this directory.

### Options

- `./lux -c "install htop"` — one-shot command, no REPL.
- `LUX_NO_PORTABLE=1 ./lux` — skip the sibling auto-spawn (useful when
  developing against a different server).
- `./lux --server-url http://localhost:11434` — target an explicit
  OpenAI-compatible server (Ollama, llama-server, etc.); sibling auto-spawn
  is skipped.

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
- `llama-server` + `libggml*`/`libllama*`/`libmtmd*`: MIT, bundled unmodified
  from https://github.com/ggml-org/llama.cpp.
- `models/`: the `henrywangxf/lux` weights are redistributed under Apache-2.0,
  inherited from the Qwen3 base model.
