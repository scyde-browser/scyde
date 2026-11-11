//! `shield-compiler` CLI.
//!
//! ```text
//! shield-compiler --lists <dir|file>... --out <dir> [--version V] [--resources resources.json]
//!                 [--require-signature]
//!     Signs with the base64 seed in $FILTERS_SIGNING_KEY when it is set.
//! shield-compiler verify <bundle-dir> --pubkey <file|base64>
//! shield-compiler keygen
//!     Prints a fresh signing seed (for the FILTERS_SIGNING_KEY secret) and its public key.
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use shield_compiler::bundle::{self, Options, Source};

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  shield-compiler --lists <dir|file>... --out <dir> [--version V] [--resources F] [--require-signature]\n  \
         shield-compiler verify <dir> --pubkey <file|base64>\n  shield-compiler keygen"
    );
    ExitCode::from(2)
}

fn read_sources(paths: &[PathBuf]) -> std::io::Result<Vec<Source>> {
    let mut files = Vec::new();
    for p in paths {
        if p.is_dir() {
            let mut entries: Vec<PathBuf> = fs::read_dir(p)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|f| f.extension().is_some_and(|x| x == "txt"))
                .collect();
            entries.sort();
            files.extend(entries);
        } else {
            files.push(p.clone());
        }
    }
    files
        .into_iter()
        .map(|f| {
            Ok(Source {
                name: f
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                text: fs::read_to_string(&f)?,
            })
        })
        .collect()
}

fn keygen() -> ExitCode {
    let mut seed = [0u8; 32];
    match fs::File::open("/dev/urandom")
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut seed))
    {
        Ok(()) => {
            let k = ed25519_dalek::SigningKey::from_bytes(&seed);
            println!("FILTERS_SIGNING_KEY (secret, store as a GitHub Actions secret):");
            println!("{}", bundle::signing_key_to_b64(&k));
            println!(
                "public key (commit to brand/keys/filters.pub), key id {}:",
                bundle::key_id(&k.verifying_key())
            );
            println!("{}", bundle::verifying_key_to_b64(&k.verifying_key()));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("keygen needs /dev/urandom: {e}");
            ExitCode::FAILURE
        }
    }
}

fn verify(dir: &Path, pubkey: &str) -> ExitCode {
    let text = fs::read_to_string(pubkey).unwrap_or_else(|_| pubkey.to_owned());
    let key = match bundle::verifying_key_from_b64(&text) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match bundle::verify(dir, &key) {
        Ok(m) => {
            println!(
                "ok: bundle {} ({} files, key {})",
                m["version"],
                m["files"].as_object().map_or(0, serde_json::Map::len),
                bundle::key_id(&key)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("verify failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("keygen") => return keygen(),
        Some("verify") => {
            let (Some(dir), Some(i)) = (args.get(1), args.iter().position(|a| a == "--pubkey"))
            else {
                return usage();
            };
            let Some(pk) = args.get(i + 1) else {
                return usage();
            };
            return verify(Path::new(dir), pk);
        }
        _ => {}
    }

    let mut lists = Vec::new();
    let mut out = None;
    let mut opts = Options::default();
    let mut require_sig = false;
    let mut it = args.into_iter();
    let mut in_lists = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--lists" => in_lists = true,
            "--out" => {
                in_lists = false;
                out = it.next().map(PathBuf::from);
            }
            "--version" => {
                in_lists = false;
                opts.version = it.next().unwrap_or_default();
            }
            "--resources" => {
                in_lists = false;
                let Some(p) = it.next() else { return usage() };
                match fs::read_to_string(&p) {
                    Ok(t) => opts.resources = Some(t),
                    Err(e) => {
                        eprintln!("{p}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            "--require-signature" => {
                in_lists = false;
                require_sig = true;
            }
            _ if in_lists && !a.starts_with("--") => lists.push(PathBuf::from(a)),
            _ => return usage(),
        }
    }
    let Some(out) = out else { return usage() };
    if lists.is_empty() {
        return usage();
    }

    let key = match env::var("FILTERS_SIGNING_KEY") {
        Ok(s) if !s.trim().is_empty() => match bundle::signing_key_from_b64(&s) {
            Ok(k) => Some(k),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        },
        _ if require_sig => {
            eprintln!("--require-signature: FILTERS_SIGNING_KEY is not set");
            return ExitCode::FAILURE;
        }
        _ => {
            eprintln!("warning: FILTERS_SIGNING_KEY not set, the bundle is unsigned");
            None
        }
    };

    let sources = match read_sources(&lists) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("reading lists: {e}");
            return ExitCode::FAILURE;
        }
    };
    match bundle::compile(&sources, &opts, &out, key.as_ref()) {
        Ok(c) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&c.report).unwrap_or_default()
            );
            println!("bundle {} → {}", opts.version, out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
