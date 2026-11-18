#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { buildKnowledgeGraph, convertTranscriptToOkf, validateOkfDocument } from '../src/index.mjs';

const args = process.argv.slice(2);
const cmd = args[0];

function printHelp() {
  console.log(`okf-markdown v1.2.1 — Open Knowledge Format (OKF v0.2) Validator & Transcript CLI

Usage:
  okf validate <dir-or-file.md> [--json]
  okf graph <dir> [--out graph.json]
  okf ingest <transcript.vtt|srt|txt> [--title "Meeting Title"] [--out session.md]

Built by OneMira Engineering (https://onemira.com) — MIT License.`);
}

if (!cmd || cmd === '--help' || cmd === '-h') {
  printHelp();
  process.exit(0);
}

if (cmd === 'validate') {
  const target = args[1] || '.';
  const asJson = args.includes('--json');
  const abs = path.resolve(target);
  if (!fs.existsSync(abs)) {
    console.error(`Error: path not found: ${abs}`);
    process.exit(1);
  }
  const stat = fs.statSync(abs);
  if (stat.isFile()) {
    const res = validateOkfDocument(fs.readFileSync(abs, 'utf8'), path.basename(abs));
    if (asJson) console.log(JSON.stringify(res, null, 2));
    else {
      console.log(`${res.valid ? 'PASS' : 'FAIL'} ${res.filePath}`);
      for (const e of res.errors) console.error(`  [error] ${e}`);
      for (const w of res.warnings) console.warn(`  [warn]  ${w}`);
    }
    process.exit(res.valid ? 0 : 1);
  }

  const graph = buildKnowledgeGraph(abs);
  if (asJson) {
    console.log(JSON.stringify(graph, null, 2));
  } else {
    console.log(`${graph.valid ? 'PASS' : 'FAIL'} Scanned ${graph.stats.files} OKF Markdown files (${graph.stats.edges} links, ${graph.stats.orphans} isolated notes)`);
    for (const se of graph.schemaErrors) console.error(`  [schema] ${se.file}: ${se.error}`);
    for (const bl of graph.brokenLinks) console.error(`  [broken] ${bl.source} -> ${bl.target} (${bl.reason})`);
  }
  process.exit(graph.valid ? 0 : 1);
}

if (cmd === 'graph') {
  const target = args[1] || '.';
  const outIdx = args.indexOf('--out');
  const outPath = outIdx !== -1 ? args[outIdx + 1] : null;
  const graph = buildKnowledgeGraph(path.resolve(target));
  const json = JSON.stringify(graph, null, 2) + '\n';
  if (outPath) {
    fs.writeFileSync(path.resolve(outPath), json);
    console.log(`Wrote OKF knowledge graph (${graph.stats.files} nodes, ${graph.stats.edges} edges) to ${outPath}`);
  } else {
    process.stdout.write(json);
  }
  process.exit(0);
}

if (cmd === 'ingest') {
  const inputPath = args[1];
  if (!inputPath || !fs.existsSync(path.resolve(inputPath))) {
    console.error('Error: provide an existing .vtt, .srt, or .txt transcript file');
    process.exit(1);
  }
  const titleIdx = args.indexOf('--title');
  const outIdx = args.indexOf('--out');
  const title = titleIdx !== -1 ? args[titleIdx + 1] : path.basename(inputPath, path.extname(inputPath));
  const raw = fs.readFileSync(path.resolve(inputPath), 'utf8');
  const { markdown, speakers, turnCount } = convertTranscriptToOkf(raw, {
    title,
    source: path.basename(inputPath),
  });
  if (outIdx !== -1 && args[outIdx + 1]) {
    fs.mkdirSync(path.dirname(path.resolve(args[outIdx + 1])), { recursive: true });
    fs.writeFileSync(path.resolve(args[outIdx + 1]), markdown);
    console.log(`Converted ${turnCount} speaker turns (${speakers.length} speakers) -> ${args[outIdx + 1]}`);
  } else {
    process.stdout.write(markdown);
  }
  process.exit(0);
}

console.error(`Unknown command: ${cmd}`);
printHelp();
process.exit(1);
