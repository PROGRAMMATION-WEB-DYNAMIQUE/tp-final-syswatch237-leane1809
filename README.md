# SysWatch

Projet Rust - Moniteur système en réseau.

## Lancer
```bash
cargo build
cargo run
```

Le serveur écoute sur `127.0.0.1:7878` (ou IP machine) port `7878`.

## Tester
Dans un autre terminal:
```bash
telnet localhost 7878
```
ou
```bash
nc localhost 7878
```

Commandes:
- cpu
- mem
- ps
- all
- help
- quit
