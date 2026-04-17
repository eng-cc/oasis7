use super::*;

pub(super) fn schedule_periodic_republish(mut command_tx: mpsc::Sender<Command>, interval_ms: i64) {
    if interval_ms <= 0 {
        return;
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(interval_ms as u64));
        match command_tx.try_send(Command::RepublishProviders) {
            Ok(()) => {}
            Err(err) if err.is_full() => {}
            Err(_) => break,
        }
    });
}

pub(super) fn schedule_periodic_discovery_refresh(
    mut command_tx: mpsc::Sender<Command>,
    interval_ms: i64,
) {
    if interval_ms <= 0 {
        return;
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(interval_ms as u64));
        match command_tx.try_send(Command::RefreshPeerDiscovery) {
            Ok(()) => {}
            Err(err) if err.is_full() => {}
            Err(_) => break,
        }
    });
}

pub(super) fn schedule_bootstrap_redial(
    mut command_tx: mpsc::Sender<Command>,
    peers: Vec<Multiaddr>,
    interval_ms: i64,
) {
    if interval_ms <= 0 || peers.is_empty() {
        return;
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(interval_ms as u64));
        for addr in &peers {
            match command_tx.try_send(Command::Dial(addr.clone())) {
                Ok(()) => {}
                Err(err) if err.is_full() => break,
                Err(_) => return,
            }
        }
    });
}

pub(super) fn enqueue_initial_bootstrap_dials(
    mut command_tx: mpsc::Sender<Command>,
    peers: Vec<Multiaddr>,
) {
    for addr in peers {
        // Best effort: if the background task exits, dial requests can be dropped.
        if command_tx.try_send(Command::Dial(addr)).is_err() {
            break;
        }
    }
}
