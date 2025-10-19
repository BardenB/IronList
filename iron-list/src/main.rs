use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

use chrono::NaiveDate;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to todo file (default: ironlist.txt)
    #[arg(short, long, value_name = "FILE", default_value = "ironlist.txt")]
    file: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all entries (numbered, sorted by date asc)
    List {},
    /// Append a raw entry line to the todo file. The line should follow the expected format.
    Add {
        /// The raw line to append (e.g. "YYYY-MM-DD\tDescription\ttag1,tag2")
        #[arg(value_name = "LINE")]
        line: String,
    },
    /// Query entries by date range and/or tags
    Query {
        /// Start date YYYY-MM-DD (inclusive)
        #[arg(long, value_name = "DATE")]
        from: Option<String>,

        /// End date YYYY-MM-DD (inclusive)
        #[arg(long, value_name = "DATE")]
        to: Option<String>,

        /// Exact date YYYY-MM-DD (sets both from and to)
        #[arg(long, value_name = "DATE")]
        date: Option<String>,

        /// Tag filter; can be passed multiple times
        #[arg(long, value_name = "TAG")]
        tag: Vec<String>,

        /// If set, match entries that contain ANY of the provided tags (OR semantics).
        /// By default the query requires ALL provided tags (AND semantics).
        #[arg(long)]
        any: bool,
    },
}

#[derive(Debug)]
struct Entry {
    date: NaiveDate,
    desc: String,
    tags: Vec<String>,
    #[allow(dead_code)]
    raw_line: String,
}

fn parse_line(line: &str) -> Option<Entry> {
    // Expected format: YYYY-MM-DD<TAB>Description<TAB>tag1,tag2
    // Also accept runs of 4+ spaces as a separator because many shells don't accept literal tabs.
    let parts: Vec<&str> = split_on_tab_or_spaces(line);
    if parts.len() < 2 {
        return None;
    }
    let date = NaiveDate::parse_from_str(parts[0].trim(), "%Y-%m-%d").ok()?;
    let desc = parts[1].trim().to_string();
    let tags = if parts.len() >= 3 {
        parts[2]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    Some(Entry {
        date,
        desc,
        tags,
        raw_line: line.to_string(),
    })
}

/// Split a line into fields using either tab characters or runs of 4+ spaces as separators.
fn split_on_tab_or_spaces(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\t' => {
                // separator at i
                parts.push(s[start..i].trim());
                i += 1;
                start = i;
            }
            b' ' => {
                // count run of spaces
                let mut j = i;
                while j < bytes.len() && bytes[j] == b' ' {
                    j += 1;
                }
                if j - i >= 4 {
                    // treat as separator
                    parts.push(s[start..i].trim());
                    // skip all spaces
                    i = j;
                    start = i;
                    continue;
                } else {
                    // not a separator, continue
                    i = j;
                    continue;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    // push remainder
    if start <= s.len() {
        parts.push(s[start..].trim());
    }
    // filter out empty parts that may occur
    parts.into_iter().filter(|p| !p.is_empty()).collect()
}

fn read_entries(path: &PathBuf) -> io::Result<Vec<Entry>> {
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    let mut entries = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        match line {
            Ok(l) => match parse_line(&l) {
                Some(e) => entries.push(e),
                None => eprintln!("Skipping malformed line {}: {}", i + 1, l),
            },
            Err(err) => eprintln!("Error reading line {}: {}", i + 1, err),
        }
    }
    Ok(entries)
}

fn append_entry(path: &PathBuf, line: &str) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

fn entry_to_line(e: &Entry) -> String {
    let tag_str = if e.tags.is_empty() { String::new() } else { e.tags.join(",") };
    if tag_str.is_empty() {
        format!("{}\t{}", e.date.format("%Y-%m-%d"), e.desc)
    } else {
        format!("{}\t{}\t{}", e.date.format("%Y-%m-%d"), e.desc, tag_str)
    }
}

fn filter_by_date_range(entries: Vec<Entry>, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Vec<Entry> {
    entries
        .into_iter()
        .filter(|e| {
            if let Some(f) = from {
                if e.date < f {
                    return false;
                }
            }
            if let Some(t) = to {
                if e.date > t {
                    return false;
                }
            }
            true
        })
        .collect()
}

fn filter_by_tags(entries: Vec<Entry>, tags: &[String], any: bool) -> Vec<Entry> {
    if tags.is_empty() {
        return entries;
    }
    if any {
        // OR semantics: entry must match at least one tag (case-insensitive)
        entries
            .into_iter()
            .filter(|e| tags.iter().any(|q| e.tags.iter().any(|et| et.eq_ignore_ascii_case(q))))
            .collect()
    } else {
        // AND semantics: entry must contain all query tags (case-insensitive)
        entries
            .into_iter()
            .filter(|e| tags.iter().all(|q| e.tags.iter().any(|et| et.eq_ignore_ascii_case(q))))
            .collect()
    }
}

fn print_numbered(entries: &[Entry]) {
    for (i, e) in entries.iter().enumerate() {
        let tag_str = if e.tags.is_empty() {
            String::from("-")
        } else {
            e.tags.join(",")
        };
        println!("{}. {}\t{}\t[{}]", i + 1, e.date.format("%Y-%m-%d"), e.desc, tag_str);
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    // Resolve the file path: if the user passed a simple file name (like "ironlist.txt"),
    // try to find it two directories up (workspace root). This allows running the
    // binary from the crate directory while keeping the data file next to the workspace root.
    // Hardcoded path requested by user
    let hardcoded = PathBuf::from(r"C:\Users\barde\IronList\ironlist.txt");
    // If the user supplied an explicit path (either absolute or relative) and it exists, prefer it.
    // Otherwise fall back to the hardcoded path.
    let file_path = if cli.file.as_os_str() != "ironlist.txt" && cli.file.exists() {
        cli.file.clone()
    } else {
        hardcoded
    };
    let mut entries = read_entries(&file_path)?;

    // sort by date ascending
    entries.sort_by_key(|e| e.date);

    match cli.command {
        Commands::List {} => {
            print_numbered(&entries);
        }
    Commands::Query { from, to, date, tag, any } => {
            // Require at least one criterion (date range, exact date, or tag)
            if from.is_none() && to.is_none() && date.is_none() && tag.is_empty() {
                eprintln!("Query requires at least one of --from, --to, --date or --tag");
                std::process::exit(1);
            }

            // If exact date provided, it overrides from/to
            let (from_date, to_date) = if let Some(d) = date {
                let parsed = NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok();
                (parsed, parsed)
            } else {
                (
                    from.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    to.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                )
            };

            let by_date = filter_by_date_range(entries, from_date, to_date);
            let by_tags = filter_by_tags(by_date, &tag, any);
            print_numbered(&by_tags);
        }
        Commands::Add { line } => {
            // Validate and normalize the line before appending
            let parsed = match parse_line(&line) {
                Some(e) => e,
                None => {
                    eprintln!("Provided line is malformed; expected: YYYY-MM-DD<TAB>Description<TAB>tag1,tag2");
                    std::process::exit(1);
                }
            };
            let norm = entry_to_line(&parsed);
            append_entry(&file_path, &norm)?;
            println!("Appended normalized entry to {}", file_path.display());
        }
    }

    Ok(())
}

// Note: repository search helpers were removed because the file path is currently hardcoded.
