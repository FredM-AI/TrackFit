//! CLI de `gr-synth` (M2-4) : `cargo run -p gr-synth --release -- --count N --out DIR`,
//! invoque par `just synth N=1000000` (voir CLAUDE.md §4).

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use gr_synth::SynthCorpus;

/// Graine fixe par defaut : reproductible d'une execution a l'autre sans
/// argument supplementaire (utile pour comparer deux runs de perf, M8-4).
const DEFAULT_SEED: u64 = 0xC0FF_EE00_1234_5678;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(options) = parse_args(&args) else {
        eprintln!("usage: gr-synth --count N --out DIR [--seed S]");
        return ExitCode::FAILURE;
    };

    if let Err(e) = fs::create_dir_all(&options.out) {
        eprintln!("impossible de creer {}: {e}", options.out.display());
        return ExitCode::FAILURE;
    }

    let mut hands_written = 0usize;
    let mut files_written = 0usize;
    for tournament in SynthCorpus::new(options.count, options.seed) {
        let path = options.out.join(format!("{}.txt", tournament.file_stem));
        if let Err(e) = fs::write(&path, &tournament.content) {
            eprintln!("impossible d'ecrire {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        hands_written += tournament.hand_count;
        files_written += 1;
    }

    println!(
        "{hands_written} mains generees dans {files_written} fichiers sous {}",
        options.out.display()
    );
    ExitCode::SUCCESS
}

struct Options {
    count: usize,
    out: PathBuf,
    seed: u64,
}

fn parse_args(args: &[String]) -> Option<Options> {
    let mut count = None;
    let mut out = None;
    let mut seed = DEFAULT_SEED;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                i += 1;
                count = args.get(i)?.parse().ok();
            }
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i)?));
            }
            "--seed" => {
                i += 1;
                seed = args.get(i)?.parse().ok()?;
            }
            _ => return None,
        }
        i += 1;
    }

    Some(Options {
        count: count?,
        out: out?,
        seed,
    })
}
