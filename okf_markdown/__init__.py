"""Zero-dependency Open Knowledge Format (OKF v0.2) parser, validator & VTT/SRT converter."""
from __future__ import annotations
import os
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

OKF_SPEC_VERSION = "0.2"
VALID_NODE_TYPES = {"session", "meeting", "topic", "person", "project", "decision", "note"}


def _unquote(val: str) -> str:
    if (val.startswith('"') and val.endswith('"')) or (val.startswith("'") and val.endswith("'")):
        return val[1:-1].replace('\\"', '"')
    return val


def parse_frontmatter(markdown: str) -> dict[str, Any]:
    text = str(markdown).replace("\r\n", "\n")
    if not text.startswith("---\n"):
        return {"frontmatter": None, "body": text, "errors": ["Missing YAML frontmatter delimiter (---)"]}
    end_idx = text.find("\n---\n", 4)
    if end_idx == -1:
        return {"frontmatter": None, "body": text, "errors": ["Unclosed YAML frontmatter block"]}

    raw_yaml = text[4:end_idx]
    body = text[end_idx + 5 :]
    fm: dict[str, Any] = {}
    errors: list[str] = []
    cur_arr: str | None = None

    for raw_line in raw_yaml.split("\n"):
        line = raw_line.rstrip()
        if not line.strip() or line.strip().startswith("#"):
            continue
        m_list = re.match(r"^\s*-\s+(.*)$", line)
        if m_list and cur_arr is not None:
            fm[cur_arr].append(_unquote(m_list.group(1).strip()))
            continue
        m_kv = re.match(r"^([a-zA-Z0-9_-]+)\s*:\s*(.*)$", line)
        if not m_kv:
            errors.append(f"Invalid frontmatter line: {line}")
            continue
        k, v = m_kv.group(1), m_kv.group(2)
        cur_arr = None
        if v == "":
            fm[k] = []
            cur_arr = k
        elif v.startswith("[") and v.endswith("]"):
            inner = v[1:-1].strip()
            fm[k] = [_unquote(x.strip()) for x in inner.split(",") if x.strip()] if inner else []
        elif v in ("true", "false"):
            fm[k] = v == "true"
        elif re.match(r"^-?\d+(\.\d+)?$", v):
            fm[k] = float(v) if "." in v else int(v)
        else:
            fm[k] = _unquote(v)

    return {"frontmatter": fm, "body": body, "errors": errors}


def extract_links(body: str) -> dict[str, list[dict[str, str | None]]]:
    cleaned = re.sub(r"```[\s\S]*?```", "", str(body))
    cleaned = re.sub(r"`[^`]*`", "", cleaned)
    wiki = [
        {"target": m.group(1).strip(), "alias": m.group(2).strip() if m.group(2) else None}
        for m in re.finditer(r"\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|([^\]]+))?\]\]", cleaned)
    ]
    md = []
    for m in re.finditer(r"(?<!!)\[([^\]]+)\]\(([^)]+)\)", cleaned):
        href = m.group(2).strip().split("#")[0]
        if href and not re.match(r"^(https?:|mailto:|tel:)", href, re.I):
            md.append({"label": m.group(1).strip(), "href": href})
    return {"wiki_links": wiki, "markdown_links": md}


def validate_okf_document(markdown: str, file_path: str = "document.md") -> dict[str, Any]:
    parsed = parse_frontmatter(markdown)
    fm = parsed["frontmatter"]
    errors = list(parsed["errors"])
    warnings: list[str] = []
    if fm is not None:
        if not fm.get("okf_version"):
            warnings.append('Missing recommended `okf_version: "0.2"` field')
        if not isinstance(fm.get("id"), str) or not fm["id"]:
            errors.append("Missing required string field `id`")
        if not isinstance(fm.get("title"), str) or not fm["title"]:
            errors.append("Missing required string field `title`")
        if str(fm.get("type", "")) not in VALID_NODE_TYPES:
            errors.append(f"Field `type` must be one of: {', '.join(sorted(VALID_NODE_TYPES))}")
    return {
        "file_path": file_path,
        "valid": len(errors) == 0,
        "errors": errors,
        "warnings": warnings,
        "frontmatter": fm,
        "links": extract_links(parsed["body"]),
    }


def convert_transcript_to_okf(raw_input: str, title: str = "Recorded Meeting Session", created_at: str | None = None) -> dict[str, Any]:
    text = re.sub(r"^WEBVTT[^\n]*\n+", "", str(raw_input).replace("\r\n", "\n").strip(), flags=re.I)
    blocks = [b.strip() for b in re.split(r"\n{2,}", text) if b.strip()]
    turns: list[dict[str, str | None]] = []
    for block in blocks:
        lines = [l.strip() for l in block.split("\n") if l.strip()]
        t_idx = next((i for i, l in enumerate(lines) if "-->" in l), -1)
        start = None
        payload = lines
        if t_idx != -1:
            raw_s = lines[t_idx].split("-->")[0].strip().replace(",", ".").split(".")[0]
            parts = raw_s.split(":")
            start = f"{parts[1]}:{parts[2]}" if len(parts) == 3 and parts[0] == "00" else raw_s
            payload = lines[t_idx + 1 :]
        joined = re.sub(r"<v\s+([^>]+)>", r"\1: ", " ".join(payload), flags=re.I)
        joined = re.sub(r"<[^>]+>", "", joined).strip()
        if not joined:
            continue
        m_spk = re.match(r"^([A-Za-z0-9_ .'-]{1,36}):\s+(.*)$", joined)
        spk, content = (m_spk.group(1).strip(), m_spk.group(2).strip()) if m_spk else (None, joined)
        if turns and spk and turns[-1]["speaker"] == spk:
            turns[-1]["text"] = f"{turns[-1]['text']} {content}".strip()
        else:
            turns.append({"start": start, "speaker": spk, "text": content})

    speakers = list(dict.fromkeys(t["speaker"] for t in turns if t["speaker"]))
    slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-") or "untitled-session"
    iso = created_at or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    spk_json = ", ".join(f'"{s}"' for s in speakers)
    out_lines = [
        "---",
        f'okf_version: "{OKF_SPEC_VERSION}"',
        f'id: "session/{slug}"',
        'type: "session"',
        f'title: "{title}"',
        f'created_at: "{iso}"',
        f"speakers: [{spk_json}]",
        'tags: ["meeting", "transcript"]',
        "---",
        "",
        f"# {title}",
        "",
        "## Transcript",
        "",
    ]
    for t in turns:
        ts = f"[{t['start']}] " if t["start"] else ""
        sp = f"**{t['speaker']}:** " if t["speaker"] else ""
        out_lines.append(f"- {ts}{sp}{t['text']}")
    return {"markdown": "\n".join(out_lines) + "\n", "speakers": speakers, "turn_count": len(turns)}
