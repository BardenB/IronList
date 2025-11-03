use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

use chrono::Local;
use chrono::NaiveDate;
use chrono::NaiveTime;
use clap::{Parser, Subcommand};
use std::env;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to todo file
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
    /// Run a notifier that will pop up system notifications summarizing today's tasks.
    /// By default this runs once a day at the provided time (default 09:00). Use --interval
    /// to run notifications more frequently (minutes).
    Notify {
        /// Time of day for the daily notification in HH:MM (24-hour) format. Default: 09:00
        #[arg(long, value_name = "HH:MM", default_value = "09:00")]
        time: String,

        /// If provided, send notifications every N minutes instead of once per day at --time.
        #[arg(long, value_name = "MINUTES")]
        interval: Option<u64>,

        /// Install a background scheduled job (system scheduler) and exit. The job will run
        /// the `notify` command at the configured time/interval.
        #[arg(long)]
        install: bool,

        /// Uninstall the scheduled job previously installed and exit.
        #[arg(long)]
        uninstall: bool,
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
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| show_all || !is_complete(e))
        .map(|(i, _)| i)
        .collect()
}

fn parse_line(line: &str) -> Option<Entry> {
    // Expected format: YYYY-MM-DD    Description    tag1,tag2
    // Also accept literal tabs as a separator but is not suggested.
    let parts: Vec<&str> = split_on_tab_or_spaces(line);
    if parts.len() < 2 {
        return None;
    }
    let date = NaiveDate::parse_from_str(parts[0].trim(), "%Y-%m-%d").ok()?;
    let desc = parts[1].trim();
    let tags: Vec<Cow<str>> = if parts.len() >= 3 {
        parts[2]
            .split(',')
            .map(|s| Cow::from(s.trim()))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    Some(Entry {
        date,
        desc: desc.to_string(),
        tags: tags.into_iter().map(|cow| cow.into_owned()).collect(),
        raw_line: line.to_string(),
    })
}

/// Send a system notification using notify-rust. Best-effort: ignore any error.
fn send_notification(summary: &str, body: &str) {
    // Use notify-rust which supports Linux (libnotify), macOS and Windows backends where available.
    match notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .show()
    {
        Ok(_) => (),
        Err(e) => eprintln!("Warning: failed to send notification: {}", e),
    }
}

/// Run a notifier loop. If `interval_minutes` is Some, send every that many minutes.
/// Otherwise send once a day at `time_str` (HH:MM).
fn run_notifier(path: PathBuf, time_str: &str, interval_minutes: Option<u64>) -> io::Result<()> {
    // parse target time
    let target_time = match NaiveTime::parse_from_str(time_str, "%H:%M") {
        Ok(t) => t,
        Err(_) => {
            eprintln!("Invalid time format: {}. Expected HH:MM", time_str);
            std::process::exit(1);
        }
    };

    loop {
        // read fresh entries each notification so changes are picked up
        let entries = match read_entries(&path) {
            Ok(mut v) => {
                v.sort_by_key(|e| e.date);
                v
            }
            Err(e) => {
                eprintln!("Error reading entries for notification: {}", e);
                Vec::new()
            }
        };

        let today = Local::now().date_naive();
        // Upcoming items are entries with date >= today and not complete. Keep order (entries already sorted).
        let upcoming: Vec<&Entry> = entries
            .iter()
            .filter(|e| e.date >= today && !is_complete(e))
            .collect();

        let summary = if upcoming.is_empty() {
            "IronList: no upcoming items".to_string()
        } else {
            format!("IronList: {} upcoming item(s)", upcoming.len())
        };

        let mut body = String::new();
        for e in upcoming.iter().take(10) {
            // include date, short description, and tags
            let tag_str = if e.tags.is_empty() {
                String::from("-")
            } else {
                e.tags.join(",")
            };
            body.push_str(&format!(
                "- {}: {} [{}]\n",
                e.date.format("%Y-%m-%d"),
                e.desc.trim(),
                tag_str
            ));
        }
        if upcoming.len() > 10 {
            body.push_str(&format!("and {} more...", upcoming.len() - 10));
        }

        send_notification(&summary, &body);

        // scheduling
        if let Some(mins) = interval_minutes {
            let dur = std::time::Duration::from_secs(mins.saturating_mul(60));
            std::thread::sleep(dur);
            continue;
        }

        // otherwise compute time until next daily target
        let now = Local::now();
        let today_dt = today.and_time(target_time);
        // if target today is still ahead, wait until then; otherwise wait until tomorrow's target
        let next_dt = if now.time() < target_time {
            today_dt
        } else {
            (today + chrono::Duration::days(1)).and_time(target_time)
        };

        let delta = next_dt - now.naive_local();
        // convert chrono::Duration to std::time::Duration (best effort)
        match delta.to_std() {
            Ok(dur) => std::thread::sleep(dur),
            Err(_) => std::thread::sleep(std::time::Duration::from_secs(60)),
        }
    }
}

/// Install a scheduled job using the platform scheduler so the program does not need to stay running.
#[cfg(target_os = "windows")]
fn install_scheduled_task(time_str: &str, interval_minutes: Option<u64>) -> io::Result<()> {
    let exe = env::current_exe().unwrap_or_else(|_| {
        std::env::args()
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("iron-list"))
    });
    let task_name = "IronList Notify";
    if let Some(mins) = interval_minutes {
        let args = [
            "/Create",
            "/SC",
            "MINUTE",
            "/MO",
            &mins.to_string(),
            "/TN",
            task_name,
            "/TR",
            &format!(
                "powershell -WindowStyle Hidden -Command \"{} notify --time {}\"",
                exe.display(),
                time_str
            ),
            "/F",
        ];
        let status = Command::new("schtasks").args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "schtasks failed with: {}",
                status
            )))
        }
    } else {
        let args = [
            "/Create",
            "/SC",
            "DAILY",
            "/TN",
            task_name,
            "/TR",
            &format!(
                "powershell -WindowStyle Hidden -Command \"{} notify --time {}\"",
                exe.display(),
                time_str
            ),
            "/ST",
            time_str,
            "/F",
        ];
        let status = Command::new("schtasks").args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "schtasks failed with: {}",
                status
            )))
        }
    }
}

#[cfg(target_os = "linux")]
fn install_scheduled_task(time_str: &str, interval_minutes: Option<u64>) -> io::Result<()> {
    use std::fs;
    use std::path::PathBuf;
    let exe = env::current_exe().unwrap_or_else(|_| {
        std::env::args()
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("iron-list"))
    });
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config/systemd/user");
    fs::create_dir_all(&config_dir).ok();
    let service_path = config_dir.join("ironlist-notify.service");
    let timer_path = config_dir.join("ironlist-notify.timer");

    let service = format!(
        r#"[Unit]
Description=IronList notification

[Service]
Type=oneshot
ExecStart={} notify --time {}
"#,
        exe.display(),
        time_str
    );

    let timer = if let Some(mins) = interval_minutes {
        format!(
            r#"[Unit]
Description=Run IronList notify every {} minutes

[Timer]
OnUnitActiveSec={}s
Persistent=true

[Install]
WantedBy=timers.target
"#,
            mins,
            mins * 60
        )
    } else {
        format!(
            r#"[Unit]
Description=Run IronList notify daily at {}

[Timer]
OnCalendar=*-*-* {}:00
Persistent=true

[Install]
WantedBy=timers.target
"#,
            time_str, time_str
        )
    };

    fs::write(&service_path, service)?;
    fs::write(&timer_path, timer)?;

    // reload and enable timer
    let _ = Command::new("systemctl")
        .arg("--user")
        .arg("daemon-reload")
        .status();
    let enable = Command::new("systemctl")
        .arg("--user")
        .arg("enable")
        .arg("--now")
        .arg("ironlist-notify.timer")
        .status();
    match enable {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("systemctl failed with: {}", s),
        )),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("failed to run systemctl: {}", e),
        )),
    }
}

#[cfg(target_os = "macos")]
fn install_scheduled_task(time_str: &str, interval_minutes: Option<u64>) -> io::Result<()> {
    use std::fs;
    use std::path::PathBuf;
    let exe = env::current_exe().unwrap_or_else(|_| {
        std::env::args()
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("iron-list"))
    });
    let launch_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library/LaunchAgents");
    fs::create_dir_all(&launch_dir).ok();
    let plist_path = launch_dir.join("com.ironlist.notify.plist");

    let plist = if let Some(mins) = interval_minutes {
        // StartInterval in seconds
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.ironlist.notify</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>notify</string>
    <string>--time</string>
    <string>{}</string>
  </array>
  <key>StartInterval</key>
  <integer>{}</integer>
</dict>
</plist>
"#,
            exe.display(),
            time_str,
            mins * 60
        )
    } else {
        // StartCalendarInterval: split HH:MM
        let parts: Vec<&str> = time_str.split(':').collect();
        let hour = parts.get(0).unwrap_or(&"0");
        let minute = parts.get(1).unwrap_or(&"0");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.ironlist.notify</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>notify</string>
    <string>--time</string>
    <string>{}</string>
  </array>
  <key>StartCalendarInterval</key>
  <dict>
    <key>Hour</key>
    <integer>{}</integer>
    <key>Minute</key>
    <integer>{}</integer>
  </dict>
</dict>
</plist>
"#,
            exe.display(),
            time_str,
            hour,
            minute
        )
    };

    fs::write(&plist_path, plist)?;

    // load the plist
    let load = Command::new("launchctl")
        .arg("load")
        .arg(plist_path.as_os_str())
        .status();
    match load {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("launchctl failed with: {}", s),
        )),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("failed to run launchctl: {}", e),
        )),
    }
}

/// Uninstall the scheduled job we installed earlier.
#[cfg(target_os = "windows")]
fn uninstall_scheduled_task() -> io::Result<()> {
    let task_name = "IronList Notify";
    let status = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(io::Error::other(format!("schtasks delete failed: {}", s))),
        Err(e) => Err(io::Error::other(format!("failed to run schtasks: {}", e))),
    }
}

#[cfg(target_os = "linux")]
fn uninstall_scheduled_task() -> io::Result<()> {
    use std::path::PathBuf;
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config/systemd/user");
    let service_path = config_dir.join("ironlist-notify.service");
    let timer_path = config_dir.join("ironlist-notify.timer");
    let _ = Command::new("systemctl")
        .arg("--user")
        .arg("disable")
        .arg("--now")
        .arg("ironlist-notify.timer")
        .status();
    let _ = std::fs::remove_file(service_path);
    let _ = std::fs::remove_file(timer_path);
    let _ = Command::new("systemctl")
        .arg("--user")
        .arg("daemon-reload")
        .status();
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_scheduled_task() -> io::Result<()> {
    use std::path::PathBuf;
    let plist_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library/LaunchAgents/com.ironlist.notify.plist");
    let _ = Command::new("launchctl")
        .arg("unload")
        .arg(plist_path.as_os_str())
        .status();
    let _ = std::fs::remove_file(plist_path);
    Ok(())
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

    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
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

    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    for e in entries {
        let line = entry_to_line(e);
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
    }
    Ok(())
}

fn entry_to_line(e: &Entry) -> String {
    let tag_str = if e.tags.is_empty() {
        String::new()
    } else {
        e.tags.join(",")
    };
    if tag_str.is_empty() {
        format!("{}\t{}", e.date.format("%Y-%m-%d"), e.desc)
    } else {
        format!("{}\t{}\t{}", e.date.format("%Y-%m-%d"), e.desc, tag_str)
    }
}

// Define a trait for filtering entries
trait EntryFilter {
    fn filter(&self, entry: &Entry) -> bool;
}

// Implement the trait for closures
impl<F> EntryFilter for F
where
    F: Fn(&Entry) -> bool,
{
    fn filter(&self, entry: &Entry) -> bool {
        self(entry)
    }
}

// Refactor filtering functions to use the trait
fn filter_entries<F>(entries: Vec<Entry>, filter: F) -> Vec<Entry>
where
    F: EntryFilter,
{
    entries.into_iter().filter(|e| filter.filter(e)).collect()
}

/// Filters entries based on a date range.
/// - `entries`: The list of entries to filter.
/// - `from`: The start date (inclusive).
/// - `to`: The end date (inclusive).
fn filter_by_date_range(
    entries: Vec<Entry>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Vec<Entry> {
    filter_entries(entries, |entry: &Entry| match (start_date, end_date) {
        (Some(start), Some(end)) => entry.date >= start && entry.date <= end,
        (Some(start), None) => entry.date >= start,
        (None, Some(end)) => entry.date <= end,
        (None, None) => true,
    })
}

/// Filters entries based on tags.
/// - `entries`: The list of entries to filter.
/// - `tags`: The tags to filter by.
/// - `any`: If true, matches entries with any of the tags. If false, matches entries with all the tags.
fn filter_by_tags(entries: Vec<Entry>, tags: &[String], match_any: bool) -> Vec<Entry> {
    filter_entries(entries, |entry: &Entry| {
        if tags.is_empty() {
            return true;
        }
        if match_any {
            tags.iter().any(|query_tag| {
                entry
                    .tags
                    .iter()
                    .any(|entry_tag| entry_tag.eq_ignore_ascii_case(query_tag))
            })
        } else {
            tags.iter().all(|query_tag| {
                entry
                    .tags
                    .iter()
                    .any(|entry_tag| entry_tag.eq_ignore_ascii_case(query_tag))
            })
        }
    })
}

#[allow(dead_code)]
/// Wraps text to a specified width.
/// - `text`: The text to wrap.
/// - `max_width`: The maximum width of each line.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.trim().is_empty() {
        return vec![];
    }

    text.split_whitespace().fold(Vec::new(), |mut lines, word| {
        if let Some(last_line) = lines.last_mut()
            && last_line.len() + word.len() < max_width
        {
            last_line.push(' ');
            last_line.push_str(word);
            return lines;
        }
        lines.push(word.to_string());
        lines
    })
}

/// Prints entries in two tables: incomplete and completed.
/// - `all_entries`: The list of all entries.
/// - `show_all`: If true, includes completed entries in a separate table.
fn print_titled_tables(all_entries: &[Entry], show_all: bool) {
    // First table: incomplete entries
    let incomplete: Vec<Entry> = all_entries
        .iter()
        .filter(|entry| !is_complete(entry))
        .cloned()
        .collect();
    print_numbered(&incomplete);

    // If requested, print completed entries in a second table below
    if show_all {
        let completed: Vec<Entry> = all_entries
            .iter()
            .filter(|entry| is_complete(entry))
            .cloned()
            .collect();
        if !completed.is_empty() {
            println!();
            println!("Completed:");
            print_numbered(&completed);
        }
    }
}

/// Prints the given entries in a numbered list format.
fn print_numbered(entries: &[Entry]) {
    for (i, entry) in entries.iter().enumerate() {
        let tag_str = if entry.tags.is_empty() {
            String::from("-")
        } else {
            entry.tags.join(",")
        };
        println!("{:>3}: {} [{}]", i + 1, entry.desc.trim(), tag_str);
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    // If the user asked to show the saved default, print and exit.
    if cli.show_default {
        if let Some(p) = read_saved_default()? {
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
        Some(Commands::Query {
            from,
            to,
            date,
            tag,
            any,
        }) => {
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
                    eprintln!(
                        "Provided line is malformed; expected: YYYY-MM-DD    Description    tag1,tag2"
                    );
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
                    eprintln!(
                        "Replacement line is malformed; expected: YYYY-MM-DD    Description    tag1,tag2"
                    );
                    std::process::exit(1);
                }
            };

            // Map the user-provided index (1-based within visible list) to the original entries vector
            let vis_idxs = visible_indices(&entries, cli.show_all);
            if index == 0 || index > vis_idxs.len() {
                eprintln!(
                    "Index out of range: {} (there are {} visible entries)",
                    index,
                    vis_idxs.len()
                );
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
                eprintln!(
                    "Index out of range: {} (there are {} visible entries)",
                    index,
                    vis_idxs.len()
                );
                std::process::exit(1);
            }
            let orig_idx = vis_idxs[index - 1];

            let tags = &mut entries[orig_idx].tags;
            // add 'complete' tag if not already present (case-insensitive)
            if !tags.iter().any(|t| t.eq_ignore_ascii_case("complete")) {
                tags.push("complete".to_string());
            }

            write_entries_to_file(&file_path, &entries)?;
            println!(
                "Marked entry {} as complete in {}",
                index,
                file_path.display()
            );
        }
        Some(Commands::Notify {
            time,
            interval,
            install,
            uninstall,
        }) => {
            if install {
                install_scheduled_task(&time, interval)?;
                println!("Installed scheduled notification job.");
                return Ok(());
            }
            if uninstall {
                uninstall_scheduled_task()?;
                println!("Removed scheduled notification job (if present).");
                return Ok(());
            }

            // Run notifier loop (this function blocks until killed)
            run_notifier(file_path.clone(), &time, interval)?;
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
        if cfg.exists()
            && let Ok(s) = std::fs::read_to_string(cfg)
        {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
    }

    // Not found: prompt the user
    eprintln!("No default data file configured. Please enter the path to your ironlist file:");
    let mut input = String::new();
    stdin().read_line(&mut input).map_err(io::Error::other)?;
    let entered = input.trim();
    if entered.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No path entered",
        ));
    }

    let path = PathBuf::from(entered);

    // Persist into the first available config path (prefer home)
    if let Some(cfg) = config_paths.first() {
        if let Some(parent) = cfg.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(mut f) = std::fs::File::create(cfg) {
            writeln!(f, "{}", path.display()).ok();
        }
    }

    Ok(path)
}

fn persist_default_path(path: &Path) -> io::Result<()> {
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

fn read_saved_default() -> io::Result<Option<PathBuf>> {
    if let Some(home) = dirs::home_dir() {
        let cfg = home.join(".ironlist_default");
        if cfg.exists()
            && let Ok(s) = std::fs::read_to_string(&cfg)
        {
            let t = s.trim();
            if !t.is_empty() {
                return Ok(Some(PathBuf::from(t)));
            }
        }
    }
    if let Ok(s) = std::fs::read_to_string(".ironlist_default") {
        let t = s.trim();
        if !t.is_empty() {
            return Ok(Some(PathBuf::from(t)));
        }
    }
    Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_filter_by_date_range() {
        let entries = vec![
            Entry {
                date: NaiveDate::from_ymd_opt(2025, 11, 1).unwrap(),
                desc: "Task 1".to_string(),
                tags: vec!["work".to_string()],
                raw_line: "2025-11-01\tTask 1\twork".to_string(),
            },
            Entry {
                date: NaiveDate::from_ymd_opt(2025, 11, 2).unwrap(),
                desc: "Task 2".to_string(),
                tags: vec!["home".to_string()],
                raw_line: "2025-11-02\tTask 2\thome".to_string(),
            },
        ];

        let filtered = filter_by_date_range(entries.clone(), Some(NaiveDate::from_ymd_opt(2025, 11, 1).unwrap()), None);
        assert_eq!(filtered.len(), 2);

        let filtered = filter_by_date_range(entries.clone(), Some(NaiveDate::from_ymd_opt(2025, 11, 2).unwrap()), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].desc, "Task 2");
    }

    #[test]
    fn test_filter_by_tags() {
        let entries = vec![
            Entry {
                date: NaiveDate::from_ymd_opt(2025, 11, 1).unwrap(),
                desc: "Task 1".to_string(),
                tags: vec!["work".to_string()],
                raw_line: "2025-11-01\tTask 1\twork".to_string(),
            },
            Entry {
                date: NaiveDate::from_ymd_opt(2025, 11, 2).unwrap(),
                desc: "Task 2".to_string(),
                tags: vec!["home".to_string()],
                raw_line: "2025-11-02\tTask 2\thome".to_string(),
            },
        ];

        let filtered = filter_by_tags(entries.clone(), &["work".to_string()], false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].desc, "Task 1");

        let filtered = filter_by_tags(entries.clone(), &["home".to_string()], true);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].desc, "Task 2");
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a long line of text that needs to be wrapped.";
        let wrapped = wrap_text(text, 10);
        assert_eq!(wrapped, vec!["This is a", "long line", "of text", "that needs", "to be", "wrapped."]);

        let text = "Short line.";
        let wrapped = wrap_text(text, 20);
        assert_eq!(wrapped, vec!["Short line."]);
    }
}
