#!/usr/bin/env python3
"""Build a 1 cm voxel model from a 2x3 six-view orthographic reference image.

The expected input layout is:

    front, back, left
    right, top, bottom

The pipeline uses silhouette carving. It does not infer hidden mechanical
details; it creates the largest voxel volume that satisfies all six masks.
"""

from __future__ import annotations

import argparse
import gzip
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from PIL import Image, ImageChops, ImageDraw, ImageFilter


VIEW_ORDER = ("front", "back", "left", "right", "top", "bottom")
PLAN_VIEW_ORDER = ("top", "bottom")
ASSET_SCHEMA_VERSION = 1
GENERATOR_NAME = "six_view_to_voxel_model.py"

COORDINATE_CONTRACT = {
    "grid_index_origin": "minimum x, y, z corner of the voxel grid",
    "world_origin": "centered on X/Z at ground plane; feet rest at y=0",
    "unit": "centimeter",
    "voxel_size_cm": 1,
    "axis_directions": {
        "+x": "right",
        "-x": "left",
        "+y": "top/up",
        "-y": "bottom/down",
        "+z": "front",
        "-z": "back",
    },
    "face_normals": {
        "front": [0, 0, 1],
        "back": [0, 0, -1],
        "left": [-1, 0, 0],
        "right": [1, 0, 0],
        "top": [0, 1, 0],
        "bottom": [0, -1, 0],
    },
    "grid_to_world_cm": {
        "x": "x - width_cm / 2",
        "y": "y",
        "z": "z - depth_cm / 2",
    },
}


@dataclass(frozen=True)
class MaskView:
    name: str
    mask: Image.Image
    bbox: tuple[int, int, int, int]

    @property
    def width(self) -> int:
        return self.bbox[2] - self.bbox[0]

    @property
    def height(self) -> int:
        return self.bbox[3] - self.bbox[1]


@dataclass(frozen=True)
class ColorView:
    name: str
    image: Image.Image


@dataclass(frozen=True)
class VoxelGrid:
    width_cm: int
    height_cm: int
    depth_cm: int
    voxels: set[tuple[int, int, int]]
    face_colors: dict[tuple[int, int, int], dict[str, tuple[int, int, int]]]


@dataclass(frozen=True)
class EvalConfig:
    view_weights: dict[str, float]
    objective: str
    coarse_grow_block_cm: int
    part_aware_growth: bool


def parse_view_weights(raw: str) -> dict[str, float]:
    weights = {view: 1.0 for view in VIEW_ORDER}
    if not raw.strip():
        return weights

    for item in raw.split(","):
        if not item.strip():
            continue
        if "=" not in item:
            raise ValueError(f"invalid --view-weight item {item!r}; expected view=value")
        view, value = item.split("=", 1)
        view = view.strip()
        if view not in VIEW_ORDER:
            raise ValueError(f"unknown view in --view-weight: {view!r}")
        parsed = float(value)
        if parsed < 0:
            raise ValueError(f"view weight must be non-negative: {item!r}")
        weights[view] = parsed
    if sum(weights.values()) <= 0:
        raise ValueError("--view-weight must leave at least one positive view weight")
    return weights


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Convert a 2x3 six-view robot image into a 1 cm voxel OBJ model."
    )
    parser.add_argument("--input", required=True, type=Path, help="2x3 six-view PNG/JPG.")
    parser.add_argument("--out-dir", required=True, type=Path, help="Output directory.")
    parser.add_argument(
        "--height-cm",
        type=int,
        default=200,
        help="Real-world model height in centimeters. 200 means a two-meter robot.",
    )
    parser.add_argument(
        "--mask-threshold",
        type=int,
        default=28,
        help="Foreground threshold for non-transparent/checkerboard inputs.",
    )
    parser.add_argument(
        "--alpha-threshold",
        type=int,
        default=24,
        help="Alpha threshold for transparent inputs.",
    )
    parser.add_argument(
        "--max-obj-voxels",
        type=int,
        default=600_000,
        help="Safety cap before writing OBJ surface mesh.",
    )
    parser.add_argument(
        "--color-bins",
        type=int,
        default=8,
        help="Per-channel color quantization bins for OBJ/MTL materials.",
    )
    parser.add_argument(
        "--min-view-votes",
        type=int,
        default=6,
        help="Minimum number of six orthographic masks a voxel must hit. 6 is strict carving.",
    )
    parser.add_argument(
        "--optimize-fit",
        action="store_true",
        help="Try several view-vote thresholds and keep the one with best projection IoU.",
    )
    parser.add_argument(
        "--disable-source-cleanup",
        action="store_true",
        help="Deprecated alias for --source-cleanup-policy off.",
    )
    parser.add_argument(
        "--source-cleanup-policy",
        choices=("robot", "off"),
        default="robot",
        help="Source cleanup policy. 'robot' removes likely plan-view generation artifacts.",
    )
    parser.add_argument(
        "--refine-iterations",
        type=int,
        default=0,
        help="Run this many residual-driven refinement rounds after initial carving.",
    )
    parser.add_argument(
        "--view-weight",
        default="front=1,back=1,left=1,right=1,top=0.45,bottom=0.45",
        help="Comma-separated fit weights, for example front=1,top=0.45.",
    )
    parser.add_argument(
        "--optimize-objective",
        choices=("balanced", "recall", "precision"),
        default="balanced",
        help="Scoring policy for threshold search and refinement candidate selection.",
    )
    parser.add_argument(
        "--coarse-grow-block-cm",
        type=int,
        default=4,
        help="Coarse block size for patch-level growth candidates inside refinement.",
    )
    parser.add_argument(
        "--disable-part-aware-growth",
        action="store_true",
        help="Disable vertical part-band quotas for refinement growth candidates.",
    )
    return parser.parse_args()


def split_six_grid(image: Image.Image) -> dict[str, Image.Image]:
    width, height = image.size
    if width < 6 or height < 4:
        raise ValueError(f"input is too small to be a 2x3 grid: {width}x{height}")
    if width % 3 != 0 or height % 2 != 0:
        raise ValueError(
            f"input dimensions must divide evenly into a 3x2 grid: {width}x{height}"
        )

    cell_w = width // 3
    cell_h = height // 2
    cells: dict[str, Image.Image] = {}
    for index, name in enumerate(VIEW_ORDER):
        col = index % 3
        row = index // 3
        left = col * cell_w
        upper = row * cell_h
        right = (col + 1) * cell_w
        lower = (row + 1) * cell_h
        cells[name] = image.crop((left, upper, right, lower))
    return cells


def checkerboard_score(pixel: tuple[int, int, int]) -> int:
    r, g, b = pixel
    return max(abs(r - g), abs(g - b), abs(r - b))


def foreground_mask(
    cell: Image.Image,
    *,
    alpha_threshold: int,
    mask_threshold: int,
) -> Image.Image:
    rgba = cell.convert("RGBA")
    alpha = rgba.getchannel("A")
    if alpha.getextrema()[0] < 250:
        mask = alpha.point(lambda value: 255 if value > alpha_threshold else 0)
    else:
        rgb = rgba.convert("RGB")
        blurred = rgb.filter(ImageFilter.GaussianBlur(10))
        diff = ImageChops.difference(rgb, blurred).convert("L")
        # Metal robots over checkerboard/white backgrounds have strong local
        # structure. The background has low local contrast and near-neutral RGB.
        mask = Image.new("L", rgb.size, 0)
        source = rgb.load()
        contrast = diff.load()
        target = mask.load()
        for y in range(rgb.height):
            for x in range(rgb.width):
                r, g, b = source[x, y]
                neutral = checkerboard_score((r, g, b)) < 5 and r > 180
                if not neutral and contrast[x, y] > mask_threshold:
                    target[x, y] = 255

    mask = mask.filter(ImageFilter.MaxFilter(5)).filter(ImageFilter.MinFilter(3))
    return mask


def likely_floor_pixel(pixel: tuple[int, int, int, int]) -> bool:
    r, g, b, a = pixel
    if a <= 16:
        return False
    max_channel = max(r, g, b)
    min_channel = min(r, g, b)
    return max_channel >= 220 and (max_channel - min_channel) <= 24


def connected_components(mask: Image.Image) -> list[list[tuple[int, int]]]:
    pixels = mask.load()
    seen: set[tuple[int, int]] = set()
    components: list[list[tuple[int, int]]] = []
    for y in range(mask.height):
        for x in range(mask.width):
            if pixels[x, y] == 0 or (x, y) in seen:
                continue
            stack = [(x, y)]
            seen.add((x, y))
            component: list[tuple[int, int]] = []
            while stack:
                px, py = stack.pop()
                component.append((px, py))
                for nx, ny in ((px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)):
                    if nx < 0 or ny < 0 or nx >= mask.width or ny >= mask.height:
                        continue
                    if (nx, ny) in seen or pixels[nx, ny] == 0:
                        continue
                    seen.add((nx, ny))
                    stack.append((nx, ny))
            components.append(component)
    return components


def remove_plan_view_floor_artifacts(
    cell: Image.Image,
    mask: Image.Image,
    view_name: str,
) -> tuple[Image.Image, dict[str, object]]:
    rgba = cell.convert("RGBA")
    contrast = ImageChops.difference(
        rgba.convert("RGB"),
        rgba.convert("RGB").filter(ImageFilter.GaussianBlur(4)),
    ).convert("L")
    floor_mask = Image.new("L", mask.size, 0)
    source = rgba.load()
    contrast_pixels = contrast.load()
    floor_pixels = floor_mask.load()
    original_pixels = mask.load()
    for y in range(mask.height):
        for x in range(mask.width):
            if (
                original_pixels[x, y] > 0
                and likely_floor_pixel(source[x, y])
                and contrast_pixels[x, y] < 10
            ):
                floor_pixels[x, y] = 255

    components = connected_components(floor_mask)
    cleaned = mask.copy()
    cleaned_pixels = cleaned.load()
    removed_pixels = 0
    removed_components = 0
    min_area = max(36, int(mask.width * mask.height * 0.002))
    for component in components:
        if len(component) < min_area:
            continue
        xs = [point[0] for point in component]
        ys = [point[1] for point in component]
        width = max(xs) - min(xs) + 1
        height = max(ys) - min(ys) + 1
        elongated = width > mask.width * 0.08 or height > mask.height * 0.08
        if not elongated:
            continue
        removed_components += 1
        for x, y in component:
            if cleaned_pixels[x, y] > 0:
                cleaned_pixels[x, y] = 0
                removed_pixels += 1

    before = sum(1 for value in mask.getdata() if value > 0)
    after = sum(1 for value in cleaned.getdata() if value > 0)
    return cleaned, {
        "view": view_name,
        "cleanup": "near_white_low_texture_component_removal",
        "source_pixels_before": before,
        "source_pixels_after": after,
        "removed_pixels": removed_pixels,
        "removed_fraction": round(removed_pixels / before, 6) if before else 0.0,
        "removed_components": removed_components,
        "status": "cleaned" if removed_pixels else "unchanged",
    }


def plan_support_mask(
    masks: dict[str, Image.Image],
    target_size: tuple[int, int],
    y_samples: int = 96,
) -> Image.Image:
    width, height = target_size
    support = Image.new("L", target_size, 0)
    pixels = support.load()

    for sample in range(y_samples):
        v_y = sample / max(1, y_samples - 1)
        x_counts: list[int] = []
        z_counts: list[int] = []
        for px in range(width):
            u_x = (px + 0.5) / width
            x_counts.append(
                int(mask_hit(masks["front"], u_x, v_y))
                + int(mask_hit(masks["back"], 1.0 - u_x, v_y))
            )
        for py in range(height):
            u_z = (py + 0.5) / height
            z_counts.append(
                int(mask_hit(masks["left"], u_z, v_y))
                + int(mask_hit(masks["right"], 1.0 - u_z, v_y))
            )

        x_at_least_one = [index for index, count in enumerate(x_counts) if count >= 1]
        x_both = [index for index, count in enumerate(x_counts) if count == 2]
        z_at_least_one = [index for index, count in enumerate(z_counts) if count >= 1]
        z_both = [index for index, count in enumerate(z_counts) if count == 2]

        for px in x_both:
            for py in z_at_least_one:
                pixels[px, py] = 255
        for px in x_at_least_one:
            for py in z_both:
                pixels[px, py] = 255
    return support.filter(ImageFilter.MaxFilter(13))


def apply_plan_geometric_cleanup(
    masks: dict[str, Image.Image],
    validation: dict[str, object],
) -> dict[str, Image.Image]:
    cleaned = dict(masks)
    for view in PLAN_VIEW_ORDER:
        support = plan_support_mask(masks, masks[view].size)
        before = sum(1 for value in masks[view].getdata() if value > 0)
        source_pixels = masks[view].load()
        support_pixels = support.load()
        output = masks[view].copy()
        output_pixels = output.load()
        removed = 0
        for y in range(output.height):
            for x in range(output.width):
                if source_pixels[x, y] > 0 and support_pixels[x, y] == 0:
                    output_pixels[x, y] = 0
                    removed += 1
        report = dict(validation["views"].get(view, {}))
        report["geometric_cleanup"] = "front_back_left_right_plan_support"
        report["geometric_removed_pixels"] = removed
        report["geometric_removed_fraction"] = round(removed / before, 6) if before else 0.0
        report["source_pixels_after_geometric"] = max(0, before - removed)
        if removed:
            report["status"] = "cleaned"
        validation["views"][view] = report
        cleaned[view] = output
    return cleaned


def crop_mask(mask: Image.Image, padding: int = 2) -> MaskView:
    bbox = mask.getbbox()
    if bbox is None:
        raise ValueError("view mask has no foreground")
    left = max(0, bbox[0] - padding)
    top = max(0, bbox[1] - padding)
    right = min(mask.width, bbox[2] + padding)
    bottom = min(mask.height, bbox[3] + padding)
    return MaskView("", mask.crop((left, top, right, bottom)), (left, top, right, bottom))


def build_masks(
    cells: dict[str, Image.Image],
    *,
    alpha_threshold: int,
    mask_threshold: int,
    cleanup_source: bool = True,
) -> tuple[dict[str, MaskView], dict[str, object]]:
    raw_masks: dict[str, Image.Image] = {}
    validation: dict[str, object] = {
        "source_cleanup_enabled": cleanup_source,
        "views": {},
    }
    for name, cell in cells.items():
        mask = foreground_mask(
            cell,
            alpha_threshold=alpha_threshold,
            mask_threshold=mask_threshold,
        )
        view_report: dict[str, object] = {
            "view": name,
            "cleanup": "none",
            "status": "not_applicable",
        }
        if cleanup_source and name in PLAN_VIEW_ORDER:
            mask, view_report = remove_plan_view_floor_artifacts(cell, mask, name)
        raw_masks[name] = mask
        validation["views"][name] = view_report

    if cleanup_source:
        raw_masks = apply_plan_geometric_cleanup(raw_masks, validation)

    masks: dict[str, MaskView] = {}
    for name, mask in raw_masks.items():
        cropped = crop_mask(mask)
        masks[name] = MaskView(name, cropped.mask, cropped.bbox)
    return masks, validation


def build_color_views(cells: dict[str, Image.Image], masks: dict[str, MaskView]) -> dict[str, ColorView]:
    views: dict[str, ColorView] = {}
    for name, cell in cells.items():
        views[name] = ColorView(name, cell.convert("RGBA").crop(masks[name].bbox))
    return views


def infer_dimensions_cm(masks: dict[str, MaskView], height_cm: int) -> tuple[int, int, int]:
    front_width = masks["front"].width / max(1, masks["front"].height) * height_cm
    back_width = masks["back"].width / max(1, masks["back"].height) * height_cm
    side_depth_l = masks["left"].width / max(1, masks["left"].height) * height_cm
    side_depth_r = masks["right"].width / max(1, masks["right"].height) * height_cm
    top_width = masks["top"].width / max(1, masks["top"].height)
    top_aspect = top_width if top_width > 0 else 1.0

    width_cm = max(1, round((front_width + back_width) / 2))
    depth_cm = max(1, round((side_depth_l + side_depth_r) / 2))

    # Top/bottom views can correct the side-derived depth ratio when present.
    top_depth_from_width = width_cm / top_aspect
    if math.isfinite(top_depth_from_width) and top_depth_from_width > 0:
        depth_cm = max(1, round((depth_cm * 2 + top_depth_from_width) / 3))

    return width_cm, height_cm, depth_cm


def mask_hit(mask: Image.Image, u: float, v: float) -> bool:
    x = min(mask.width - 1, max(0, round(u * (mask.width - 1))))
    y = min(mask.height - 1, max(0, round(v * (mask.height - 1))))
    return mask.getpixel((x, y)) > 0


def sample_rgb(view: ColorView, u: float, v: float) -> tuple[int, int, int] | None:
    x = min(view.image.width - 1, max(0, round(u * (view.image.width - 1))))
    y = min(view.image.height - 1, max(0, round(v * (view.image.height - 1))))
    r, g, b, a = view.image.getpixel((x, y))
    if a <= 16:
        return None
    return (r, g, b)


def average_colors(samples: list[tuple[int, int, int]]) -> tuple[int, int, int]:
    if not samples:
        return (128, 138, 148)
    return tuple(round(sum(sample[i] for sample in samples) / len(samples)) for i in range(3))


def voxel_face_colors(
    color_views: dict[str, ColorView],
    *,
    u_x: float,
    u_z: float,
    v_y: float,
) -> dict[str, tuple[int, int, int]]:
    samples = {
        "front": sample_rgb(color_views["front"], u_x, v_y),
        "back": sample_rgb(color_views["back"], 1.0 - u_x, v_y),
        "left": sample_rgb(color_views["left"], u_z, v_y),
        "right": sample_rgb(color_views["right"], 1.0 - u_z, v_y),
        "top": sample_rgb(color_views["top"], u_x, u_z),
        "bottom": sample_rgb(color_views["bottom"], u_x, 1.0 - u_z),
    }
    fallback = average_colors([sample for sample in samples.values() if sample is not None])
    return {face: sample if sample is not None else fallback for face, sample in samples.items()}


def voxel_average_color(grid: VoxelGrid, voxel: tuple[int, int, int]) -> tuple[int, int, int]:
    return average_colors(list(grid.face_colors[voxel].values()))


def carve_voxels(
    masks: dict[str, MaskView],
    color_views: dict[str, ColorView],
    height_cm: int,
    min_view_votes: int = 6,
) -> VoxelGrid:
    min_view_votes = max(1, min(6, min_view_votes))
    width_cm, height_cm, depth_cm = infer_dimensions_cm(masks, height_cm)
    voxels: set[tuple[int, int, int]] = set()
    face_colors: dict[tuple[int, int, int], dict[str, tuple[int, int, int]]] = {}

    for y in range(height_cm):
        v_y = 1.0 - ((y + 0.5) / height_cm)
        for x in range(width_cm):
            u_x = (x + 0.5) / width_cm
            for z in range(depth_cm):
                u_z = (z + 0.5) / depth_cm
                hits = (
                    mask_hit(masks["front"].mask, u_x, v_y),
                    mask_hit(masks["back"].mask, 1.0 - u_x, v_y),
                    mask_hit(masks["left"].mask, u_z, v_y),
                    mask_hit(masks["right"].mask, 1.0 - u_z, v_y),
                    mask_hit(masks["top"].mask, u_x, u_z),
                    mask_hit(masks["bottom"].mask, u_x, 1.0 - u_z),
                )
                if sum(1 for hit in hits if hit) < min_view_votes:
                    continue
                voxel = (x, y, z)
                voxels.add(voxel)
                face_colors[voxel] = voxel_face_colors(
                    color_views,
                    u_x=u_x,
                    u_z=u_z,
                    v_y=v_y,
                )

    return VoxelGrid(width_cm, height_cm, depth_cm, voxels, face_colors)


FACE_DELTAS = {
    "left": (-1, 0, 0),
    "right": (1, 0, 0),
    "bottom": (0, -1, 0),
    "top": (0, 1, 0),
    "back": (0, 0, -1),
    "front": (0, 0, 1),
}

FACE_CORNERS = {
    "left": ((0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)),
    "right": ((1, 0, 1), (1, 0, 0), (1, 1, 0), (1, 1, 1)),
    "bottom": ((0, 0, 1), (0, 0, 0), (1, 0, 0), (1, 0, 1)),
    "top": ((0, 1, 0), (0, 1, 1), (1, 1, 1), (1, 1, 0)),
    "back": ((1, 0, 0), (0, 0, 0), (0, 1, 0), (1, 1, 0)),
    "front": ((0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)),
}


def quantize_color(color: tuple[int, int, int], bins: int) -> tuple[int, int, int]:
    bins = max(2, bins)
    step = 255 / (bins - 1)
    return tuple(round(round(channel / step) * step) for channel in color)


def material_name(color: tuple[int, int, int]) -> str:
    return f"mat_{color[0]:03d}_{color[1]:03d}_{color[2]:03d}"


def write_mtl(path: Path, colors: Iterable[tuple[int, int, int]]) -> int:
    unique = sorted(set(colors))
    with path.open("w", encoding="utf-8") as handle:
        handle.write("# Quantized voxel colors generated by six_view_to_voxel_model.py\n")
        for color in unique:
            handle.write(f"newmtl {material_name(color)}\n")
            handle.write("Ka 0.000 0.000 0.000\n")
            handle.write(f"Kd {color[0] / 255:.6f} {color[1] / 255:.6f} {color[2] / 255:.6f}\n")
            handle.write("Ks 0.180 0.180 0.180\n")
            handle.write("Ns 32.000\n\n")
    return len(unique)


def write_obj(path: Path, grid: VoxelGrid, color_bins: int) -> tuple[int, int]:
    vertices: list[tuple[int, int, int]] = []
    faces: list[tuple[tuple[int, int, int, int], tuple[int, int, int]]] = []

    for x, y, z in sorted(grid.voxels):
        for face, delta in FACE_DELTAS.items():
            neighbor = (x + delta[0], y + delta[1], z + delta[2])
            if neighbor in grid.voxels:
                continue
            color = quantize_color(grid.face_colors[(x, y, z)][face], color_bins)
            indices: list[int] = []
            for corner in FACE_CORNERS[face]:
                vertices.append((x + corner[0], y + corner[1], z + corner[2]))
                indices.append(len(vertices))
            faces.append((tuple(indices), color))

    material_count = write_mtl(path.with_suffix(".mtl"), (color for _, color in faces))
    with path.open("w", encoding="utf-8") as handle:
        handle.write("# 1 unit = 1 centimeter; generated by six_view_to_voxel_model.py\n")
        handle.write(f"mtllib {path.with_suffix('.mtl').name}\n")
        handle.write("o robot_1cm_voxel_color_silhouette\n")
        for x, y, z in vertices:
            # Center the model on X/Z and put feet at Y=0.
            handle.write(
                f"v {x - grid.width_cm / 2:.3f} {y:.3f} {z - grid.depth_cm / 2:.3f}\n"
            )
        last_material = ""
        for face, color in faces:
            current_material = material_name(color)
            if current_material != last_material:
                handle.write(f"usemtl {current_material}\n")
                last_material = current_material
            handle.write(f"f {face[0]} {face[1]} {face[2]} {face[3]}\n")
    return len(faces), material_count


def write_voxels(path: Path, grid: VoxelGrid) -> None:
    payload = {
        "schema_version": ASSET_SCHEMA_VERSION,
        "generator": GENERATOR_NAME,
        "unit": "centimeter",
        "voxel_size_cm": 1,
        "coordinate_contract": COORDINATE_CONTRACT,
        "dimensions_cm": {
            "width": grid.width_cm,
            "height": grid.height_cm,
            "depth": grid.depth_cm,
        },
        "voxel_count": len(grid.voxels),
        "voxels": [
            {
                "x": x,
                "y": y,
                "z": z,
                "faces": {
                    face: list(grid.face_colors[(x, y, z)][face])
                    for face in VIEW_ORDER
                },
            }
            for x, y, z in sorted(grid.voxels)
        ],
    }
    data = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    with path.open("wb") as raw_handle:
        with gzip.GzipFile(fileobj=raw_handle, mode="wb", mtime=0) as gzip_handle:
            gzip_handle.write(data)


def shade_color(color: tuple[int, int, int], y: int, height_cm: int) -> tuple[int, int, int, int]:
    light = 0.72 + 0.34 * y / max(1, height_cm)
    return (
        min(255, round(color[0] * light)),
        min(255, round(color[1] * light)),
        min(255, round(color[2] * light)),
        220,
    )


def write_preview(path: Path, grid: VoxelGrid) -> None:
    scale = 4
    image = Image.new("RGBA", (1200, 900), (255, 255, 255, 0))
    draw = ImageDraw.Draw(image)
    origin_x = 590
    origin_y = 720
    step_x = 2.4
    step_z = 2.4
    step_y = 2.2

    visible = sorted(grid.voxels, key=lambda v: (v[0] + v[2], v[1]))
    stride = max(1, len(visible) // 55_000)
    for x, y, z in visible[::stride]:
        sx = origin_x + (x - grid.width_cm / 2) * step_x - (z - grid.depth_cm / 2) * step_z
        sy = origin_y - y * step_y + (x - grid.width_cm / 2) * 0.45 + (z - grid.depth_cm / 2) * 0.45
        color = shade_color(voxel_average_color(grid, (x, y, z)), y, grid.height_cm)
        draw.rectangle((sx, sy, sx + scale, sy + scale), fill=color)

    image = image.filter(ImageFilter.UnsharpMask(radius=1.0, percent=130, threshold=2))
    image.save(path)


def write_mask_debug(path: Path, masks: dict[str, MaskView]) -> None:
    cell_w = max(view.mask.width for view in masks.values())
    cell_h = max(view.mask.height for view in masks.values())
    image = Image.new("L", (cell_w * 3, cell_h * 2), 0)
    for index, name in enumerate(VIEW_ORDER):
        col = index % 3
        row = index // 3
        mask = masks[name].mask
        image.paste(mask, (col * cell_w, row * cell_h))
    image.save(path)


def write_source_validation(path: Path, validation: dict[str, object]) -> None:
    path.write_text(json.dumps(validation, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def voxel_projection_rect(
    *,
    view: str,
    voxel: tuple[int, int, int],
    grid: VoxelGrid,
    image_size: tuple[int, int],
) -> tuple[int, int, int, int]:
    x, y, z = voxel
    image_w, image_h = image_size

    if view in {"front", "back"}:
        if view == "front":
            u0, u1 = x / grid.width_cm, (x + 1) / grid.width_cm
        else:
            u0, u1 = 1.0 - (x + 1) / grid.width_cm, 1.0 - x / grid.width_cm
        v0 = 1.0 - (y + 1) / grid.height_cm
        v1 = 1.0 - y / grid.height_cm
    elif view in {"left", "right"}:
        if view == "left":
            u0, u1 = z / grid.depth_cm, (z + 1) / grid.depth_cm
        else:
            u0, u1 = 1.0 - (z + 1) / grid.depth_cm, 1.0 - z / grid.depth_cm
        v0 = 1.0 - (y + 1) / grid.height_cm
        v1 = 1.0 - y / grid.height_cm
    elif view == "top":
        u0, u1 = x / grid.width_cm, (x + 1) / grid.width_cm
        v0, v1 = z / grid.depth_cm, (z + 1) / grid.depth_cm
    elif view == "bottom":
        u0, u1 = x / grid.width_cm, (x + 1) / grid.width_cm
        v0, v1 = 1.0 - (z + 1) / grid.depth_cm, 1.0 - z / grid.depth_cm
    else:
        raise ValueError(view)

    px0 = max(0, min(image_w - 1, math.floor(u0 * image_w)))
    py0 = max(0, min(image_h - 1, math.floor(v0 * image_h)))
    px1 = max(0, min(image_w, math.ceil(u1 * image_w)))
    py1 = max(0, min(image_h, math.ceil(v1 * image_h)))
    return px0, py0, max(px0 + 1, px1), max(py0 + 1, py1)


def render_depth_key(view: str, voxel: tuple[int, int, int]) -> int:
    x, y, z = voxel
    if view == "front":
        return z
    if view == "back":
        return -z
    if view == "left":
        return -x
    if view == "right":
        return x
    if view == "top":
        return y
    if view == "bottom":
        return -y
    raise ValueError(view)


def render_projected_views(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
) -> dict[str, Image.Image]:
    rendered: dict[str, Image.Image] = {}
    for view in VIEW_ORDER:
        target_size = masks[view].mask.size
        image = Image.new("RGBA", target_size, (255, 255, 255, 0))
        draw = ImageDraw.Draw(image)
        for voxel in sorted(grid.voxels, key=lambda item: render_depth_key(view, item)):
            rect = voxel_projection_rect(view=view, voxel=voxel, grid=grid, image_size=target_size)
            color = (*grid.face_colors[voxel][view], 255)
            draw.rectangle((rect[0], rect[1], rect[2] - 1, rect[3] - 1), fill=color)
        rendered[view] = image
    return rendered


def render_mask_from_projection(image: Image.Image) -> Image.Image:
    return image.getchannel("A").point(lambda value: 255 if value > 0 else 0)


def mask_metrics(source: Image.Image, rendered: Image.Image) -> dict[str, float | int]:
    source_mask = source.point(lambda value: 255 if value > 0 else 0)
    rendered_mask = rendered.point(lambda value: 255 if value > 0 else 0)
    source_pixels = source_mask.load()
    rendered_pixels = rendered_mask.load()

    intersection = 0
    union = 0
    false_positive = 0
    false_negative = 0
    source_count = 0
    rendered_count = 0
    for y in range(source_mask.height):
        for x in range(source_mask.width):
            source_on = source_pixels[x, y] > 0
            rendered_on = rendered_pixels[x, y] > 0
            if source_on:
                source_count += 1
            if rendered_on:
                rendered_count += 1
            if source_on and rendered_on:
                intersection += 1
            if source_on or rendered_on:
                union += 1
            if rendered_on and not source_on:
                false_positive += 1
            if source_on and not rendered_on:
                false_negative += 1

    precision = intersection / rendered_count if rendered_count else 0.0
    recall = intersection / source_count if source_count else 0.0
    iou = intersection / union if union else 0.0
    return {
        "source_pixels": source_count,
        "rendered_pixels": rendered_count,
        "intersection_pixels": intersection,
        "union_pixels": union,
        "false_positive_pixels": false_positive,
        "false_negative_pixels": false_negative,
        "precision": round(precision, 6),
        "recall": round(recall, 6),
        "iou": round(iou, 6),
    }


def color_metrics(
    source: Image.Image,
    source_mask: Image.Image,
    rendered: Image.Image,
) -> dict[str, float | int]:
    source_rgba = source.convert("RGBA")
    rendered_rgba = rendered.convert("RGBA")
    source_pixels = source_rgba.load()
    mask_pixels = source_mask.point(lambda value: 255 if value > 0 else 0).load()
    rendered_pixels = rendered_rgba.load()
    compared = 0
    abs_sum = [0, 0, 0]
    sq_sum = [0, 0, 0]
    max_delta = 0

    for y in range(source_rgba.height):
        for x in range(source_rgba.width):
            sr, sg, sb, sa = source_pixels[x, y]
            rr, rg, rb, ra = rendered_pixels[x, y]
            if mask_pixels[x, y] == 0 or sa == 0 or ra == 0:
                continue
            compared += 1
            deltas = (abs(sr - rr), abs(sg - rg), abs(sb - rb))
            for index, delta in enumerate(deltas):
                abs_sum[index] += delta
                sq_sum[index] += delta * delta
            max_delta = max(max_delta, *deltas)

    if compared == 0:
        return {
            "compared_pixels": 0,
            "mae_rgb": [0.0, 0.0, 0.0],
            "mae_mean": 0.0,
            "rmse_rgb": [0.0, 0.0, 0.0],
            "rmse_mean": 0.0,
            "max_channel_delta": 0,
        }

    mae_rgb = [value / compared for value in abs_sum]
    rmse_rgb = [math.sqrt(value / compared) for value in sq_sum]
    return {
        "compared_pixels": compared,
        "mae_rgb": [round(value, 6) for value in mae_rgb],
        "mae_mean": round(sum(mae_rgb) / 3, 6),
        "rmse_rgb": [round(value, 6) for value in rmse_rgb],
        "rmse_mean": round(sum(rmse_rgb) / 3, 6),
        "max_channel_delta": max_delta,
    }


def make_color_error_cell(
    source: Image.Image,
    source_mask: Image.Image,
    rendered: Image.Image,
) -> Image.Image:
    source_rgba = source.convert("RGBA")
    rendered_rgba = rendered.convert("RGBA")
    cell = Image.new("RGBA", source.size, (255, 255, 255, 0))
    source_pixels = source_rgba.load()
    mask_pixels = source_mask.point(lambda value: 255 if value > 0 else 0).load()
    rendered_pixels = rendered_rgba.load()
    target = cell.load()
    for y in range(cell.height):
        for x in range(cell.width):
            sr, sg, sb, sa = source_pixels[x, y]
            rr, rg, rb, ra = rendered_pixels[x, y]
            if mask_pixels[x, y] == 0 or sa == 0 or ra == 0:
                continue
            error = (abs(sr - rr) + abs(sg - rg) + abs(sb - rb)) / (3 * 255)
            heat = min(255, round(error * 420))
            cool = max(0, 180 - heat)
            target[x, y] = (heat, cool, 255 - heat // 2, 215)
    return cell


def paste_six_grid(images: dict[str, Image.Image], mode: str = "RGBA") -> Image.Image:
    cell_w = max(image.width for image in images.values())
    cell_h = max(image.height for image in images.values())
    background = (255, 255, 255, 0) if mode == "RGBA" else 0
    canvas = Image.new(mode, (cell_w * 3, cell_h * 2), background)
    for index, view in enumerate(VIEW_ORDER):
        col = index % 3
        row = index // 3
        image = images[view]
        x = col * cell_w + (cell_w - image.width) // 2
        y = row * cell_h + (cell_h - image.height) // 2
        canvas.paste(image, (x, y), image if image.mode == "RGBA" else None)
    return canvas


def make_comparison_cell(source: Image.Image, rendered: Image.Image) -> Image.Image:
    source_mask = source.point(lambda value: 255 if value > 0 else 0)
    rendered_mask = render_mask_from_projection(rendered)
    cell = Image.new("RGBA", source_mask.size, (255, 255, 255, 0))
    pixels = cell.load()
    source_pixels = source_mask.load()
    rendered_pixels = rendered_mask.load()
    rendered_rgb = rendered.convert("RGB").load()
    for y in range(cell.height):
        for x in range(cell.width):
            source_on = source_pixels[x, y] > 0
            rendered_on = rendered_pixels[x, y] > 0
            if source_on and rendered_on:
                r, g, b = rendered_rgb[x, y]
                pixels[x, y] = (r, g, b, 230)
            elif source_on:
                pixels[x, y] = (255, 64, 64, 210)
            elif rendered_on:
                pixels[x, y] = (64, 132, 255, 210)
    return cell


def make_masked_source_cell(source: Image.Image, source_mask: Image.Image) -> Image.Image:
    source_rgba = source.convert("RGBA")
    mask = source_mask.point(lambda value: 255 if value > 0 else 0)
    cell = Image.new("RGBA", source_rgba.size, (255, 255, 255, 0))
    cell.paste(source_rgba, (0, 0), mask)
    return cell


def make_review_contact_sheet(
    sources: dict[str, Image.Image],
    rendered: dict[str, Image.Image],
    comparisons: dict[str, Image.Image],
    color_error: dict[str, Image.Image],
) -> Image.Image:
    rows = [
        paste_six_grid(sources, mode="RGBA"),
        paste_six_grid(rendered, mode="RGBA"),
        paste_six_grid(comparisons, mode="RGBA"),
        paste_six_grid(color_error, mode="RGBA"),
    ]
    width = max(row.width for row in rows)
    height = sum(row.height for row in rows)
    sheet = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    y = 0
    for row in rows:
        sheet.paste(row, ((width - row.width) // 2, y), row)
        y += row.height
    return sheet


def weighted_mean(
    per_view: dict[str, dict[str, object]],
    eval_config: EvalConfig,
    metric_group: str,
    metric_name: str,
) -> float:
    numerator = 0.0
    denominator = 0.0
    for view in VIEW_ORDER:
        weight = eval_config.view_weights.get(view, 1.0)
        if weight <= 0:
            continue
        numerator += weight * float(per_view[view][metric_group][metric_name])
        denominator += weight
    return numerator / denominator if denominator else 0.0


def score_fit(mean: dict[str, object], objective: str) -> float:
    iou = float(mean["iou"])
    precision = float(mean["precision"])
    recall = float(mean["recall"])
    color_mae = float(mean["color_mae"])
    color_penalty = 0.001 * color_mae
    if objective == "recall":
        return iou + 0.35 * recall + 0.05 * precision - color_penalty
    if objective == "precision":
        return iou + 0.35 * precision + 0.05 * recall - color_penalty
    return iou + 0.15 * recall + 0.10 * precision - color_penalty


def summarize_projection_fit(
    rendered: dict[str, Image.Image],
    masks: dict[str, MaskView],
    color_views: dict[str, ColorView],
    eval_config: EvalConfig,
) -> tuple[dict[str, object], dict[str, dict[str, object]]]:
    per_view = {
        view: {
            "shape": mask_metrics(masks[view].mask, render_mask_from_projection(rendered[view])),
            "color": color_metrics(color_views[view].image, masks[view].mask, rendered[view]),
        }
        for view in VIEW_ORDER
    }
    mean_iou = weighted_mean(per_view, eval_config, "shape", "iou")
    mean_precision = weighted_mean(per_view, eval_config, "shape", "precision")
    mean_recall = weighted_mean(per_view, eval_config, "shape", "recall")
    mean_color_mae = weighted_mean(per_view, eval_config, "color", "mae_mean")
    mean_color_rmse = weighted_mean(per_view, eval_config, "color", "rmse_mean")
    mean = {
        "iou": round(mean_iou, 6),
        "precision": round(mean_precision, 6),
        "recall": round(mean_recall, 6),
        "color_mae": round(mean_color_mae, 6),
        "color_rmse": round(mean_color_rmse, 6),
        "score": round(score_fit({
            "iou": mean_iou,
            "precision": mean_precision,
            "recall": mean_recall,
            "color_mae": mean_color_mae,
        }, eval_config.objective), 6),
    }
    return mean, per_view


def write_projection_evaluation(
    out_dir: Path,
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    color_views: dict[str, ColorView],
    eval_config: EvalConfig,
    *,
    prefix: str = "robot_1cm_projection",
) -> dict[str, object]:
    rendered = render_projected_views(grid, masks)
    render_grid = paste_six_grid(rendered, mode="RGBA")
    render_path = out_dir / f"{prefix}_render.png"
    render_grid.save(render_path)

    comparisons = {
        view: make_comparison_cell(masks[view].mask, rendered[view])
        for view in VIEW_ORDER
    }
    compare_path = out_dir / f"{prefix}_compare.png"
    paste_six_grid(comparisons, mode="RGBA").save(compare_path)

    color_error = {
        view: make_color_error_cell(color_views[view].image, masks[view].mask, rendered[view])
        for view in VIEW_ORDER
    }
    color_error_path = out_dir / (
        "robot_1cm_color_error.png"
        if prefix == "robot_1cm_projection"
        else f"{prefix}_color_error.png"
    )
    paste_six_grid(color_error, mode="RGBA").save(color_error_path)

    sources = {
        view: make_masked_source_cell(color_views[view].image, masks[view].mask)
        for view in VIEW_ORDER
    }
    contact_sheet_path = out_dir / (
        "robot_1cm_projection_contact_sheet.png"
        if prefix == "robot_1cm_projection"
        else f"{prefix}_contact_sheet.png"
    )
    make_review_contact_sheet(sources, rendered, comparisons, color_error).save(contact_sheet_path)

    mean, per_view = summarize_projection_fit(rendered, masks, color_views, eval_config)
    payload: dict[str, object] = {
        "render": str(render_path),
        "compare": str(compare_path),
        "color_error": str(color_error_path),
        "contact_sheet": str(contact_sheet_path),
        "evaluation": {
            "objective": eval_config.objective,
            "view_weights": eval_config.view_weights,
            "mean_metrics_are_weighted": True,
            "coarse_grow_block_cm": eval_config.coarse_grow_block_cm,
            "part_aware_growth": eval_config.part_aware_growth,
        },
        "legend": {
            "gray_or_color": "overlap between source mask and voxel re-render",
            "red": "source mask area missed by voxel projection",
            "blue": "voxel projection outside source mask",
            "color_error": "cool colors mean lower RGB error; hot colors mean higher RGB error on overlapping pixels",
        },
        "mean": mean,
        "views": per_view,
    }
    metrics_path = out_dir / (
        "robot_1cm_projection_metrics.json"
        if prefix == "robot_1cm_projection"
        else f"{prefix}_metrics.json"
    )
    metrics_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return payload


def evaluate_grid_fit(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    color_views: dict[str, ColorView],
    eval_config: EvalConfig,
) -> dict[str, object]:
    rendered = render_projected_views(grid, masks)
    mean, _ = summarize_projection_fit(rendered, masks, color_views, eval_config)
    return mean


def optimize_voxel_grid(
    masks: dict[str, MaskView],
    color_views: dict[str, ColorView],
    height_cm: int,
    max_obj_voxels: int,
    eval_config: EvalConfig,
) -> tuple[VoxelGrid, dict[str, object]]:
    candidates: list[dict[str, object]] = []
    best_grid: VoxelGrid | None = None
    best_score: float | None = None

    for min_view_votes in (6, 5, 4):
        grid = carve_voxels(masks, color_views, height_cm, min_view_votes=min_view_votes)
        if not grid.voxels:
            candidates.append({
                "min_view_votes": min_view_votes,
                "status": "empty",
                "voxel_count": 0,
            })
            continue
        if len(grid.voxels) > max_obj_voxels:
            candidates.append({
                "min_view_votes": min_view_votes,
                "status": "skipped_too_many_voxels",
                "voxel_count": len(grid.voxels),
            })
            continue

        mean = evaluate_grid_fit(grid, masks, color_views, eval_config)
        candidate = {
            "min_view_votes": min_view_votes,
            "status": "evaluated",
            "voxel_count": len(grid.voxels),
            "fit": mean,
        }
        candidates.append(candidate)
        score = score_fit(mean, eval_config.objective)
        if best_score is None or score > best_score:
            best_score = score
            best_grid = grid

    if best_grid is None:
        raise ValueError("fit optimization produced no usable voxel grid")

    selected = max(
        (candidate for candidate in candidates if candidate.get("status") == "evaluated"),
        key=lambda candidate: score_fit(candidate["fit"], eval_config.objective),
    )
    return best_grid, {
        "enabled": True,
        "objective": eval_config.objective,
        "view_weights": eval_config.view_weights,
        "selected_min_view_votes": selected["min_view_votes"],
        "selected_fit": selected["fit"],
        "candidates": candidates,
    }


def exposed_face_error_weight(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    voxel: tuple[int, int, int],
    eval_config: EvalConfig,
) -> float:
    error_weight = 0.0
    for face, delta in FACE_DELTAS.items():
        neighbor = (voxel[0] + delta[0], voxel[1] + delta[1], voxel[2] + delta[2])
        if neighbor in grid.voxels:
            continue
        mask = masks[face].mask
        rect = voxel_projection_rect(
            view=face,
            voxel=voxel,
            grid=grid,
            image_size=mask.size,
        )
        pixels = mask.load()
        outside = 0
        area = 0
        for py in range(rect[1], rect[3]):
            for px in range(rect[0], rect[2]):
                area += 1
                if pixels[px, py] == 0:
                    outside += 1
        if area:
            error_weight += eval_config.view_weights.get(face, 1.0) * (outside / area)
    return error_weight


def copy_grid_with_voxels(grid: VoxelGrid, voxels: set[tuple[int, int, int]]) -> VoxelGrid:
    return VoxelGrid(
        width_cm=grid.width_cm,
        height_cm=grid.height_cm,
        depth_cm=grid.depth_cm,
        voxels=voxels,
        face_colors={voxel: grid.face_colors[voxel] for voxel in voxels},
    )


def prune_bad_surface_voxels(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    eval_config: EvalConfig,
    threshold: float,
) -> tuple[VoxelGrid, int]:
    keep: set[tuple[int, int, int]] = set()
    removed = 0
    for voxel in grid.voxels:
        if exposed_face_error_weight(grid, masks, voxel, eval_config) >= threshold:
            removed += 1
            continue
        keep.add(voxel)
    return copy_grid_with_voxels(grid, keep), removed


def voxel_mask_hits(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    voxel: tuple[int, int, int],
) -> dict[str, bool]:
    x, y, z = voxel
    u_x = (x + 0.5) / grid.width_cm
    u_z = (z + 0.5) / grid.depth_cm
    v_y = 1.0 - ((y + 0.5) / grid.height_cm)
    return {
        "front": mask_hit(masks["front"].mask, u_x, v_y),
        "back": mask_hit(masks["back"].mask, 1.0 - u_x, v_y),
        "left": mask_hit(masks["left"].mask, u_z, v_y),
        "right": mask_hit(masks["right"].mask, 1.0 - u_z, v_y),
        "top": mask_hit(masks["top"].mask, u_x, u_z),
        "bottom": mask_hit(masks["bottom"].mask, u_x, 1.0 - u_z),
    }


def voxel_colors_for_grid(
    grid: VoxelGrid,
    color_views: dict[str, ColorView],
    voxel: tuple[int, int, int],
) -> dict[str, tuple[int, int, int]]:
    x, y, z = voxel
    return voxel_face_colors(
        color_views,
        u_x=(x + 0.5) / grid.width_cm,
        u_z=(z + 0.5) / grid.depth_cm,
        v_y=1.0 - ((y + 0.5) / grid.height_cm),
    )


def rank_boundary_growth_candidates(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    eval_config: EvalConfig,
) -> list[tuple[tuple[int, int, int], int, float, int]]:
    candidates: dict[tuple[int, int, int], tuple[int, float, int]] = {}
    for x, y, z in grid.voxels:
        for dx, dy, dz in FACE_DELTAS.values():
            candidate = (x + dx, y + dy, z + dz)
            cx, cy, cz = candidate
            if (
                candidate in grid.voxels
                or candidate in candidates
                or cx < 0
                or cy < 0
                or cz < 0
                or cx >= grid.width_cm
                or cy >= grid.height_cm
                or cz >= grid.depth_cm
            ):
                continue
            neighbor_count = sum(
                (cx + nx, cy + ny, cz + nz) in grid.voxels
                for nx, ny, nz in FACE_DELTAS.values()
            )
            hits = voxel_mask_hits(grid, masks, candidate)
            view_votes = sum(1 for hit in hits.values() if hit)
            missing_weight = sum(
                eval_config.view_weights.get(view, 1.0)
                for view, hit in hits.items()
                if not hit
            )
            candidates[candidate] = (view_votes, missing_weight, neighbor_count)

    return [
        (voxel, view_votes, missing_weight, neighbor_count)
        for voxel, (view_votes, missing_weight, neighbor_count) in sorted(
            candidates.items(),
            key=lambda item: (-item[1][0], item[1][1], -item[1][2], item[0]),
        )
    ]


def grow_supported_boundary_voxels(
    grid: VoxelGrid,
    color_views: dict[str, ColorView],
    ranked_candidates: list[tuple[tuple[int, int, int], int, float, int]],
    min_view_votes: int,
    max_missing_weight: float,
    min_neighbors: int,
    max_added_voxels: int,
) -> tuple[VoxelGrid, int]:
    limit = max(0, max_added_voxels)
    if limit == 0:
        return grid, 0
    selected = [
        voxel
        for voxel, view_votes, missing_weight, neighbor_count in ranked_candidates
        if view_votes >= min_view_votes
        and missing_weight <= max_missing_weight
        and neighbor_count >= min_neighbors
    ]
    selected = selected[:limit]
    if not selected:
        return grid, 0
    voxels = set(grid.voxels)
    voxels.update(selected)
    face_colors = dict(grid.face_colors)
    for voxel in selected:
        face_colors[voxel] = voxel_colors_for_grid(grid, color_views, voxel)
    return VoxelGrid(
        width_cm=grid.width_cm,
        height_cm=grid.height_cm,
        depth_cm=grid.depth_cm,
        voxels=voxels,
        face_colors=face_colors,
    ), len(selected)


def part_band_for_y(y: int, height_cm: int) -> str:
    normalized = y / max(1, height_cm - 1)
    if normalized < 0.16:
        return "feet"
    if normalized < 0.48:
        return "legs"
    if normalized < 0.72:
        return "torso"
    if normalized < 0.88:
        return "shoulders"
    return "head"


def allocate_by_part_bands(
    candidates: list[tuple[tuple[int, int, int], int, float, int]],
    *,
    height_cm: int,
    total_limit: int,
    enabled: bool,
) -> list[tuple[tuple[int, int, int], int, float, int]]:
    if not enabled or total_limit <= 0:
        return candidates[:max(0, total_limit)]

    bands = ("feet", "legs", "torso", "shoulders", "head")
    grouped = {band: [] for band in bands}
    for item in candidates:
        grouped[part_band_for_y(item[0][1], height_cm)].append(item)

    base_quota = max(1, total_limit // len(bands))
    selected: list[tuple[tuple[int, int, int], int, float, int]] = []
    selected_voxels: set[tuple[int, int, int]] = set()
    for band in bands:
        for item in grouped[band][:base_quota]:
            selected.append(item)
            selected_voxels.add(item[0])

    for item in candidates:
        if len(selected) >= total_limit:
            break
        if item[0] in selected_voxels:
            continue
        selected.append(item)
        selected_voxels.add(item[0])
    return selected


def grow_coarse_boundary_patches(
    grid: VoxelGrid,
    color_views: dict[str, ColorView],
    ranked_candidates: list[tuple[tuple[int, int, int], int, float, int]],
    *,
    min_view_votes: int,
    max_missing_weight: float,
    min_neighbors: int,
    max_added_voxels: int,
    block_cm: int,
    part_aware: bool,
) -> tuple[VoxelGrid, int]:
    block_cm = max(1, block_cm)
    eligible = [
        item for item in ranked_candidates
        if item[1] >= min_view_votes
        and item[2] <= max_missing_weight
        and item[3] >= min_neighbors
    ]
    if not eligible:
        return grid, 0

    blocks: dict[tuple[int, int, int], list[tuple[tuple[int, int, int], int, float, int]]] = {}
    for item in eligible:
        x, y, z = item[0]
        key = (x // block_cm, y // block_cm, z // block_cm)
        blocks.setdefault(key, []).append(item)

    ranked_blocks = sorted(
        blocks.values(),
        key=lambda block: (
            -len(block),
            -sum(item[1] for item in block) / len(block),
            sum(item[2] for item in block) / len(block),
            -sum(item[3] for item in block) / len(block),
        ),
    )
    flattened: list[tuple[tuple[int, int, int], int, float, int]] = []
    for block in ranked_blocks:
        flattened.extend(block)

    selected = allocate_by_part_bands(
        flattened,
        height_cm=grid.height_cm,
        total_limit=max_added_voxels,
        enabled=part_aware,
    )
    if not selected:
        return grid, 0

    voxels = set(grid.voxels)
    face_colors = dict(grid.face_colors)
    for voxel, _votes, _missing, _neighbors in selected:
        voxels.add(voxel)
        face_colors[voxel] = voxel_colors_for_grid(grid, color_views, voxel)

    return VoxelGrid(
        width_cm=grid.width_cm,
        height_cm=grid.height_cm,
        depth_cm=grid.depth_cm,
        voxels=voxels,
        face_colors=face_colors,
    ), len(selected)


def refine_voxel_grid(
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    color_views: dict[str, ColorView],
    eval_config: EvalConfig,
    iterations: int,
    out_dir: Path,
) -> tuple[VoxelGrid, list[dict[str, object]]]:
    history: list[dict[str, object]] = []
    current = grid
    current_fit = evaluate_grid_fit(current, masks, color_views, eval_config)
    current_score = score_fit(current_fit, eval_config.objective)
    prune_thresholds = (0.35, 0.75, 1.45)
    grow_policies = (
        {"mode": "fine", "min_view_votes": 5, "max_missing_weight": 0.45, "min_neighbors": 1, "max_added_voxels": 350},
        {"mode": "coarse_patch", "min_view_votes": 5, "max_missing_weight": 1.0, "min_neighbors": 2, "max_added_voxels": 1200},
    )

    for round_index in range(1, max(0, iterations) + 1):
        candidates: list[dict[str, object]] = []
        best_grid: VoxelGrid | None = None
        best_fit: dict[str, object] | None = None
        best_score = current_score
        ranked_growth_candidates = rank_boundary_growth_candidates(
            current,
            masks,
            eval_config,
        )

        for threshold in prune_thresholds:
            candidate_grid, removed = prune_bad_surface_voxels(
                current,
                masks,
                eval_config,
                threshold,
            )
            if not candidate_grid.voxels or removed == 0:
                candidates.append({
                    "operation": "prune",
                    "threshold": threshold,
                    "status": "unchanged",
                    "removed_voxels": removed,
                })
                continue
            candidate_fit = evaluate_grid_fit(candidate_grid, masks, color_views, eval_config)
            candidate_score = score_fit(candidate_fit, eval_config.objective)
            candidates.append({
                "operation": "prune",
                "threshold": threshold,
                "status": "evaluated",
                "removed_voxels": removed,
                "voxel_count": len(candidate_grid.voxels),
                "fit": candidate_fit,
                "score": round(candidate_score, 6),
            })
            if candidate_score > best_score:
                best_grid = candidate_grid
                best_fit = candidate_fit
                best_score = candidate_score

        for policy in grow_policies:
            if policy["mode"] == "coarse_patch":
                candidate_grid, added = grow_coarse_boundary_patches(
                    current,
                    color_views,
                    ranked_growth_candidates,
                    min_view_votes=int(policy["min_view_votes"]),
                    max_missing_weight=float(policy["max_missing_weight"]),
                    min_neighbors=int(policy["min_neighbors"]),
                    max_added_voxels=int(policy["max_added_voxels"]),
                    block_cm=eval_config.coarse_grow_block_cm,
                    part_aware=eval_config.part_aware_growth,
                )
            else:
                candidate_grid, added = grow_supported_boundary_voxels(
                    current,
                    color_views,
                    ranked_growth_candidates,
                    min_view_votes=int(policy["min_view_votes"]),
                    max_missing_weight=float(policy["max_missing_weight"]),
                    min_neighbors=int(policy["min_neighbors"]),
                    max_added_voxels=int(policy["max_added_voxels"]),
                )
            if added == 0:
                candidates.append({
                    "operation": "grow",
                    "mode": policy["mode"],
                    "status": "unchanged",
                    "added_voxels": 0,
                })
                continue
            candidate_fit = evaluate_grid_fit(candidate_grid, masks, color_views, eval_config)
            candidate_score = score_fit(candidate_fit, eval_config.objective)
            candidates.append({
                "operation": "grow",
                "mode": policy["mode"],
                "status": "evaluated",
                "added_voxels": added,
                "voxel_count": len(candidate_grid.voxels),
                "fit": candidate_fit,
                "score": round(candidate_score, 6),
                **policy,
            })
            if candidate_score > best_score:
                best_grid = candidate_grid
                best_fit = candidate_fit
                best_score = candidate_score

        if best_grid is None or best_fit is None:
            round_prefix = f"robot_1cm_refine_round_{round_index:02d}"
            round_metrics = write_projection_evaluation(
                out_dir,
                current,
                masks,
                color_views,
                eval_config,
                prefix=round_prefix,
            )
            history.append({
                "round": round_index,
                "accepted": False,
                "reason": "no_candidate_improved_objective",
                "fit": current_fit,
                "score": round(current_score, 6),
                "render": round_metrics["render"],
                "compare": round_metrics["compare"],
                "color_error": round_metrics["color_error"],
                "candidates": candidates,
            })
            break

        current = best_grid
        current_fit = best_fit
        current_score = best_score
        round_prefix = f"robot_1cm_refine_round_{round_index:02d}"
        round_metrics = write_projection_evaluation(
            out_dir,
            current,
            masks,
            color_views,
            eval_config,
            prefix=round_prefix,
        )
        history.append({
            "round": round_index,
            "accepted": True,
            "voxel_count": len(current.voxels),
            "fit": current_fit,
            "score": round(current_score, 6),
            "render": round_metrics["render"],
            "compare": round_metrics["compare"],
            "color_error": round_metrics["color_error"],
            "candidates": candidates,
        })

    return current, history


def write_metadata(
    path: Path,
    grid: VoxelGrid,
    masks: dict[str, MaskView],
    obj_faces: int,
    material_count: int,
    color_bins: int,
    projection_metrics: dict[str, object],
    optimization: dict[str, object] | None,
    source_validation: dict[str, object],
    eval_config: EvalConfig,
    refine_history: list[dict[str, object]],
) -> None:
    metadata = {
        "unit": "centimeter",
        "voxel_size_cm": 1,
        "schema_version": ASSET_SCHEMA_VERSION,
        "generator": GENERATOR_NAME,
        "coordinate_contract": COORDINATE_CONTRACT,
        "source_layout": ["front", "back", "left", "right", "top", "bottom"],
        "dimensions_cm": {
            "width": grid.width_cm,
            "height": grid.height_cm,
            "depth": grid.depth_cm,
        },
        "voxel_count": len(grid.voxels),
        "obj_surface_faces": obj_faces,
        "obj_material_count": material_count,
        "color": {
            "source": "per-face RGB sampled from matching orthographic source projection",
            "obj_material_quantization_bins_per_channel": color_bins,
            "voxel_json_format": "{x, y, z, faces: {front/back/left/right/top/bottom: [r, g, b]}}",
        },
        "projection_fit": {
            "shape": {
                "iou": projection_metrics["mean"]["iou"],
                "precision": projection_metrics["mean"]["precision"],
                "recall": projection_metrics["mean"]["recall"],
            },
            "color": {
                "mae": projection_metrics["mean"]["color_mae"],
                "rmse": projection_metrics["mean"]["color_rmse"],
            },
        },
        "evaluation": {
            "objective": eval_config.objective,
            "view_weights": eval_config.view_weights,
            "mean_metrics_are_weighted": True,
            "coarse_grow_block_cm": eval_config.coarse_grow_block_cm,
            "part_aware_growth": eval_config.part_aware_growth,
        },
        "refinement": {
            "enabled": bool(refine_history),
            "history": refine_history,
        },
        "optimization": optimization or {"enabled": False},
        "source_validation": source_validation,
        "mask_bboxes_px": {name: view.bbox for name, view in masks.items()},
        "method": "six-view orthographic silhouette carving",
        "limits": [
            "Hidden internal details are not reconstructed.",
            "Physical consistency depends on the six input views being mutually consistent.",
            "The OBJ exports only exposed voxel faces; each unit is one centimeter.",
            "Each voxel face is sampled from its matching source view; inconsistent views can create different colors on opposite faces.",
        ],
    }
    path.write_text(json.dumps(metadata, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def ensure_reasonable_grid(grid: VoxelGrid, max_obj_voxels: int) -> None:
    if not grid.voxels:
        raise ValueError("carving produced no voxels; check six-view layout and foreground masks")
    if len(grid.voxels) > max_obj_voxels:
        raise ValueError(
            f"carving produced {len(grid.voxels)} voxels, above --max-obj-voxels={max_obj_voxels}"
        )


def main() -> int:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    eval_config = EvalConfig(
        view_weights=parse_view_weights(args.view_weight),
        objective=args.optimize_objective,
        coarse_grow_block_cm=max(1, args.coarse_grow_block_cm),
        part_aware_growth=not args.disable_part_aware_growth,
    )

    image = Image.open(args.input)
    cells = split_six_grid(image)
    masks, source_validation = build_masks(
        cells,
        alpha_threshold=args.alpha_threshold,
        mask_threshold=args.mask_threshold,
        cleanup_source=args.source_cleanup_policy != "off" and not args.disable_source_cleanup,
    )
    source_validation["source_cleanup_policy"] = (
        "off" if args.disable_source_cleanup else args.source_cleanup_policy
    )
    color_views = build_color_views(cells, masks)
    if args.optimize_fit:
        grid, optimization = optimize_voxel_grid(
            masks,
            color_views,
            args.height_cm,
            args.max_obj_voxels,
            eval_config,
        )
    else:
        grid = carve_voxels(
            masks,
            color_views,
            args.height_cm,
            min_view_votes=args.min_view_votes,
        )
        optimization = {
            "enabled": False,
            "min_view_votes": max(1, min(6, args.min_view_votes)),
            "objective": eval_config.objective,
            "view_weights": eval_config.view_weights,
            "coarse_grow_block_cm": eval_config.coarse_grow_block_cm,
            "part_aware_growth": eval_config.part_aware_growth,
        }
    ensure_reasonable_grid(grid, args.max_obj_voxels)
    refine_history: list[dict[str, object]] = []
    if args.refine_iterations > 0:
        grid, refine_history = refine_voxel_grid(
            grid,
            masks,
            color_views,
            eval_config,
            args.refine_iterations,
            args.out_dir,
        )
        ensure_reasonable_grid(grid, args.max_obj_voxels)

    obj_path = args.out_dir / "robot_1cm_voxel.obj"
    mtl_path = args.out_dir / "robot_1cm_voxel.mtl"
    voxels_path = args.out_dir / "robot_1cm_voxels.json.gz"
    preview_path = args.out_dir / "robot_1cm_voxel_preview.png"
    mask_path = args.out_dir / "robot_1cm_mask_debug.png"
    source_validation_path = args.out_dir / "robot_1cm_source_validation.json"
    metadata_path = args.out_dir / "robot_1cm_voxel_metadata.json"
    refine_history_path = args.out_dir / "robot_1cm_refine_history.json"

    obj_faces, material_count = write_obj(obj_path, grid, args.color_bins)
    write_voxels(voxels_path, grid)
    write_preview(preview_path, grid)
    write_mask_debug(mask_path, masks)
    write_source_validation(source_validation_path, source_validation)
    if refine_history:
        refine_history_path.write_text(
            json.dumps({
                "objective": eval_config.objective,
                "view_weights": eval_config.view_weights,
                "coarse_grow_block_cm": eval_config.coarse_grow_block_cm,
                "part_aware_growth": eval_config.part_aware_growth,
                "history": refine_history,
            }, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
    projection_metrics = write_projection_evaluation(args.out_dir, grid, masks, color_views, eval_config)
    write_metadata(
        metadata_path,
        grid,
        masks,
        obj_faces,
        material_count,
        args.color_bins,
        projection_metrics,
        optimization,
        source_validation,
        eval_config,
        refine_history,
    )

    print(json.dumps({
        "obj": str(obj_path),
        "mtl": str(mtl_path),
        "voxels": str(voxels_path),
        "preview": str(preview_path),
        "metadata": str(metadata_path),
        "source_validation": str(source_validation_path),
        "projection_render": projection_metrics["render"],
        "projection_compare": projection_metrics["compare"],
        "projection_color_error": projection_metrics["color_error"],
        "projection_contact_sheet": projection_metrics["contact_sheet"],
        "projection_fit": projection_metrics["mean"],
        "optimization": optimization,
        "refinement": {
            "enabled": bool(refine_history),
            "history": str(refine_history_path) if refine_history else None,
            "rounds": refine_history,
        },
        "dimensions_cm": {
            "width": grid.width_cm,
            "height": grid.height_cm,
            "depth": grid.depth_cm,
        },
        "voxel_count": len(grid.voxels),
        "obj_surface_faces": obj_faces,
        "obj_material_count": material_count,
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
