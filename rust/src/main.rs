use okf_markdown::{
    build_vault_graph, convert_transcript_to_okf, extract_action_items, validate_okf_document,
};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn print_usage() {
    eprintln!(
        "okf-md — Open Knowledge Format (OKF v0.2) Markdown CLI\n\
         Usage:\n\
         \tokf-md validate <file-or-vault-dir>\n\
         \tokf-md stats <vault-dir>\n\
         \tokf-md actions <file-or-vault-dir>\n\
         \tokf-md ingest <transcript.vtt|srt|txt> <title> <id> <YYYY-MM-DD>"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "validate" => {
            let p = Path::new(&args[2]);
            if p.is_dir() {
                match build_vault_graph(p) {
                    Ok(g) => {
                        println!(
                            "files={} schema_errors={} broken_links={} orphans={}",
                            g.stats.files,
                            g.stats.schema_errors,
                            g.stats.broken_links,
                            g.stats.orphans
                        );
                        if g.stats.schema_errors > 0 || g.stats.broken_links > 0 {
                            for doc in &g.documents {
                                for err in &doc.errors {
                                    eprintln!("ERROR [{}]: {}", doc.file_path, err);
                                }
                            }
                            for (src, tgt) in &g.broken_links {
                                eprintln!("BROKEN_LINK [{}]: {}", src, tgt);
                            }
                            process::exit(2);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to scan vault: {e}");
                        process::exit(1);
                    }
                }
            } else {
                let content = fs::read_to_string(p).unwrap_or_else(|e| {
                    eprintln!("Failed to read {}: {e}", p.display());
                    process::exit(1);
                });
                let rep = validate_okf_document(&content, &args[2]);
                println!("valid={} errors={} warnings={}", rep.valid, rep.errors.len(), rep.warnings.len());
                for err in &rep.errors {
                    eprintln!("ERROR: {err}");
                }
                if !rep.valid {
                    process::exit(2);
                }
            }
        }
        "stats" => {
            let p = Path::new(&args[2]);
            match build_vault_graph(p) {
                Ok(g) => println!(
                    "files={} edges={} schema_errors={} broken_links={} orphans={} open_actions={} completed_actions={}",
                    g.stats.files,
                    g.stats.edges,
                    g.stats.schema_errors,
                    g.stats.broken_links,
                    g.stats.orphans,
                    g.stats.open_actions,
                    g.stats.completed_actions
                ),
                Err(e) => {
                    eprintln!("Failed to scan vault: {e}");
                    process::exit(1);
                }
            }
        }
        "actions" => {
            let p = Path::new(&args[2]);
            if p.is_file() {
                let content = fs::read_to_string(p).unwrap_or_else(|e| {
                    eprintln!("Failed to read {}: {e}", p.display());
                    process::exit(1);
                });
                for item in extract_action_items(&content) {
                    let status = if item.completed { "x" } else { " " };
                    let owner = item.owner.as_deref().unwrap_or("-");
                    println!("L{} [{}] @{} {}", item.line, status, owner, item.text);
                }
            } else {
                match build_vault_graph(p) {
                    Ok(g) => println!(
                        "open_actions={} completed_actions={}",
                        g.stats.open_actions, g.stats.completed_actions
                    ),
                    Err(e) => {
                        eprintln!("Failed to scan vault: {e}");
                        process::exit(1);
                    }
                }
            }
        }
        "ingest" => {
            if args.len() < 6 {
                print_usage();
                process::exit(1);
            }
            let raw = fs::read_to_string(&args[2]).unwrap_or_else(|e| {
                eprintln!("Failed to read {}: {e}", args[2]);
                process::exit(1);
            });
            let out = convert_transcript_to_okf(&raw, &args[3], &args[4], &args[5]);
            print!("{}", out.markdown);
        }
        _ => {
            print_usage();
            process::exit(1);
        }
    }
}
