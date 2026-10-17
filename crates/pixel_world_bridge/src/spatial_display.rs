use super::*;
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SpatialVisual {
    pub id: String,
    pub label: String,
    pub pos: Position,
    #[serde(default)]
    pub historical: bool,
    pub member_count: Option<String>,
}
pub(super) fn hash(hasher: &mut DefaultHasher, state: &RenderState, text: bool) {
    for visuals in [&state.industry_aggregates, &state.world_events] {
        visuals.len().hash(hasher);
        for visual in visuals {
            visual.id.hash(hasher);
            hash_position(hasher, &visual.pos);
            visual.historical.hash(hasher);
            if text {
                visual.label.hash(hasher);
                visual.member_count.hash(hasher);
            }
        }
    }
}
