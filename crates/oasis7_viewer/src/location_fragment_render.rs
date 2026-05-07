use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use oasis7::simulator::{
    infer_element_ppm, FragmentBlock, FragmentElementKind, FragmentPhysicalProfile,
};

use super::{BaseScale, SceneZoomLayer, Viewer3dAssets};

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(super) struct FragmentElementMarker {
    pub id: String,
    pub location_id: String,
    pub block_index: usize,
}

pub(super) fn spawn_location_fragment_elements(
    parent: &mut ChildSpawnerCommands,
    assets: &Viewer3dAssets,
    location_id: &str,
    radius_cm: i64,
    fragment_profile: &FragmentPhysicalProfile,
) {
    for (index, block) in fragment_profile.blocks.blocks.iter().enumerate() {
        let element = dominant_element_for_block(block).unwrap_or(FragmentElementKind::Silicon);
        let fragment_id = format!("{location_id}#{index}");
        let local_transform = fragment_block_local_transform(block, radius_cm);
        let base_scale = local_transform.scale;
        parent.spawn((
            Mesh3d(assets.agent_module_marker_mesh.clone()),
            MeshMaterial3d(assets.fragment_element_material_library.handle_for(element)),
            local_transform,
            BaseScale(base_scale),
            SceneZoomLayer::Detail,
            FragmentElementMarker {
                id: fragment_id,
                location_id: location_id.to_string(),
                block_index: index,
            },
            Name::new(format!(
                "location:fragment:block:{location_id}:{index}:{:?}",
                element
            )),
        ));
    }
}

fn dominant_element_for_block(block: &FragmentBlock) -> Option<FragmentElementKind> {
    let element_ppm = infer_element_ppm(&block.compounds);
    element_ppm
        .ppm
        .into_iter()
        .max_by_key(|(_, ppm)| *ppm)
        .map(|(element, _)| element)
}

fn fragment_block_local_transform(block: &FragmentBlock, radius_cm: i64) -> Transform {
    let radius = radius_cm.max(1) as f32;
    let radius_inv = 1.0 / radius;
    let center_x_cm = block.origin_cm.x_cm as f32 + block.size_cm.x_cm as f32 * 0.5 - radius;
    let center_y_cm = block.origin_cm.y_cm as f32 + block.size_cm.y_cm as f32 * 0.5 - radius;
    let center_z_cm = block.origin_cm.z_cm as f32 + block.size_cm.z_cm as f32 * 0.5 - radius;
    let scale_x = (block.size_cm.x_cm as f32 * radius_inv).max(0.01);
    let scale_y = (block.size_cm.y_cm as f32 * radius_inv).max(0.01);
    let scale_z = (block.size_cm.z_cm as f32 * radius_inv).max(0.01);

    Transform::from_translation(Vec3::new(
        center_x_cm * radius_inv,
        center_y_cm * radius_inv,
        center_z_cm * radius_inv,
    ))
    .with_scale(Vec3::new(scale_x, scale_y, scale_z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::simulator::{
        CompoundComposition, CuboidSizeCm, FragmentCompoundKind, FragmentElementKind, GridPosCm,
    };

    #[test]
    fn dominant_element_prefers_compound_signature_peak() {
        let mut compounds = CompoundComposition::new();
        compounds.set(FragmentCompoundKind::UraniumBearingOre, 700_000);
        compounds.set(FragmentCompoundKind::WaterIce, 300_000);

        let block = FragmentBlock {
            origin_cm: GridPosCm::new(0, 0, 0),
            size_cm: CuboidSizeCm {
                x_cm: 20,
                y_cm: 20,
                z_cm: 20,
            },
            density_kg_per_m3: 5000,
            compounds,
        };

        assert_eq!(
            dominant_element_for_block(&block),
            Some(FragmentElementKind::Uranium)
        );
    }

    #[test]
    fn block_local_transform_normalizes_to_location_radius() {
        let block = FragmentBlock {
            origin_cm: GridPosCm::new(0, 0, 0),
            size_cm: CuboidSizeCm {
                x_cm: 100,
                y_cm: 100,
                z_cm: 100,
            },
            density_kg_per_m3: 2800,
            compounds: CompoundComposition::new(),
        };

        let transform = fragment_block_local_transform(&block, 100);
        assert!((transform.translation.x + 0.5).abs() < 1e-6);
        assert!((transform.translation.y + 0.5).abs() < 1e-6);
        assert!((transform.translation.z + 0.5).abs() < 1e-6);
        assert!((transform.scale.x - 1.0).abs() < 1e-6);
        assert!((transform.scale.y - 1.0).abs() < 1e-6);
        assert!((transform.scale.z - 1.0).abs() < 1e-6);
    }
}
