#!/usr/bin/env python3
from __future__ import annotations

import gzip
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "tools" / "voxel_asset_pipeline" / "six_view_to_voxel_model.py"
SPEC = importlib.util.spec_from_file_location("six_view_to_voxel_model", SCRIPT)
assert SPEC and SPEC.loader
PIPELINE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = PIPELINE
SPEC.loader.exec_module(PIPELINE)


def make_fixture(path: Path) -> None:
    cell_w = 90
    cell_h = 120
    image = Image.new("RGBA", (cell_w * 3, cell_h * 2), (255, 255, 255, 0))
    draw = ImageDraw.Draw(image)
    # front/back: 40 cm wide by 80 cm high silhouette
    for col, row in [(0, 0), (1, 0)]:
        x = col * cell_w
        y = row * cell_h
        draw.rectangle((x + 25, y + 20, x + 65, y + 100), fill=(210, 40, 30, 255))
    # left/right: 20 cm deep by 80 cm high silhouette
    for col, row in [(2, 0), (0, 1)]:
        x = col * cell_w
        y = row * cell_h
        draw.rectangle((x + 35, y + 20, x + 55, y + 100), fill=(40, 120, 220, 255))
    # top/bottom: 40 cm wide by 20 cm deep silhouette
    for col, row in [(1, 1), (2, 1)]:
        x = col * cell_w
        y = row * cell_h
        draw.rectangle((x + 25, y + 50, x + 65, y + 70), fill=(230, 190, 50, 255))
    image.save(path)


def main() -> int:
    out_dir = ROOT / "target" / "voxel_asset_pipeline_test"
    out_dir.mkdir(parents=True, exist_ok=True)
    fixture = out_dir / "six_view_fixture.png"
    make_fixture(fixture)

    bad_grid = Image.new("RGBA", (271, 240), (255, 255, 255, 0))
    try:
        PIPELINE.split_six_grid(bad_grid)
        raise AssertionError("split_six_grid accepted a non-divisible 3x2 grid")
    except ValueError as exc:
        assert "3x2" in str(exc)

    command = [
            sys.executable,
            str(SCRIPT),
            "--input",
            str(fixture),
            "--out-dir",
            str(out_dir / "result"),
            "--height-cm",
            "80",
            "--refine-iterations",
            "1",
        ]
    result = subprocess.run(
        command,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    payload = json.loads(result.stdout)
    assert payload["dimensions_cm"]["height"] == 80
    assert 34 <= payload["dimensions_cm"]["width"] <= 46
    assert 14 <= payload["dimensions_cm"]["depth"] <= 26
    assert payload["voxel_count"] > 0
    assert Path(payload["obj"]).exists()
    assert Path(payload["mtl"]).exists()
    assert Path(payload["metadata"]).exists()
    assert Path(payload["source_validation"]).exists()
    assert Path(payload["preview"]).exists()
    assert Path(payload["projection_render"]).exists()
    assert Path(payload["projection_compare"]).exists()
    assert Path(payload["projection_color_error"]).exists()
    assert Path(payload["projection_contact_sheet"]).exists()
    assert payload["projection_fit"]["score"] > 0.0
    assert payload["refinement"]["enabled"] is True
    assert Path(payload["refinement"]["history"]).exists()
    assert payload["projection_fit"]["iou"] > 0.90
    assert payload["projection_fit"]["color_mae"] >= 0.0
    assert payload["projection_fit"]["color_rmse"] >= 0.0
    metadata = json.loads(Path(payload["metadata"]).read_text(encoding="utf-8"))
    assert metadata["obj_material_count"] > 0
    assert metadata["source_validation"]["source_cleanup_enabled"] is True
    assert metadata["source_validation"]["source_cleanup_policy"] == "robot"
    assert "faces:" in metadata["color"]["voxel_json_format"]
    assert metadata["projection_fit"]["shape"]["iou"] > 0.90
    assert metadata["projection_fit"]["color"]["mae"] >= 0.0
    assert metadata["evaluation"]["objective"] == "balanced"
    assert metadata["evaluation"]["coarse_grow_block_cm"] >= 1
    assert metadata["evaluation"]["part_aware_growth"] is True
    assert metadata["refinement"]["enabled"] is True

    with gzip.open(payload["voxels"], "rt", encoding="utf-8") as handle:
        voxel_payload = json.load(handle)
    assert voxel_payload["schema_version"] == 1
    assert voxel_payload["generator"] == "six_view_to_voxel_model.py"
    assert voxel_payload["coordinate_contract"]["axis_directions"]["+z"] == "front"
    assert voxel_payload["coordinate_contract"]["grid_to_world_cm"]["x"] == "x - width_cm / 2"
    assert voxel_payload["voxel_count"] == payload["voxel_count"]
    first_voxel = voxel_payload["voxels"][0]
    assert set(first_voxel["faces"]) == {"front", "back", "left", "right", "top", "bottom"}
    for color in first_voxel["faces"].values():
        assert len(color) == 3
        assert all(0 <= channel <= 255 for channel in color)

    repeat_command = list(command)
    repeat_command[repeat_command.index(str(out_dir / "result"))] = str(out_dir / "result_repeat")
    repeat = subprocess.run(
        repeat_command,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    repeat_payload = json.loads(repeat.stdout)
    assert Path(payload["voxels"]).read_bytes() == Path(repeat_payload["voxels"]).read_bytes()

    source = Image.new("RGBA", (2, 1), (255, 255, 255, 255))
    source.putpixel((0, 0), (10, 20, 30, 255))
    source.putpixel((1, 0), (240, 240, 240, 255))
    source_mask = Image.new("L", (2, 1), 0)
    source_mask.putpixel((0, 0), 255)
    rendered = Image.new("RGBA", (2, 1), (0, 0, 0, 0))
    rendered.putpixel((0, 0), (10, 20, 30, 255))
    rendered.putpixel((1, 0), (0, 0, 0, 255))
    color_fit = PIPELINE.color_metrics(source, source_mask, rendered)
    assert color_fit["compared_pixels"] == 1
    assert color_fit["mae_mean"] == 0.0

    face_colors = {face: (10, 20, 30) for face in PIPELINE.VIEW_ORDER}
    small_grid = PIPELINE.VoxelGrid(
        width_cm=2,
        height_cm=2,
        depth_cm=2,
        voxels={(0, 0, 0)},
        face_colors={(0, 0, 0): face_colors},
    )
    grown_grid, added = PIPELINE.grow_supported_boundary_voxels(
        small_grid,
        {},
        [((1, 0, 0), 6, 0.0, 1)],
        min_view_votes=1,
        max_missing_weight=1.0,
        min_neighbors=1,
        max_added_voxels=0,
    )
    assert added == 0
    assert grown_grid.voxels == small_grid.voxels
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
