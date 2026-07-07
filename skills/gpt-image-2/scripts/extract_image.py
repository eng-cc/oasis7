#!/usr/bin/env python3
"""Extract a generated image from a Codex CLI session rollout JSONL."""

from __future__ import annotations

import base64
import json
import pathlib
import urllib.parse
import urllib.request
import re
import sys

IMAGE_MAGIC_PREFIXES: dict[str, str] = {
    "iVBORw0KGgo": "png",
    "/9j/": "jpg",
    "UklGR": "webp",
}

MIN_BLOB_LENGTH = 200
BASE64_BLOB_PATTERN = re.compile(r'"([A-Za-z0-9+/=]{' + str(MIN_BLOB_LENGTH) + r',})"')
DATA_URL_PATTERN = re.compile(
    r"data:image/(?P<fmt>png|jpeg|jpg|webp|bmp)"
    r"(?P<encoding>;base64)?"
    r",(?P<data>[^)\"]+)",
    re.IGNORECASE,
)
REMOTE_IMAGE_URL_PATTERN = re.compile(
    r"https?://[^\s)\"']+",
    re.IGNORECASE,
)
MARKDOWN_IMAGE_URL_PATTERN = re.compile(
    r"!\[[^\]]*\]\((https?://[^)]+)\)",
    re.IGNORECASE,
)
ALLOWED_REMOTE_IMAGE_HOSTS: frozenset[str] = frozenset({"image.pollinations.ai"})
ALLOWED_REMOTE_IMAGE_SCHEMES: frozenset[str] = frozenset({"https"})
MAX_REMOTE_IMAGE_BYTES = 20 * 1024 * 1024
REMOTE_READ_CHUNK_BYTES = 64 * 1024


def _is_allowed_remote_image_url(url: str) -> bool:
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme.lower() not in ALLOWED_REMOTE_IMAGE_SCHEMES:
        return False
    hostname = (parsed.hostname or "").lower()
    return hostname in ALLOWED_REMOTE_IMAGE_HOSTS


def _download_remote_image(url: str) -> tuple[bytes, str] | None:
    if not _is_allowed_remote_image_url(url):
        return None

    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": (
                "Mozilla/5.0 (X11; Linux x86_64) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/136.0.0.0 Safari/537.36"
            )
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            if not _is_allowed_remote_image_url(response.geturl()):
                return None
            content_type = response.headers.get_content_type().lower()
            if not content_type.startswith("image/"):
                return None
            chunks: list[bytes] = []
            total_bytes = 0
            while True:
                chunk = response.read(REMOTE_READ_CHUNK_BYTES)
                if not chunk:
                    break
                total_bytes += len(chunk)
                if total_bytes > MAX_REMOTE_IMAGE_BYTES:
                    return None
                chunks.append(chunk)
            image_bytes = b"".join(chunks)
    except Exception:
        return None

    fmt = content_type.split("/", 1)[1]
    ext = "jpg" if fmt == "jpeg" else fmt
    return image_bytes, ext


def find_best_image_blob(session_paths: list[pathlib.Path]) -> tuple[bytes, str] | None:
    """Return the largest image payload found across given files."""
    best: tuple[bytes, str, int] | None = None

    def consider_image(image_bytes: bytes, ext: str) -> None:
        nonlocal best
        if best is None or len(image_bytes) > best[2]:
            best = (image_bytes, ext, len(image_bytes))

    for session_path in session_paths:
        try:
            text = session_path.read_text(errors="replace")
        except OSError:
            continue

        scan_texts = [text]
        for line in text.splitlines():
            try:
                obj = json.loads(line)
            except ValueError:
                continue
            scan_texts.append(json.dumps(obj))

        for flat in scan_texts:
            for match in DATA_URL_PATTERN.finditer(flat):
                fmt = match.group("fmt").lower()
                payload = match.group("data")
                encoding = (match.group("encoding") or "").lower()
                try:
                    if encoding == ";base64":
                        image_bytes = base64.b64decode(payload)
                    else:
                        image_bytes = urllib.parse.unquote_to_bytes(payload)
                except (ValueError, base64.binascii.Error):
                    continue
                ext = "jpg" if fmt == "jpeg" else fmt
                consider_image(image_bytes, ext)

            for match in BASE64_BLOB_PATTERN.finditer(flat):
                blob = match.group(1)
                for magic, ext in IMAGE_MAGIC_PREFIXES.items():
                    if blob.startswith(magic):
                        try:
                            image_bytes = base64.b64decode(blob)
                        except (ValueError, base64.binascii.Error):
                            break
                        consider_image(image_bytes, ext)
                        break

            url_candidates = MARKDOWN_IMAGE_URL_PATTERN.findall(flat)
            url_candidates.extend(REMOTE_IMAGE_URL_PATTERN.findall(flat))
            for url in url_candidates:
                downloaded = _download_remote_image(url)
                if downloaded is None:
                    continue
                image_bytes, ext = downloaded
                consider_image(image_bytes, ext)

    if best is None:
        return None
    return best[0], best[1]


ALLOWED_OUTPUT_EXTENSIONS: frozenset[str] = frozenset({".png", ".jpg", ".jpeg", ".webp", ".bmp"})

FORBIDDEN_OUTPUT_PREFIXES: tuple[str, ...] = (
    "/bin", "/boot", "/dev", "/etc", "/lib", "/proc",
    "/sbin", "/sys", "/usr", "/System", "/Library",
    "/var/root", "/var/log", "/var/db",
)


def validate_output_path(raw_out: str) -> pathlib.Path:
    """Canonicalise the output path; reject non-image extensions and system dirs."""
    candidate = pathlib.Path(raw_out)
    ext = candidate.suffix.lower()
    if ext not in ALLOWED_OUTPUT_EXTENSIONS:
        raise ValueError(
            f"output path must end in one of {sorted(ALLOWED_OUTPUT_EXTENSIONS)}; got {ext!r}"
        )

    resolved = candidate.expanduser().resolve()
    resolved_str = str(resolved)
    alt_str = (
        resolved_str[len("/private"):] if resolved_str.startswith("/private/") else None
    )

    def _is_under_forbidden(path_str: str) -> bool:
        return any(
            path_str == f or path_str.startswith(f + "/")
            for f in FORBIDDEN_OUTPUT_PREFIXES
        )

    if _is_under_forbidden(resolved_str) or (alt_str and _is_under_forbidden(alt_str)):
        raise ValueError(f"refusing to write under a system directory: {resolved}")

    return resolved


def choose_output_path(requested_path: pathlib.Path, detected_ext: str) -> pathlib.Path:
    detected_suffix = f".{detected_ext.lower()}"
    requested_suffix = requested_path.suffix.lower()
    if requested_suffix == detected_suffix:
        return requested_path
    if requested_suffix in {".jpg", ".jpeg"} and detected_suffix in {".jpg", ".jpeg"}:
        return requested_path
    return requested_path.with_suffix(detected_suffix)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(
            "usage: extract_image.py <out_path> <sessions_list_file>",
            file=sys.stderr,
        )
        return 2

    try:
        out_path = validate_output_path(argv[1])
    except ValueError as err:
        print(f"invalid output path: {err}", file=sys.stderr)
        return 2

    sessions_list_path = pathlib.Path(argv[2])
    session_paths = [
        pathlib.Path(line)
        for line in sessions_list_path.read_text().splitlines()
        if line.strip()
    ]

    result = find_best_image_blob(session_paths)
    if result is None:
        print("IMAGE_NOT_FOUND_IN_SESSION", file=sys.stderr)
        return 1

    image_bytes, detected_ext = result
    final_out_path = choose_output_path(out_path, detected_ext)
    final_out_path.parent.mkdir(parents=True, exist_ok=True)
    final_out_path.write_bytes(image_bytes)
    print(final_out_path)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
