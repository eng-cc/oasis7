use super::{Command, Libp2pNetwork};

impl Drop for Libp2pNetwork {
    fn drop(&mut self) {
        let _ = self.enqueue_command(Command::Shutdown);
    }
}
