# libp2p swarm compatibility snapshot

Source: rust-libp2p 2d8497b2615086bd88018ef3a1fb32bc613a07ca, swarm 0.48.0.
All src files are byte-identical to this upstream revision. Cargo.toml expands workspace dependencies and omits upstream-only tests/bench targets.
The only dependency policy changes are wasm-bindgen-futures = 0.4 (instead of =0.4.58), and futures-timer >=3.0.4. Version 3.0.4 uses gloo-timers 0.4 and removes the reason for the upstream WASM cap. The application retains its existing WASM stub boundary.
Remove this snapshot when upstream relaxes the cap or a compatible registry release contains the rendezvous TTL fix; verify native and WASM consumers before replacement.
