use super::{Command, Libp2pNetwork};

impl Drop for Libp2pNetwork {
    fn drop(&mut self) {
        if std::sync::Arc::strong_count(&self.shutdown_guard) == 1 {
            let _ = self.enqueue_command(Command::Shutdown);
        }
    }
}
