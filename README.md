# okf-markdown

Zero-dependency Node.js CLI and ESM library for validating **Open Knowledge Format (OKF v0.2)** Markdown vaults, building relational knowledge graphs (`[[wiki-links]]` + relative Markdown links), and converting `.vtt` / `.srt` / speaker-labeled transcripts into deterministic OKF session notes.

Designed for local-first AI knowledge workflows across **Obsidian**, **Claude Code / Claude Desktop (MCP)**, **Cursor**, and **[OneMira C1](https://onemira.com/products/c1)**.

## Quick Start (Zero Install via `npx`)

```bash
# 1. Convert a WebVTT or SRT meeting transcript into an OKF v0.2 Markdown session note
npx github:onemira-space/okf-markdown ingest ./meeting.vtt \
  --title "Q4 Product Architecture Review" \
  --out ./vault/sessions/q4-architecture.md

# 2. Validate YAML frontmatter schema & relational link integrity across an Obsidian / OKF folder
npx github:onemira-space/okf-markdown validate ./vault

# 3. Export a structured JSON knowledge graph (nodes, edges, orphan notes) for RAG or MCP agents
npx github:onemira-space/okf-markdown graph ./vault --out ./graph.json
```

## Programmatic ESM API

```js
import {
  parseFrontmatter,
  validateOkfDocument,
  buildKnowledgeGraph,
  convertTranscriptToOkf,
} from 'okf-markdown';

const { markdown, speakers, turnCount } = convertTranscriptToOkf(vttString, {
  title: 'Design Critique',
});

const graph = buildKnowledgeGraph('./vault');
console.log(graph.stats); // { files, edges, schemaErrors, brokenLinks, orphans }
```

## OKF v0.2 Frontmatter Schema

Every OKF Markdown note begins with deterministic YAML frontmatter so local LLMs, CLI tools, and MCP servers can traverse context without proprietary cloud databases:

```yaml
---
okf_version: "0.2"
id: "session/q4-architecture"
type: "session" # session | meeting | topic | person | project | decision | note
title: "Q4 Product Architecture Review"
created_at: "2026-11-16T12:00:00Z"
speakers: ["Maya Lin", "Alex Rivera"]
tags: ["architecture", "local-ai"]
---
```

## Related Documentation & Hardware Ecosystem

- **[OneMira C1 Standalone AI Voice Recorder](https://onemira.com/products/c1)** — Single-press standalone capture, 99-language local transcription, Speaker ID, Local AI summaries ($0/month), and native OKF Markdown Wiki.
- **[Local Wiki & OKF Architecture Guide](https://onemira.com/docs/daily-use/local-wiki)** — How compounding meeting memory is structured on macOS and Windows.
- **[MCP & CLI Integration Guide](https://onemira.com/docs/mcp-integration)** — Connecting local meeting Markdown to Claude, ChatGPT, and Cursor.
- **[Obsidian & Local Markdown Voice Workflow](https://onemira.com/blogs/ai-voice-recorder-for-obsidian-and-local-markdown)** — Using standalone voice capture with an Obsidian vault.
- **[Voice Recorder for Note-Taking Guide](https://onemira.com/voice-recorder-for-note-taking)** — Practical workflow from spoken meetings to structured Markdown notes.

## License

MIT © [OneMira](https://onemira.com)
