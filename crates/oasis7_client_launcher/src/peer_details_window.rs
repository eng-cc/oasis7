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
            .default_size(Self::modal_window_size(ctx, 920.0, 640.0))
            .show(ctx, |ui| {
                Self::modal_header(
                    ui,
                    self.tr("P2P Peer 明细", "P2P Peer Details"),
                    self.tr(
                        "查看本地 peer、已连接 peer、路径和来源细节。",
                        "Inspect local peer, connected peers, path kind, and discovery details.",
                    ),
                    Some(if self.config.chain_enabled {
                        (
                            self.tr("链已启用", "Chain Enabled"),
                            egui::Color32::from_rgb(62, 152, 92),
                        )
                    } else {
                        (
                            self.tr("链已禁用", "Chain Disabled"),
                            egui::Color32::from_rgb(188, 60, 60),
                        )
                    }),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        Self::modal_card(ui, |ui| {
                            self.render_chain_peer_details_panel(ui, false);
                        });
                    });
            });

        self.peer_details_window_open = window_open;
    }
}
