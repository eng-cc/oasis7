use super::*;

pub(super) fn fragment_color(fragment: &FragmentTerrainPatch, lod: FragmentTerrainLod) -> Color {
    let alpha = fragment_alpha(fragment, lod);
    Color::srgba_u8(
        fragment.color[0],
        fragment.color[1],
        fragment.color[2],
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn fragment_inset_color(fragment: &FragmentTerrainPatch) -> Color {
    let alpha = fragment_alpha(fragment, FragmentTerrainLod::Detail);
    Color::srgba_u8(
        fragment.color[0].saturating_mul(3) / 5,
        fragment.color[1].saturating_mul(3) / 5,
        fragment.color[2].saturating_mul(3) / 5,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn fragment_fleck_color(fragment: &FragmentTerrainPatch) -> Color {
    let alpha = fragment_alpha(fragment, FragmentTerrainLod::Detail);
    let lighten = |channel: u8| channel.saturating_add((u8::MAX - channel) * 2 / 5);
    Color::srgba_u8(
        lighten(fragment.color[0]),
        lighten(fragment.color[1]),
        lighten(fragment.color[2]),
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn fragment_shadow_color(
    fragment: &FragmentTerrainPatch,
    lod: FragmentTerrainLod,
) -> Color {
    let alpha = fragment_alpha(fragment, lod) * f64::from(FRAGMENT_SHADOW_ALPHA_CAP);
    Color::srgba_u8(
        fragment.color[0].saturating_mul(2) / 5,
        fragment.color[1].saturating_mul(2) / 5,
        fragment.color[2].saturating_mul(2) / 5,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}
