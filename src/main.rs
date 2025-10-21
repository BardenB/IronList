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
    /// Persist a default file path and exit
    #[arg(long = "set-default", value_name = "PATH")]
    set_default: Option<PathBuf>,
    /// Show the currently saved default and exit
    #[arg(long = "show-default")]
    show_default: bool,

    /// Show all entries including those tagged `complete` (by default completed entries are hidden)
    #[arg(long = "show-all")]
    show_all: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all entries (numbered, sorted by date asc)
    List {},
    /// Append a raw entry line to the todo file. The line should follow the expected format.
    Add {
        /// The raw line to append (e.g. "YYYY-MM-DD    Description    tag1,tag2")
        #[arg(value_name = "LINE")]
        line: String,
    },
    /// Edit an entry by its printed number (from `list`). Replacement_line must be a valid entry.
    Edit {
        /// 1-based index as shown in `list`
        #[arg(value_name = "INDEX")]
        index: usize,

        /// The replacement line (same format as `add`)
        #[arg(value_name = "LINE")]
        line: String,
    },
    /// Mark an entry (by printed number from `list`) as complete by adding the `complete` tag.
    Complete {
        /// 1-based index as shown in `list`
        #[arg(value_name = "INDEX")]
        index: usize,
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

#[derive(Debug, Clone)]
struct Entry {
    date: NaiveDate,
    desc: String,
    tags: Vec<String>,
    #[allow(dead_code)]
    raw_line: String,
}

fn is_complete(e: &Entry) -> bool {
    e.tags.iter().any(|t| t.eq_ignore_ascii_case("complete"))
}

/// Return indices (into the original entries slice) for the entries that should be visible
/// given the `show_all` flag.
fn visible_indices(entries: &[Entry], show_all: bool) -> Vec<usize> {
    if show_all {
        (0..entries.len()).collect()
    } else {
        entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !is_complete(e))
            .map(|(i, _)| i)
            .collect()
    }
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

fn write_entries_to_file(path: &PathBuf, entries: &[Entry]) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
    for e in entries {
        let line = entry_to_line(e);
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
    }
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
    // Table columns:
    // No. (right-aligned width 3) | Date (10) | Task (30, wrapped) | Tags (rest)
    const NUM_AREA: usize = 5; // e.g. "  1. " length
    const TASK_W: usize = 30;
    const TAG_W: usize = 20;

    // Header
    println!("{:>3}  {:10}  {:30}  {:<width$}", "No", "Date", "Task", "Tags", width = TAG_W);
    // underline: dashes matching each column width (tags column uses TAG_W)
    let tag_underline = "-".repeat(TAG_W);
    println!("{:->3}  {:->10}  {:->30}  {}", "", "", "", tag_underline);

    for (i, e) in entries.iter().enumerate() {
        let tag_str = if e.tags.is_empty() { String::from("-") } else { e.tags.join(",") };

        let date_str = e.date.format("%Y-%m-%d").to_string();
        let wrapped = wrap_text(&e.desc, TASK_W);

        for (line_idx, task_line) in wrapped.iter().enumerate() {
            if line_idx == 0 {
                // first line: print number, date, first task part, tags
                println!("{:>3}. {:10}  {:30}  {:<width$}", i + 1, date_str, task_line, tag_str, width = TAG_W);
            } else {
                // continuation lines: blank number and date columns
                let spacer = " ".repeat(NUM_AREA);
                println!("{}{:10}  {:30}  {:<width$}", spacer, "", task_line, "", width = TAG_W);
            }
        }
        // if description was empty, still print a line
        if wrapped.is_empty() {
            println!("{:>3}. {:10}  {:30}  {}", i + 1, date_str, "", tag_str);
        }
    }
}

fn print_titled_tables(all_entries: &[Entry], show_all: bool) {
    // First table: incomplete entries
    let incomplete: Vec<Entry> = all_entries.iter().filter(|e| !is_complete(e)).cloned().collect();
    print_numbered(&incomplete);

    // If requested, print completed entries in a second table below
    if show_all {
        let completed: Vec<Entry> = all_entries.iter().filter(|e| is_complete(e)).cloned().collect();
        if !completed.is_empty() {
            println!("");
            println!("Completed:");
            print_numbered(&completed);
        }
    }
}

/// Simple word-wrap helper: splits on whitespace and builds lines of maximum `width` characters.
fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if s.trim().is_empty() {
        return vec![];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in s.split_whitespace() {
        if current.is_empty() {
            if word.chars().count() <= width {
                current.push_str(word);
            } else {
                // word longer than width -> hard-break
                let mut start = 0;
                let chars: Vec<char> = word.chars().collect();
                while start < chars.len() {
                    let end = (start + width).min(chars.len());
                    let slice: String = chars[start..end].iter().collect();
                    lines.push(slice);
                    start = end;
                }
            }
        } else {
            let tentative = format!("{} {}", current, word);
            if tentative.chars().count() <= width {
                current = tentative;
            } else {
                // move current into lines and leave current empty
                lines.push(std::mem::take(&mut current));
                // start new line with word
                if word.chars().count() <= width {
                    current = word.to_string();
                } else {
                    // word itself is longer than width; break it
                    let mut start = 0;
                    let chars: Vec<char> = word.chars().collect();
                    while start < chars.len() {
                        let end = (start + width).min(chars.len());
                        let slice: String = chars[start..end].iter().collect();
                        if end < chars.len() {
                            lines.push(slice);
                        } else {
                            current = slice;
                        }
                        start = end;
                    }
                }
            }
        }
    }
    if !current.is_empty() {
        lines.push(std::mem::take(&mut current));
    }
    lines
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    // If the user asked to show the saved default, print and exit.
    if cli.show_default {
        if let Some(p) = read_saved_default() {
            println!("Saved default: {}", p.display());
        } else {
            println!("No saved default");
        }
        return Ok(());
    }

    // If the user asked to persist a default path, handle special cases and exit.
    if let Some(p) = &cli.set_default {
        // special case: '-' clears the saved default
        if p.as_os_str() == "-" {
            clear_saved_default()?;
            println!("Cleared saved default");
            return Ok(());
        }

        // validate existence; if missing prompt to create
        if !p.exists() {
            eprintln!("Provided path does not exist: {}", p.display());
            eprintln!("Create the file? (y/N)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            if input.trim().eq_ignore_ascii_case("y") {
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::File::create(p)?;
                eprintln!("Created file: {}", p.display());
            } else {
                eprintln!("Aborted; not saving default.");
                return Ok(());
            }
        }

        persist_default_path(p)?;
        println!("Saved default path to config: {}", p.display());
        return Ok(());
    }

    // Determine the data file path. If the user passed an explicit --file that exists, prefer it.
    // Otherwise consult the persisted default (or ask the user on first run).
    let file_path = if cli.file.as_os_str() != "ironlist.txt" && cli.file.exists() {
        cli.file.clone()
    } else {
        get_or_ask_default_file()?
    };
    let mut entries = read_entries(&file_path)?;

    // sort by date ascending
    entries.sort_by_key(|e| e.date);

    match cli.command {
        None | Some(Commands::List {}) => {
            // Print incomplete entries first; if --show-all, show completed entries in a second table
            print_titled_tables(&entries, cli.show_all);
        }
        Some(Commands::Query { from, to, date, tag, any }) => {
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
            // Print incomplete matches first; if --show-all, show completed matches in a separate table
            print_titled_tables(&by_tags, cli.show_all);
            }
        Some(Commands::Add { line }) => {
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
        Some(Commands::Edit { index, line }) => {
            // Validate replacement
            let parsed = match parse_line(&line) {
                Some(e) => e,
                None => {
                    eprintln!("Replacement line is malformed; expected: YYYY-MM-DD<TAB>Description<TAB>tag1,tag2");
                    std::process::exit(1);
                }
            };


            // Map the user-provided index (1-based within visible list) to the original entries vector
            let vis_idxs = visible_indices(&entries, cli.show_all);
            if index == 0 || index > vis_idxs.len() {
                eprintln!("Index out of range: {} (there are {} visible entries)", index, vis_idxs.len());
                std::process::exit(1);
            }
            let orig_idx = vis_idxs[index - 1];

            // Replace (mapped index)
            entries[orig_idx] = parsed;

            // Write all entries back to the file (normalized)
            write_entries_to_file(&file_path, &entries)?;
            println!("Replaced entry {} in {}", index, file_path.display());
            }
        Some(Commands::Complete { index }) => {
            // Map index from visible list to original entries vector
            let vis_idxs = visible_indices(&entries, cli.show_all);
            if index == 0 || index > vis_idxs.len() {
                eprintln!("Index out of range: {} (there are {} visible entries)", index, vis_idxs.len());
                std::process::exit(1);
            }
            let orig_idx = vis_idxs[index - 1];

            let tags = &mut entries[orig_idx].tags;
            // add 'complete' tag if not already present (case-insensitive)
            if !tags.iter().any(|t| t.eq_ignore_ascii_case("complete")) {
                tags.push("complete".to_string());
            }

            write_entries_to_file(&file_path, &entries)?;
            println!("Marked entry {} as complete in {}", index, file_path.display());
            }
    }

    Ok(())
}

/// Returns the persisted default file path or prompts the user to enter one and persists it.
fn get_or_ask_default_file() -> io::Result<PathBuf> {
    use std::io::{Write, stdin};

    // Try home directory first
    let mut config_paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        config_paths.push(home.join(".ironlist_default"));
    }
    // fallback to current directory
    config_paths.push(PathBuf::from(".ironlist_default"));

    for cfg in &config_paths {
        if cfg.exists() {
            if let Ok(s) = std::fs::read_to_string(cfg) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return Ok(PathBuf::from(trimmed));
                }
            }
        }
    }

    // Not found: prompt the user
    eprintln!("No default data file configured. Please enter the path to your ironlist file:");
    let mut input = String::new();
    stdin().read_line(&mut input).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let entered = input.trim();
    if entered.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No path entered"));
    }

    let path = PathBuf::from(entered);

    // Persist into the first available config path (prefer home)
    if let Some(cfg) = config_paths.get(0) {
        if let Some(parent) = cfg.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(mut f) = std::fs::File::create(cfg) {
            writeln!(f, "{}", path.display()).ok();
        }
    }

    Ok(path)
}

fn persist_default_path(path: &PathBuf) -> io::Result<()> {
    let cfg = if let Some(home) = dirs::home_dir() {
        home.join(".ironlist_default")
    } else {
        PathBuf::from(".ironlist_default")
    };

    if let Some(parent) = cfg.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut f = std::fs::File::create(cfg)?;
    use std::io::Write;
    writeln!(f, "{}", path.display())?;
    Ok(())
}

fn read_saved_default() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        let cfg = home.join(".ironlist_default");
        if cfg.exists() {
            if let Ok(s) = std::fs::read_to_string(cfg) {
                let t = s.trim();
                if !t.is_empty() {
                    return Some(PathBuf::from(t));
                }
            }
        }
    }
    if let Ok(s) = std::fs::read_to_string(".ironlist_default") {
        let t = s.trim();
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    None
}

fn clear_saved_default() -> io::Result<()> {
    if let Some(home) = dirs::home_dir() {
        let cfg = home.join(".ironlist_default");
        if cfg.exists() {
            std::fs::remove_file(cfg)?;
            return Ok(());
        }
    }
    if PathBuf::from(".ironlist_default").exists() {
        std::fs::remove_file(".ironlist_default")?;
    }
    Ok(())
}

