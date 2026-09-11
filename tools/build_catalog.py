"""Toolkit Hub catalog generator.

Reads `catalog/sources.toml`, resolves each tool's latest GitHub release, picks
the one asset the tool's rule names, pins its SHA-256, and writes
`catalog/catalog.json` -- optionally signed with minisign.

Run it:

    py -3 tools/build_catalog.py --out catalog/catalog.json
    py -3 tools/build_catalog.py --only forgepact --no-download
    py -3 tools/build_catalog.py --sign --key catalog-signing.key

Why a generator instead of the hub calling GitHub directly (D3): ten
`releases/latest` calls would burn a sixth of an unauthenticated client's hourly
rate limit on every launch, would give the hub no integrity guarantee about what
came back, and could not express rules like "of HSCraftSim's seven assets, the
Windows-x64 zip". One signed file answers all three.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import re
import sys
import tomllib
import urllib.error
import urllib.request
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))

import minisign  # noqa: E402  (same directory, deliberately not a package)

SCHEMA_VERSION = 1
USER_AGENT = "hero-siege-toolkit-catalog-builder/1.0 (+https://github.com/falorfrozen-cmd)"

# Assets that carry checksums for *other* assets rather than being installable
# themselves. The three spellings below are all in current use across the ten
# repos: `ForgePact-1.3.16.zip.sha256`, `SHA256SUMS.txt`,
# `HSCraftSim-1.0.2-SHA256SUMS.txt` and `SHA256SUMS-HSSaveEditor-v1.4.1.txt`.
_CHECKSUM_ASSET = re.compile(r"(?i)(\.sha256$|sha256sums)")

_CHECKSUM_LINE = re.compile(r"^([0-9a-fA-F]{64})\s+\*?(.+?)\s*$")

VALID_KINDS = {"zip", "exe", "nsis", "html"}


class CatalogError(Exception):
    """A rule could not be satisfied. Always fatal -- never a silent skip."""


# --------------------------------------------------------------------------
# Fetching
# --------------------------------------------------------------------------


class GitHubFetcher:
    """The real network. Tests substitute an object with the same two methods."""

    def __init__(self, token: Optional[str] = None, cache_dir: Optional[Path] = None) -> None:
        self.token = token
        self.cache_dir = cache_dir
        if cache_dir is not None:
            cache_dir.mkdir(parents=True, exist_ok=True)

    def _request(self, url: str, accept: str) -> bytes:
        request = urllib.request.Request(url, headers={
            "User-Agent": USER_AGENT,
            "Accept": accept,
        })
        if self.token:
            request.add_header("Authorization", f"Bearer {self.token}")
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace")[:400]
            raise CatalogError(f"GET {url} -> HTTP {exc.code}: {detail}") from exc
        except urllib.error.URLError as exc:
            raise CatalogError(f"GET {url} failed: {exc.reason}") from exc

    def release(self, repo: str) -> Dict[str, Any]:
        url = f"https://api.github.com/repos/{repo}/releases/latest"
        return json.loads(self._request(url, "application/vnd.github+json"))

    def asset(self, url: str, name: str = "") -> bytes:
        """Download an asset, using the on-disk cache when the bytes are already here."""
        cache_path = None
        if self.cache_dir is not None and name:
            cache_path = self.cache_dir / name
            if cache_path.exists():
                return cache_path.read_bytes()
        data = self._request(url, "application/octet-stream")
        if cache_path is not None:
            cache_path.write_bytes(data)
        return data


# --------------------------------------------------------------------------
# Pure helpers -- each of these is what the unit tests actually exercise
# --------------------------------------------------------------------------


def normalize_version(tag: str) -> str:
    """`v1.3.16` -> `1.3.16`. The tag is authoritative; the asset name is not.

    HS-ValueEditor's tag `v1.0.2` ships `HSValueScanner-Windows-v1.0.1.zip`, so
    reading the version out of the filename would publish a version that does not
    exist.
    """
    tag = tag.strip()
    return tag[1:] if re.match(r"^[vV][0-9]", tag) else tag


def select_asset(assets: Sequence[Dict[str, Any]], pattern: str, tool_id: str) -> Dict[str, Any]:
    """Find the one asset a tool's rule names. Zero or several is a hard error."""
    compiled = re.compile(pattern)
    matches = [a for a in assets if compiled.match(a["name"])]
    if not matches:
        available = ", ".join(a["name"] for a in assets) or "(none)"
        raise CatalogError(
            f"{tool_id}: no release asset matches {pattern!r}. Available: {available}"
        )
    if len(matches) > 1:
        names = ", ".join(a["name"] for a in matches)
        raise CatalogError(
            f"{tool_id}: {pattern!r} matched {len(matches)} assets ({names}); "
            "tighten the pattern so the choice stays explicit"
        )
    return matches[0]


def parse_checksums(text: str) -> Dict[str, str]:
    """Parse a `sha256sum`-style file into {filename: lowercase hex}.

    Four variants are live in these repos: two-space separated and
    `*`-prefixed (binary mode), lowercase and uppercase hex.
    """
    result: Dict[str, str] = {}
    for line in text.replace("\r\n", "\n").split("\n"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        match = _CHECKSUM_LINE.match(line)
        if match:
            result[match.group(2)] = match.group(1).lower()
    return result


def _checksum_name_keys(name: str) -> List[str]:
    """Every spelling a checksum file might use for the asset called `name`.

    GitHub rewrites spaces in an uploaded asset's filename to dots, so
    HS-Offline-Tracker's `SHA256SUMS-0.1.2.txt` lists
    `HS Offline Tracker_0.1.2_x64-setup.exe` while the asset downloads as
    `HS.Offline.Tracker_0.1.2_x64-setup.exe`. Matching only on the literal name
    would silently fall through to re-downloading and re-hashing, which works but
    hides a real mismatch behind a slow path.
    """
    keys = [name, name.replace(" ", "."), Path(name).name]
    return list(dict.fromkeys(keys))


def find_checksum(checksums: Dict[str, str], asset_name: str) -> Optional[str]:
    normalized = {k.replace(" ", "."): v for k, v in checksums.items()}
    lowered = {k.lower(): v for k, v in normalized.items()}
    for key in _checksum_name_keys(asset_name):
        if key in checksums:
            return checksums[key]
        dotted = key.replace(" ", ".")
        if dotted in normalized:
            return normalized[dotted]
        if dotted.lower() in lowered:
            return lowered[dotted.lower()]
    return None


def is_checksum_asset(name: str) -> bool:
    return bool(_CHECKSUM_ASSET.search(name))


def derive_strip_prefix(archive: bytes) -> str:
    """The single top-level directory a zip wraps everything in, or "".

    ForgePact zips `dist/ForgePact/`, so its archive has a `ForgePact/` root that
    must come off during install or the exe lands one level too deep. Other tools
    zip the contents directly. Reading it off the archive beats declaring it: a
    tool that changes its packaging gets caught by the next catalog build instead
    of by a player whose Launch button does nothing.
    """
    with zipfile.ZipFile(io.BytesIO(archive)) as zf:
        names = [n for n in zf.namelist() if n not in ("", "/")]
    if not names:
        raise CatalogError("archive is empty")
    roots = {n.replace("\\", "/").split("/", 1)[0] for n in names}
    if len(roots) != 1:
        return ""
    root = roots.pop()
    # A single root that is itself a file (no entry below it) is not a wrapper.
    if not any(n.replace("\\", "/").startswith(root + "/") for n in names):
        return ""
    return root + "/"


def archive_contains(archive: bytes, relative_path: str, strip_prefix: str) -> bool:
    wanted = (strip_prefix + relative_path).replace("\\", "/").lower()
    with zipfile.ZipFile(io.BytesIO(archive)) as zf:
        return any(n.replace("\\", "/").lower() == wanted for n in zf.namelist())


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# --------------------------------------------------------------------------
# Building
# --------------------------------------------------------------------------


def load_sources(path: Path) -> Dict[str, Any]:
    with path.open("rb") as handle:
        sources = tomllib.load(handle)
    if sources.get("schema") != SCHEMA_VERSION:
        raise CatalogError(
            f"{path} declares schema {sources.get('schema')!r}; this generator writes {SCHEMA_VERSION}"
        )
    tools = sources.get("tool")
    if not tools:
        raise CatalogError(f"{path} defines no [[tool]] entries")
    seen: set = set()
    for tool in tools:
        for field in ("id", "name", "repo", "asset_pattern", "kind"):
            if not tool.get(field):
                raise CatalogError(f"a [[tool]] entry is missing {field!r}")
        if tool["kind"] not in VALID_KINDS:
            raise CatalogError(
                f"{tool['id']}: kind {tool['kind']!r} is not one of {sorted(VALID_KINDS)}"
            )
        if tool["id"] in seen:
            raise CatalogError(f"duplicate tool id {tool['id']!r}")
        seen.add(tool["id"])
    return sources


def build_tool_entry(
    tool: Dict[str, Any],
    fetcher: Any,
    *,
    download: bool = True,
    warnings: Optional[List[str]] = None,
) -> Dict[str, Any]:
    warnings = warnings if warnings is not None else []
    tool_id = tool["id"]
    release = fetcher.release(tool["repo"])
    if "tag_name" not in release:
        raise CatalogError(
            f"{tool_id}: {tool['repo']} has no published release "
            f"({release.get('message', 'no tag_name in response')})"
        )

    assets = release.get("assets", [])
    asset = select_asset(assets, tool["asset_pattern"], tool_id)
    version = normalize_version(release["tag_name"])

    published_sha = _sha256_from_sidecars(assets, asset, fetcher, tool_id, warnings)

    payload: Optional[bytes] = None
    if download:
        payload = fetcher.asset(asset["browser_download_url"], asset["name"])
        computed = sha256_hex(payload)
        if published_sha and published_sha != computed:
            raise CatalogError(
                f"{tool_id}: {asset['name']} hashes to {computed} but the release's "
                f"own checksum file says {published_sha}. Refusing to pin either."
            )
        if len(payload) != asset.get("size", len(payload)):
            warnings.append(
                f"{tool_id}: downloaded {len(payload)} bytes, release metadata said "
                f"{asset.get('size')}"
            )
        sha256 = computed
    elif published_sha:
        sha256 = published_sha
    else:
        raise CatalogError(
            f"{tool_id}: {asset['name']} has no published checksum and --no-download "
            "forbids hashing it. Either drop --no-download or publish a sidecar."
        )

    artifact: Dict[str, Any] = {
        "kind": tool["kind"],
        "name": asset["name"],
        "url": asset["browser_download_url"],
        "size": asset.get("size", len(payload) if payload else 0),
        "sha256": sha256,
        "sha256_source": "published" if (published_sha and not download) else
                         "published+verified" if published_sha else "computed",
    }

    launch = dict(tool.get("launch", {}))
    if tool["kind"] in ("zip", "html"):
        # `html` describes how the hub opens the thing, not how it arrives: the
        # Steam Deck editor ships as a zip around an .html, so it extracts
        # exactly like `zip` does.
        strip_prefix = tool.get("strip_prefix")
        if strip_prefix is None and payload is not None:
            strip_prefix = derive_strip_prefix(payload)
        artifact["strip_prefix"] = strip_prefix or ""
        entry_point = launch.get("exe")
        if payload is not None and entry_point and not archive_contains(
            payload, entry_point, artifact["strip_prefix"]
        ):
            raise CatalogError(
                f"{tool_id}: {asset['name']} does not contain "
                f"{artifact['strip_prefix']}{entry_point}. The launch rule and the "
                "archive disagree; one of them is stale."
            )
    elif tool["kind"] in ("exe", "nsis"):
        # A bare executable is its own install tree. The download is saved under
        # the launch rule's name rather than the release's, so the path the hub
        # runs stays the same across versions -- `HeroSiegeItemEditor.exe` rather
        # than `HeroSiegeItemEditor-v2.15.4-s10.exe`, which would otherwise put
        # the version in two places that can disagree.
        artifact["strip_prefix"] = ""
        launch.setdefault("exe", asset["name"])
        artifact["install_as"] = launch["exe"]

    entry: Dict[str, Any] = {
        "id": tool_id,
        "name": tool["name"],
        "summary": tool.get("summary", ""),
        "repo": tool["repo"],
        "submodule": tool.get("submodule", ""),
        "version": version,
        "tag": release["tag_name"],
        "published": release.get("published_at", ""),
        "license": tool.get("license", "NOASSERTION"),
        "requires": {
            "admin": bool(tool.get("requires", {}).get("admin", False)),
            "game_closed": bool(tool.get("requires", {}).get("game_closed", False)),
            "game_running": bool(tool.get("requires", {}).get("game_running", False)),
            "windows_only": bool(tool.get("requires", {}).get("windows_only", True)),
        },
        "artifact": artifact,
        "launch": launch,
        "notes_url": release.get("html_url", ""),
        "notes": _trim_notes(release.get("body") or ""),
        "guide": tool.get("guide", ""),
    }
    if tool.get("source_launch"):
        entry["source_launch"] = tool["source_launch"]
    return entry


def _sha256_from_sidecars(
    assets: Sequence[Dict[str, Any]],
    asset: Dict[str, Any],
    fetcher: Any,
    tool_id: str,
    warnings: List[str],
) -> Optional[str]:
    for sidecar in assets:
        if sidecar["name"] == asset["name"] or not is_checksum_asset(sidecar["name"]):
            continue
        try:
            text = fetcher.asset(sidecar["browser_download_url"], sidecar["name"]).decode(
                "utf-8", "replace"
            )
        except CatalogError as exc:
            warnings.append(f"{tool_id}: could not read {sidecar['name']}: {exc}")
            continue
        found = find_checksum(parse_checksums(text), asset["name"])
        if found:
            return found
    return None


def _trim_notes(body: str, limit: int = 8000) -> str:
    body = body.replace("\r\n", "\n").strip()
    if len(body) <= limit:
        return body
    return body[:limit].rstrip() + "\n\n[...truncated; see the release page...]"


def build_catalog(
    sources: Dict[str, Any],
    fetcher: Any,
    *,
    only: Optional[Iterable[str]] = None,
    download: bool = True,
    generated: Optional[str] = None,
) -> Dict[str, Any]:
    only_set = set(only) if only else None
    warnings: List[str] = []
    tools: List[Dict[str, Any]] = []
    for tool in sources["tool"]:
        if only_set is not None and tool["id"] not in only_set:
            continue
        tools.append(build_tool_entry(tool, fetcher, download=download, warnings=warnings))
    if only_set is not None:
        missing = only_set - {t["id"] for t in tools}
        if missing:
            raise CatalogError(f"--only named unknown tools: {', '.join(sorted(missing))}")
    return {
        "schema": SCHEMA_VERSION,
        "generated": generated or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "tools": tools,
        "warnings": warnings,
    }


def serialize(catalog: Dict[str, Any]) -> bytes:
    """The exact bytes that get hashed, signed and shipped.

    Signing covers these bytes, so the serialization has to be stable: sorted
    nothing (field order is meaningful and hand-chosen), two-space indent, a
    trailing newline, and no platform-dependent line endings.
    """
    text = json.dumps(catalog, indent=2, ensure_ascii=False) + "\n"
    return text.encode("utf-8")


def main(argv: Optional[Sequence[str]] = None) -> int:
    repo_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--sources", type=Path, default=repo_root / "catalog" / "sources.toml")
    parser.add_argument("--out", type=Path, default=repo_root / "catalog" / "catalog.json")
    parser.add_argument("--only", nargs="+", default=None, metavar="ID")
    parser.add_argument(
        "--no-download",
        action="store_true",
        help="trust the releases' own checksum sidecars instead of fetching every "
             "artifact; skips strip_prefix derivation and the launch-path check",
    )
    parser.add_argument("--cache-dir", type=Path, default=None,
                        help="keep downloaded assets here so reruns are cheap")
    parser.add_argument("--token", default=os.environ.get("GITHUB_TOKEN"),
                        help="GitHub token; raises the rate limit, never required")
    parser.add_argument("--sign", action="store_true", help="also write <out>.minisig")
    parser.add_argument("--key", type=Path, default=None,
                        help="minisign secret key; defaults to $HUB_MINISIGN_SECRET_KEY")
    parser.add_argument("--print", dest="print_only", action="store_true",
                        help="write nothing; dump the catalog to stdout")
    args = parser.parse_args(argv)

    try:
        sources = load_sources(args.sources)
        fetcher = GitHubFetcher(token=args.token, cache_dir=args.cache_dir)
        catalog = build_catalog(
            sources, fetcher, only=args.only, download=not args.no_download
        )
    except CatalogError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    payload = serialize(catalog)
    if args.print_only:
        sys.stdout.write(payload.decode("utf-8"))
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(payload)
    print(f"catalog -> {args.out} ({len(catalog['tools'])} tools, {len(payload)} bytes)")

    if args.sign:
        try:
            key = minisign.load_secret_key(args.key)
        except minisign.MinisignError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 1
        signature_path = args.out.with_name(args.out.name + ".minisig")
        signature_path.write_text(
            minisign.sign_bytes(
                key,
                payload,
                untrusted_comment="Hero Siege Toolkit catalog",
                trusted_comment=f"catalog {catalog['generated']} tools:{len(catalog['tools'])}",
            ),
            encoding="utf-8",
        )
        print(f"signature -> {signature_path}")

    for warning in catalog["warnings"]:
        print(f"warning: {warning}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
