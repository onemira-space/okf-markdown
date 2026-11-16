import fs from 'node:fs';
import path from 'node:path';

export const OKF_SPEC_VERSION = '0.2';
export const VALID_NODE_TYPES = new Set([
  'session',
  'meeting',
  'topic',
  'person',
  'project',
  'decision',
  'note',
]);

/**
 * Parse YAML-subset frontmatter (scalars, quoted strings, inline arrays, block arrays)
 * without external dependencies.
 */
export function parseFrontmatter(markdown) {
  const normalized = String(markdown).replace(/\r\n/g, '\n');
  if (!normalized.startsWith('---\n')) {
    return { frontmatter: null, body: normalized, errors: ['Missing YAML frontmatter delimiter (---)'] };
  }
  const endIdx = normalized.indexOf('\n---\n', 4);
  if (endIdx === -1) {
    return { frontmatter: null, body: normalized, errors: ['Unclosed YAML frontmatter block'] };
  }
  const rawYaml = normalized.slice(4, endIdx);
  const body = normalized.slice(endIdx + 5);
  const frontmatter = {};
  const errors = [];
  let currentArrayKey = null;

  for (const rawLine of rawYaml.split('\n')) {
    const line = rawLine.replace(/\s+$/, '');
    if (!line.trim() || line.trim().startsWith('#')) continue;

    const listMatch = line.match(/^\s*-\s+(.*)$/);
    if (listMatch && currentArrayKey) {
      frontmatter[currentArrayKey].push(unquote(listMatch[1].trim()));
      continue;
    }

    const kvMatch = line.match(/^([a-zA-Z0-9_-]+)\s*:\s*(.*)$/);
    if (!kvMatch) {
      errors.push(`Invalid frontmatter line: ${line}`);
      continue;
    }
    const [, key, rawVal] = kvMatch;
    currentArrayKey = null;

    if (rawVal === '') {
      frontmatter[key] = [];
      currentArrayKey = key;
    } else if (rawVal.startsWith('[') && rawVal.endsWith(']')) {
      const inner = rawVal.slice(1, -1).trim();
      frontmatter[key] = inner
        ? inner.split(',').map((item) => unquote(item.trim())).filter(Boolean)
        : [];
    } else if (rawVal === 'true' || rawVal === 'false') {
      frontmatter[key] = rawVal === 'true';
    } else if (/^-?\d+(\.\d+)?$/.test(rawVal)) {
      frontmatter[key] = Number(rawVal);
    } else {
      frontmatter[key] = unquote(rawVal);
    }
  }

  return { frontmatter, body, errors };
}

function unquote(val) {
  if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'"))) {
    return val.slice(1, -1).replace(/\\"/g, '"');
  }
  return val;
}

/**
 * Extract deterministic Wiki links [[target|alias]] and relative Markdown links [text](./file.md).
 */
export function extractLinks(body) {
  const wikiLinks = [];
  const markdownLinks = [];

  const withoutCode = String(body).replace(/```[\s\S]*?```/g, '').replace(/`[^`]*`/g, '');

  const wikiRe = /\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|([^\]]+))?\]\]/g;
  let m;
  while ((m = wikiRe.exec(withoutCode)) !== null) {
    wikiLinks.push({ target: m[1].trim(), alias: m[2] ? m[2].trim() : null });
  }

  const mdRe = /(?<!!)\[([^\]]+)\]\(([^)]+)\)/g;
  while ((m = mdRe.exec(withoutCode)) !== null) {
    const href = m[2].trim().split('#')[0];
    if (!href || /^(https?:|mailto:|tel:)/i.test(href)) continue;
    markdownLinks.push({ label: m[1].trim(), href });
  }

  return { wikiLinks, markdownLinks };
}

/**
 * Validate a single OKF v0.2 Markdown document.
 */
export function validateOkfDocument(markdown, filePath = 'document.md') {
  const { frontmatter, body, errors: parseErrors } = parseFrontmatter(markdown);
  const errors = [...parseErrors];
  const warnings = [];

  if (frontmatter) {
    if (!frontmatter.okf_version) {
      warnings.push('Missing recommended `okf_version: "0.2"` field');
    }
    if (!frontmatter.id || typeof frontmatter.id !== 'string') {
      errors.push('Missing required string field `id`');
    }
    if (!frontmatter.title || typeof frontmatter.title !== 'string') {
      errors.push('Missing required string field `title`');
    }
    if (!frontmatter.type || !VALID_NODE_TYPES.has(String(frontmatter.type))) {
      errors.push(`Field \`type\` must be one of: ${Array.from(VALID_NODE_TYPES).join(', ')}`);
    }
    if (frontmatter.created_at && Number.isNaN(Date.parse(String(frontmatter.created_at)))) {
      errors.push(`Field \`created_at\` is not a valid ISO-8601 date: ${frontmatter.created_at}`);
    }
  }

  const links = extractLinks(body);
  return {
    filePath,
    valid: errors.length === 0,
    errors,
    warnings,
    frontmatter,
    links,
  };
}

/**
 * Scan a directory of OKF Markdown files, validate relational link integrity, and build a knowledge graph.
 */
export function buildKnowledgeGraph(rootDir) {
  const absRoot = path.resolve(rootDir);
  const mdFiles = collectMarkdownFiles(absRoot);
  const nodes = [];
  const byId = new Map();
  const byRelPath = new Map();
  const byStem = new Map();

  for (const absFile of mdFiles) {
    const relPath = path.relative(absRoot, absFile).split(path.sep).join('/');
    const raw = fs.readFileSync(absFile, 'utf8');
    const doc = validateOkfDocument(raw, relPath);
    const id = doc.frontmatter?.id || relPath.replace(/\.md$/i, '');
    const stem = path.basename(relPath, '.md').toLowerCase();
    const node = {
      id,
      title: doc.frontmatter?.title || path.basename(relPath, '.md'),
      type: doc.frontmatter?.type || 'unknown',
      path: relPath,
      speakers: Array.isArray(doc.frontmatter?.speakers) ? doc.frontmatter.speakers : [],
      tags: Array.isArray(doc.frontmatter?.tags) ? doc.frontmatter.tags : [],
      valid: doc.valid,
      errors: doc.errors,
      warnings: doc.warnings,
      rawLinks: doc.links,
    };
    nodes.push(node);
    byId.set(id.toLowerCase(), node);
    byRelPath.set(relPath.toLowerCase(), node);
    byStem.set(stem, node);
  }

  const edges = [];
  const brokenLinks = [];
  const inboundCount = new Map(nodes.map((n) => [n.path, 0]));

  for (const node of nodes) {
    const baseDir = path.posix.dirname(node.path);

    for (const wl of node.rawLinks.wikiLinks) {
      const key = wl.target.toLowerCase().replace(/\.md$/i, '');
      const targetNode = byId.get(key) || byRelPath.get(`${key}.md`) || byStem.get(path.posix.basename(key));
      if (targetNode) {
        edges.push({ from: node.id, to: targetNode.id, kind: 'wiki' });
        inboundCount.set(targetNode.path, (inboundCount.get(targetNode.path) || 0) + 1);
      } else {
        brokenLinks.push({ source: node.path, target: `[[${wl.target}]]`, reason: 'Unresolved wiki target' });
      }
    }

    for (const ml of node.rawLinks.markdownLinks) {
      const resolved = path.posix.normalize(path.posix.join(baseDir, ml.href)).toLowerCase();
      const targetNode = byRelPath.get(resolved) || byRelPath.get(`${resolved}.md`);
      if (targetNode) {
        edges.push({ from: node.id, to: targetNode.id, kind: 'markdown' });
        inboundCount.set(targetNode.path, (inboundCount.get(targetNode.path) || 0) + 1);
      } else if (ml.href.endsWith('.md')) {
        brokenLinks.push({ source: node.path, target: ml.href, reason: 'Missing relative Markdown file' });
      }
    }
  }

  const orphans = nodes
    .filter((n) => (inboundCount.get(n.path) || 0) === 0 && n.rawLinks.wikiLinks.length === 0 && n.rawLinks.markdownLinks.length === 0)
    .map((n) => n.path);

  const schemaErrors = nodes.flatMap((n) => n.errors.map((err) => ({ file: n.path, error: err })));

  return {
    okfSpecVersion: OKF_SPEC_VERSION,
    valid: schemaErrors.length === 0 && brokenLinks.length === 0,
    stats: {
      files: nodes.length,
      edges: edges.length,
      schemaErrors: schemaErrors.length,
      brokenLinks: brokenLinks.length,
      orphans: orphans.length,
    },
    nodes: nodes.map(({ rawLinks, ...rest }) => rest),
    edges,
    schemaErrors,
    brokenLinks,
    orphans,
  };
}

function collectMarkdownFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('.') || entry.name === 'node_modules') continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...collectMarkdownFiles(full));
    else if (entry.isFile() && entry.name.toLowerCase().endsWith('.md')) out.push(full);
  }
  return out.sort();
}

/**
 * Parse WebVTT (.vtt), SubRip (.srt), or speaker-prefixed plain text into merged speaker turns
 * and serialize as a compliant OKF v0.2 session Markdown file.
 */
export function convertTranscriptToOkf(rawInput, options = {}) {
  const text = String(rawInput).replace(/\r\n/g, '\n').trim();
  const cues = parseTranscriptCues(text);
  const mergedTurns = mergeConsecutiveSpeakerTurns(cues);

  const speakers = Array.from(
    new Set(mergedTurns.map((t) => t.speaker).filter((s) => s && s !== 'Speaker')),
  );
  const nowIso = options.createdAt || new Date().toISOString();
  const title = options.title || 'Recorded Meeting Session';
  const slug = slugify(options.id || title);
  const tags = options.tags || ['meeting', 'transcript'];

  const yamlLines = [
    '---',
    `okf_version: "${OKF_SPEC_VERSION}"`,
    `id: "session/${slug}"`,
    'type: "session"',
    `title: "${title.replace(/"/g, '\\"')}"`,
    `created_at: "${nowIso}"`,
    `speakers: [${speakers.map((s) => `"${s.replace(/"/g, '\\"')}"`).join(', ')}]`,
    `tags: [${tags.map((t) => `"${t}"`).join(', ')}]`,
    `source: "${(options.source || 'local-transcript').replace(/"/g, '\\"')}"`,
    '---',
    '',
    `# ${title}`,
    '',
    '## Transcript',
    '',
  ];

  for (const turn of mergedTurns) {
    const tsPrefix = turn.start ? `[${turn.start}] ` : '';
    const spkPrefix = turn.speaker ? `**${turn.speaker}:** ` : '';
    yamlLines.push(`- ${tsPrefix}${spkPrefix}${turn.text}`);
  }

  return {
    markdown: yamlLines.join('\n') + '\n',
    speakers,
    turnCount: mergedTurns.length,
  };
}

export function parseTranscriptCues(text) {
  const blocks = text
    .replace(/^WEBVTT[^\n]*\n+/i, '')
    .split(/\n{2,}/)
    .map((b) => b.trim())
    .filter(Boolean);

  const cues = [];
  for (const block of blocks) {
    const lines = block.split('\n').map((l) => l.trim()).filter(Boolean);
    if (!lines.length) continue;

    let timeIdx = lines.findIndex((l) => l.includes('-->'));
    let start = null;
    let payloadLines = lines;

    if (timeIdx !== -1) {
      const rawStart = lines[timeIdx].split('-->')[0].trim().replace(',', '.');
      start = normalizeTimestamp(rawStart);
      payloadLines = lines.slice(timeIdx + 1);
    }

    const joined = payloadLines
      .join(' ')
      .replace(/<v\s+([^>]+)>/gi, '$1: ')
      .replace(/<[^>]+>/g, '')
      .replace(/\s+/g, ' ')
      .trim();
    if (!joined) continue;

    const spkMatch = joined.match(/^([A-Za-z0-9_\u00C0-\u024F\u4e00-\u9fa5 .'-]{1,36}):\s+(.*)$/);
    if (spkMatch) {
      cues.push({ start, speaker: spkMatch[1].trim(), text: spkMatch[2].trim() });
    } else {
      cues.push({ start, speaker: null, text: joined });
    }
  }
  return cues;
}

function normalizeTimestamp(ts) {
  const parts = ts.split('.')[0].split(':');
  if (parts.length === 3 && parts[0] === '00') return `${parts[1]}:${parts[2]}`;
  return parts.join(':');
}

function mergeConsecutiveSpeakerTurns(cues) {
  const out = [];
  for (const cue of cues) {
    const prev = out[out.length - 1];
    if (prev && prev.speaker && cue.speaker === prev.speaker) {
      prev.text = `${prev.text} ${cue.text}`.trim();
    } else {
      out.push({ ...cue });
    }
  }
  return out;
}

function slugify(str) {
  return String(str)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'untitled-session';
}
