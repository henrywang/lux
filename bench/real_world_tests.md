# Real-world test log

Running lux on Xiaofeng's Fedora 43 machine. Each failure becomes a candidate dataset entry.

## Test categories to cover

- [ ] Package: install CLI tool (vim, htop, neovim)
- [ ] Package: install GUI (firefox, vlc, vscode)
- [ ] Package: remove (uninstall old tool)
- [ ] Service: status (is nginx running?)
- [ ] Service: start/stop/restart/enable
- [ ] Logs: show recent errors, filter by service
- [ ] Disk: which partition is full?
- [ ] Network: wifi issues, DNS, connectivity
- [ ] Firewall: open port, block IP
- [ ] Bootc: status, rollback (if on bootc system)
- [ ] Ambiguous: "my machine is slow", "something is broken"

## Failures

| # | Input | Expected | Got | Category | Notes |
|---|-------|----------|-----|----------|-------|
| 1 | install htop | package installed | sudo required error | tool impl | dnf needs sudo; applies to install/remove/service/firewall tools |
| 2 | something is wrong with my machine | read_logs(priority=err) | read_logs(priority=info) | model | vague symptom → err is more useful than info |
| 3 | clean up disk space | run_command(cleanup cmds) | check_disk_usage | intent+model | intent matcher false-positive on "disk space"; need "clean/free" guard |
| 4 | show me what failed to start at boot | run_command(systemctl --failed) or read_logs(boot=true) | read_logs(info, 100 lines) | model+tool | read_logs has no boot filter; model doesn't know boot-specific queries |
| 5 | block IP 192.168.1.100 | manage_firewall(action=block, source=IP) | manage_firewall(action=deny, port=IP) "success" | tool | no IP/source field; deny wrongly maps to --remove (removes allow rule); needs rich rule for IP blocking |
| 6 | update all packages | run_command(dnf upgrade) or new update_system tool | install_package(["update"]) | tool+model | no system-update tool exists; model defaulted to install_package |

## Surprises (not failures, but interesting)

- 
