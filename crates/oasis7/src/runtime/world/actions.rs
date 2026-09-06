use super::super::{Action, ActionEnvelope, ActionId};
use super::World;

impl World {
    // ---------------------------------------------------------------------
    // Action submission
    // ---------------------------------------------------------------------

    pub fn submit_action(&mut self, action: Action) -> ActionId {
        let action_id = self.allocate_next_action_id();
        self.submit_action_with_id(action_id, action);
        action_id
    }

    /// Queue a World action under a previously allocated durable ID. Cognition
    /// commit finalization uses this to keep the receipt's action_id identical
    /// to the ActionEnvelope that the step actually executes.
    pub(super) fn submit_action_with_id(&mut self, action_id: ActionId, action: Action) {
        self.pending_actions.push_back(ActionEnvelope {
            id: action_id,
            action,
        });
        self.enforce_pending_action_limit();
    }

    pub fn pending_actions_len(&self) -> usize {
        self.pending_actions.len()
    }

    pub fn pending_effects_len(&self) -> usize {
        self.pending_effects.len()
    }
}
