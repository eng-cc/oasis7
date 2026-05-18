from __future__ import annotations

import json
import pathlib
import re
from collections import OrderedDict

SAFE_SCALAR_RE = re.compile(r"[A-Za-z0-9_.:/+-]+")


def parse_scalar(value: str):
    value = value.strip()
    if value == "null":
        return None
    if value == "true":
        return True
    if value == "false":
        return False
    if value.startswith('"'):
        return json.loads(value)
    return value


def format_scalar(value) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    value = str(value)
    if SAFE_SCALAR_RE.fullmatch(value):
        return value
    return json.dumps(value, ensure_ascii=False)


def parse_key_value(text: str) -> tuple[str, str]:
    key, sep, value = text.partition(": ")
    if not sep:
        raise ValueError(f"invalid key/value line: {text!r}")
    return key, value


def load_list_document(
    path: pathlib.Path, list_key: str
) -> tuple[OrderedDict[str, object], list[OrderedDict[str, object]]]:
    header: OrderedDict[str, object] = OrderedDict()
    items: list[OrderedDict[str, object]] = []
    current: OrderedDict[str, object] | None = None
    active_list_key: str | None = None
    in_items = False

    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not raw_line.strip():
            continue
        if not in_items:
            if raw_line == f"{list_key}: []":
                return header, []
            if raw_line == f"{list_key}:":
                in_items = True
                continue
            if raw_line.startswith(" "):
                raise ValueError(f"{path}:{line_no}: unexpected indentation before {list_key}")
            if raw_line.endswith(": []"):
                key = raw_line[:-4]
                header[key] = []
                continue
            key, value = parse_key_value(raw_line)
            header[key] = parse_scalar(value)
            continue

        if raw_line.startswith("  - "):
            if current is not None:
                items.append(current)
            current = OrderedDict()
            key, value = parse_key_value(raw_line[4:])
            current[key] = parse_scalar(value)
            active_list_key = None
            continue

        if raw_line.startswith("      - "):
            if current is None or active_list_key is None:
                raise ValueError(f"{path}:{line_no}: dangling nested list item")
            value = parse_scalar(raw_line[8:])
            current[active_list_key].append(value)
            continue

        if raw_line.startswith("    "):
            if current is None:
                raise ValueError(f"{path}:{line_no}: item field before item start")
            stripped = raw_line[4:]
            if stripped.endswith(": []"):
                key = stripped[:-4]
                current[key] = []
                active_list_key = None
                continue
            if stripped.endswith(":"):
                key = stripped[:-1]
                current[key] = []
                active_list_key = key
                continue
            key, value = parse_key_value(stripped)
            current[key] = parse_scalar(value)
            active_list_key = None
            continue

        raise ValueError(f"{path}:{line_no}: unsupported line: {raw_line!r}")

    if in_items and current is not None:
        items.append(current)
    return header, items


def dump_list_document(
    path: pathlib.Path,
    header: OrderedDict[str, object],
    list_key: str,
    items: list[OrderedDict[str, object]],
) -> None:
    lines: list[str] = []
    for key, value in header.items():
        if isinstance(value, list):
            if value:
                raise ValueError(f"top-level lists other than {list_key} are not supported in {path}")
            lines.append(f"{key}: []")
        else:
            lines.append(f"{key}: {format_scalar(value)}")

    if not items:
        lines.append(f"{list_key}: []")
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return

    lines.append(f"{list_key}:")
    for item in items:
        first = True
        for key, value in item.items():
            prefix = "  - " if first else "    "
            if isinstance(value, list):
                if not value:
                    lines.append(f"{prefix}{key}: []")
                else:
                    lines.append(f"{prefix}{key}:")
                    for entry in value:
                        lines.append(f"      - {format_scalar(entry)}")
            else:
                lines.append(f"{prefix}{key}: {format_scalar(value)}")
            first = False

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def load_mapping_document(path: pathlib.Path) -> OrderedDict[str, object]:
    data: OrderedDict[str, object] = OrderedDict()
    active_list_key: str | None = None
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not raw_line.strip():
            continue
        if raw_line.startswith("  - "):
            if active_list_key is None:
                raise ValueError(f"{path}:{line_no}: list item without list key")
            data[active_list_key].append(parse_scalar(raw_line[4:]))
            continue
        if raw_line.startswith(" "):
            raise ValueError(f"{path}:{line_no}: unsupported indentation in mapping doc")
        if raw_line.endswith(": []"):
            data[raw_line[:-4]] = []
            active_list_key = None
            continue
        if raw_line.endswith(":"):
            key = raw_line[:-1]
            data[key] = []
            active_list_key = key
            continue
        key, value = parse_key_value(raw_line)
        data[key] = parse_scalar(value)
        active_list_key = None
    return data


def dump_mapping_document(path: pathlib.Path, data: OrderedDict[str, object]) -> None:
    lines: list[str] = []
    for key, value in data.items():
        if isinstance(value, list):
            if not value:
                lines.append(f"{key}: []")
            else:
                lines.append(f"{key}:")
                for entry in value:
                    lines.append(f"  - {format_scalar(entry)}")
        else:
            lines.append(f"{key}: {format_scalar(value)}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
