use super::Command;

pub(super) struct ShutdownGuard {
    command_tx: futures::channel::mpsc::Sender<Command>,
}

impl ShutdownGuard {
    pub(super) fn new(command_tx: futures::channel::mpsc::Sender<Command>) -> Self {
        Self { command_tx }
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // `Arc` invokes its inner destructor exactly once, after the last owner has gone away;
        // unlike a sampled strong_count this cannot race another clone/drop.
        let _ = self.command_tx.try_send(Command::Shutdown);
    }
}
