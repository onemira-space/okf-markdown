import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import {
  parseFrontmatter,
  validateOkfDocument,
  buildKnowledgeGraph,
  convertTranscriptToOkf,
} from '../src/index.mjs';

test('converts WebVTT voice tags into merged OKF v0.2 Markdown speaker turns', () => {
  const vtt = fs.readFileSync(new URL('../examples/sample.vtt', import.meta.url), 'utf8');
  const res = convertTranscriptToOkf(vtt, {
    title: 'Q4 Local AI Architecture Sync',
    createdAt: '2026-11-16T12:00:00Z',
  });
  assert.deepEqual(res.speakers, ['Maya Lin', 'Alex Rivera']);
  assert.equal(res.turnCount, 2); // Two consecutive Maya Lin cues merged cleanly!
  const check = validateOkfDocument(res.markdown);
  assert.equal(check.valid, true);
  assert.equal(check.frontmatter.id, 'session/q4-local-ai-architecture-sync');
});

test('validates relational wiki-links and detects broken references across a vault', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'okf-vault-'));
  fs.writeFileSync(
    path.join(tmp, 'topic-local-ai.md'),
    [
      '---',
      'okf_version: "0.2"',
      'id: "topic/local-ai"',
      'type: "topic"',
      'title: "Local AI Architecture"',
      'tags: ["privacy", "local-ai"]',
      '---',
      '',
      'Connected to [[session/kickoff|Kickoff Notes]].',
    ].join('\n'),
  );
  fs.writeFileSync(
    path.join(tmp, 'session-kickoff.md'),
    [
      '---',
      'okf_version: "0.2"',
      'id: "session/kickoff"',
      'type: "session"',
      'title: "Kickoff Notes"',
      '---',
      '',
      'Discussed [Local AI](./topic-local-ai.md).',
    ].join('\n'),
  );

  const graph = buildKnowledgeGraph(tmp);
  assert.equal(graph.valid, true);
  assert.equal(graph.stats.files, 2);
  assert.equal(graph.stats.edges, 2);
  assert.equal(graph.stats.brokenLinks, 0);

  // CLI smoke test
  const cliOut = execFileSync(
    process.execPath,
    [new URL('../bin/okf.mjs', import.meta.url).pathname, 'validate', tmp, '--json'],
    { encoding: 'utf8' },
  );
  const parsed = JSON.parse(cliOut);
  assert.equal(parsed.valid, true);
  fs.rmSync(tmp, { recursive: true, force: true });
});

test('flags broken wiki links and invalid node types accurately', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'okf-broken-'));
  fs.writeFileSync(
    path.join(tmp, 'bad.md'),
    [
      '---',
      'okf_version: "0.2"',
      'id: "note/1"',
      'type: "note"',
      'title: "Broken Link Check"',
      '---',
      '',
      'See [[topic/non-existent]].',
    ].join('\n'),
  );
  const graph = buildKnowledgeGraph(tmp);
  assert.equal(graph.valid, false);
  assert.equal(graph.brokenLinks.length, 1);
  fs.rmSync(tmp, { recursive: true, force: true });
});
