# okf-markdown

> **Interactive OKF v0.2 Browser Validator:** [https://onemira-okf-markdown.static.hf.space/](https://onemira-okf-markdown.static.hf.space/) · **Hugging Face Space:** [onemira/okf-markdown](https://huggingface.co/spaces/onemira/okf-markdown)
> **Codeberg Mirror:** [codeberg.org/onemira/okf-markdown](https://codeberg.org/onemira/okf-markdown) · **Open-Source Index:** [onemira.codeberg.page](https://onemira.codeberg.page/) · **Launchpad:** [launchpad.net/okf-markdown](https://launchpad.net/okf-markdown)

Zero-dependency Node.js CLI and ESM library for validating **Open Knowledge Format (OKF v0.2)** Markdown vaults, building relational knowledge graphs (`[[wiki-links]]` + relative Markdown links), and converting `.vtt` / `.srt` / speaker-labeled transcripts into deterministic OKF session notes.

Designed for local-first AI knowledge workflows across **Obsidian**, **Claude Code / Claude Desktop (MCP)**, **Cursor**, and **[OneMira C1](https://onemira.com/products/c1)**.


### Install via Homebrew (macOS & Linux)

```bash
brew tap onemira-space/tap
brew install okf-markdown
```

## Install as an AI Agent Skill (Claude Code, Cursor, Codex, Gemini CLI)

```bash
# Universal Agent Skills CLI (skills.sh)
npx skills add onemira-space/okf-markdown

# Or in Claude Code interactive session:
# /plugin marketplace add onemira-space/okf-markdown
# /plugin install okf-markdown@onemira-okf-skills
```

## Quick Start (Zero Install via `npx`)

```bash
# 1. Convert a WebVTT or SRT meeting transcript into an OKF v0.2 Markdown session note
npx okf-markdown ingest ./meeting.vtt \
  --title "Q4 Product Architecture Review" \
  --out ./vault/sessions/q4-architecture.md

# 2. Validate YAML frontmatter schema & relational link integrity across an Obsidian / OKF folder
npx okf-markdown validate ./vault

# 3. Export a structured JSON knowledge graph (nodes, edges, orphan notes) for RAG or MCP agents
npx okf-markdown graph ./vault --out ./graph.json
```

## Python 3 Quick Start (`pip`)

```bash
pip install git+https://github.com/onemira-space/okf-markdown.git
okf-py ingest ./meeting.vtt --title "Q4 Architecture Review" > session.md
okf-py validate session.md
```

## Rust Quick Start (`cargo` / `crates.io`)

Zero-dependency Rust library and `okf-md` CLI ([`crates.io/crates/okf-markdown`](https://crates.io/crates/okf-markdown) · [`docs.rs/okf-markdown`](https://docs.rs/okf-markdown)):

```bash
cargo add okf-markdown
# Or install the standalone binary:
cargo install okf-markdown
okf-md validate ./vault
okf-md stats ./vault
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
- **[transcript-to-okf](https://github.com/onemira-space/transcript-to-okf)** (`v1.0.0`, [Docs](https://onemira-space.github.io/transcript-to-okf/)): Offline multi-vendor transcript migrator (`Otter.ai`, `Plaud`, `HiNotes`, `Notta`, `WebVTT`, `SRT`) to [Google OKF v0.2 Markdown & Obsidian](https://onemira.com/blogs/ai-voice-recorder-for-obsidian-and-local-markdown) (`brew install onemira-space/tap/transcript-to-okf`).
- **[okf-wiki-mcp](https://github.com/onemira-space/okf-wiki-mcp)** — Zero-dependency offline Model Context Protocol (`MCP`) server & CLI for local Google OKF v0.2 and Obsidian Markdown meeting vaults (`brew install onemira-space/tap/okf-wiki-mcp` · [Docs](https://onemira-space.github.io/okf-wiki-mcp/) · [OneMira MCP Guide](https://onemira.com/docs/mcp-integration)).

### Regional & Multilingual OKF Note-Taking Guides
- [EN: Voice Recorder for Note-Taking Guide](https://onemira.com/voice-recorder-for-note-taking) · [DE: KI-Diktiergerät für Notizen](https://onemira.com/de/voice-recorder-for-note-taking) · [FR: Dictaphone IA Prise de Notes](https://onemira.com/fr/voice-recorder-for-note-taking) · [IT: Registratore Vocale IA per Appunti](https://onemira.com/it/voice-recorder-for-note-taking) · [ES: Grabadora de Voz IA para Notas](https://onemira.com/es/voice-recorder-for-note-taking)

## License

MIT © [OneMira](https://onemira.com)
- Context7 AI Agent Docs Index: [OneMira LLMs Full Context](https://context7.com/llmstxt/onemira_llms-full_txt) · [OKF Markdown on Context7](https://context7.com/onemira-space/okf-markdown)
- VS Code & Cursor Extension: [OKF Markdown & Local Wiki on Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=onemira.okf-markdown) ([Publisher: OneMira](https://marketplace.visualstudio.com/publishers/onemira))
- Open VSX Registry (Cursor / Windsurf / VSCodium): [OKF Markdown & Local Wiki on Open VSX](https://open-vsx.org/extension/onemira-space/okf-markdown)
- LibHunt Open-Source Index: [OKF Markdown](https://www.libhunt.com/r/okf-markdown) · [OKF Wiki MCP](https://www.libhunt.com/r/okf-wiki-mcp)
- npm.io Package Mirror: [okf-markdown](https://npm.io/package/okf-markdown) · [okf-wiki-mcp](https://npm.io/package/okf-wiki-mcp)
- Architecture Dispatches: [Velog (KR)](https://velog.io/@onemira-ai/%ED%81%B4%EB%9D%BC%EC%9A%B0%EB%93%9C-%EC%97%86%EC%9D%B4-%EB%A1%9C%EC%BB%AC-AI-%ED%9A%8C%EC%9D%98%EB%A1%9D%EC%9D%84-Markdown-Wiki%EC%99%80-MCP%EB%A1%9C-%EC%97%B0%EA%B2%B0%ED%95%98%EB%8A%94-OKF-v0.2-%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98) · [Viblo (EN)](https://viblo.asia/p/building-a-zero-dependency-local-stdio-mcp-server-for-plain-text-okf-markdown-meeting-wikis-kY4gDG6yJAe)
- OKF v0.2 Interactive Specification Mirror: [onemira.neocities.org](https://onemira.neocities.org/) ([Neocities Profile](https://neocities.org/site/onemira))
- Engineering Newsletter Archive: [OneMira Engineering on Buttondown](https://buttondown.com/onemira/archive/okf-v02-local-ai-meeting-notes-markdown-wiki-mcp/) ([RSS](https://buttondown.com/onemira/rss))
