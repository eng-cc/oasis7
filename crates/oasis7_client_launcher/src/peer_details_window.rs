use super::*;

impl ClientLauncherApp {
    pub(crate) fn show_peer_details_window(&mut self, ctx: &egui::Context) {
        if !self.peer_details_window_open {
            return;
        }

        let mut window_open = self.peer_details_window_open;
        egui::Window::new(self.tr("P2P Peer 明细", "P2P Peer Details"))
            .open(&mut window_open)
            .resizable(true)
            .default_size(egui::vec2(980.0, 620.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_chain_peer_details_panel(ui, true);
                    });
            });

        self.peer_details_window_open = window_open;
    }
}
