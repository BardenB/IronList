# IronList

A small CLI tool for managing a simple date-tagged todo list stored in a plain text file.

This README documents every command, option and behavior implemented in the current codebase.

---

## Summary

IronList reads a text file where each entry is a single line in the (normalized) format:

```
YYYY-MM-DD<TAB>Description<TAB>tag1,tag2
```

- The program accepts either literal tabs as separators or runs of 4+ spaces when parsing input from shells that don't accept tabs.
- When adding entries the program validates and *normalizes* the entry to use literal tabs before writing to disk.
- Tags are a comma-separated list on the third field (optional).

---

## Build

You need Rust and Cargo installed.

```powershell
# build the project (crate folder)
cd iron-list
cargo build --release
```

The debug/dev build is created with `cargo build`.

---

## Where the data file lives

By default the program uses the hardcoded absolute path:

```
C:\Users\barde\IronList\ironlist.txt
```

You can override this with `-f` / `--file` and point to a different file. The program currently prefers a user-provided `--file` only when that path already exists. If the provided `--file` does not exist, the program will fall back to the hardcoded default path.

Important: the program attempts to read the file at startup. If the chosen file (after the selection logic described above) does not exist, the program will fail when attempting to read entries. In other words, `add` will not create the file unless the selected file already exists at startup. (The append code will create a missing file on write, but the read-at-start behavior prevents reaching that write code if the file is missing.)

If you want to append to a new file, create it first (empty file) or pass a `--file` that already exists. Changing this behavior (so `add` creates the file unconditionally) is a small code change and can be added on request.

---

## Usage

All commands are run from the crate directory (or via the built executable). Basic usage pattern:

```powershell
# show help
cargo run -- --help

# run a command
cargo run -- <command> [options]
```

### Global option

- `-f`, `--file <FILE>`
  - Path to the todo file. Default value (if you don't provide a file) is `ironlist.txt` which participates in the selection logic described above. If you pass a path that already exists, it will be used. Otherwise the hardcoded default will be used.

---

## Commands

### list

```
cargo run -- list
```

List all entries. Output is numbered, sorted by date ascending. Each printed line shows: number, date, description and tags (tags are shown inside square brackets). If an entry has no tags, `-` is printed.

### add

```
cargo run -- add "<LINE>"
```

Append a new entry. `LINE` should be a single-line string that contains at least a date and a description, and optionally tags. The expected input format is:

```
YYYY-MM-DD<TAB>Description<TAB>tag1,tag2
```

Because typing a literal tab in many shells is awkward, the parser also accepts runs of 4 or more spaces as field separators. For example both of these are accepted:

```powershell
cargo run -- add "2025-10-18\tBuy iron\ttools,home"
cargo run -- add "2025-10-18    Buy iron    tools,home"  # 4+ spaces as separators
```

What happens when you `add`:

1. The program validates the provided line using the same parser it uses for reading entries. Validation checks: date parses as `YYYY-MM-DD` and there is at least a date and a description.
2. If validation fails, the program prints an error and exits without writing.
3. If validation succeeds, the entry is normalized into the canonical tab-separated format and appended to the chosen file (with a trailing newline). Example normalized output:

```
2025-10-18\tBuy iron\ttools,home
```

Notes about files: as mentioned above the program reads the file at startup. If the selected file does not exist at startup the program will fail before the `add` handler runs. Create the file first if you plan to append to a brand-new file.

### query

```
cargo run -- query [--from DATE] [--to DATE] [--date DATE] [--tag TAG]... [--any]
```

Query entries by date range and/or tags. At least one of `--from`, `--to`, `--date`, or `--tag` must be provided.

Options:
- `--from <DATE>`
  - Inclusive start date. Format: `YYYY-MM-DD`.
- `--to <DATE>`
  - Inclusive end date. Format: `YYYY-MM-DD`.
- `--date <DATE>`
  - Exact date match shorthand (sets both `from` and `to` to the same date).
- `--tag <TAG>` (repeatable)
  - Tag filter. You may pass this flag multiple times; a single `--tag` value may contain no commas (the program expects comma-separated tags only when they appear in the file itself). The program matches tags case-insensitively.
- `--any`
  - By default multiple `--tag` flags are combined with AND semantics (an entry must include *all* provided tags). If `--any` is supplied, tags are combined with OR semantics (an entry that contains any one of the provided tags will match).

Behavior notes:
- Date filtering is inclusive and combined with tag filtering: the query returns only entries that satisfy both the date constraints and the tag constraints (if provided).
- Examples:
  - Query entries on an exact date with tags (AND):
    ```powershell
    cargo run -- query --date 2025-10-18 --tag work --tag urgent
    ```
    Returns entries on 2025-10-18 that contain both `work` and `urgent` (case-insensitive).
  - Query entries in a date range with OR tag semantics:
    ```powershell
    cargo run -- query --from 2025-10-01 --to 2025-10-31 --any --tag personal --tag errands
    ```
    Returns entries in October 2025 that have either `personal` or `errands`.

---

## File format details

- Each entry is a single line containing at least two fields:
  1. `YYYY-MM-DD` (date, required)
  2. `Description` (string, required)
  3. `tag1,tag2,...` (optional, comma-separated, no spaces required — whitespace will be trimmed)

- Accepted input separators when parsing:
  - Literal TAB characters (`\t`)
  - Runs of 4 or more spaces (helps enter data from shells that don't accept literal tabs)

- Normalization on write:
  - When you `add` an entry the program writes a normalized, tab-separated line to the file using the canonical order: date, tab, description, tab, comma-separated tags (if any).

- Tag matching:
  - Tags are matched case-insensitively for queries.
  - Default semantics for multiple `--tag` flags is AND (entry must contain all tags). Use `--any` to switch to OR semantics.

---

## Examples

Append an item (using spaces as separators):

```powershell
cargo run -- add "2025-10-18    Buy iron    tools,home"
```

List everything:

```powershell
cargo run -- list
```

Query by exact date:

```powershell
cargo run -- query --date 2025-10-18
```

Query by date range and tags (AND semantics):

```powershell
cargo run -- query --from 2025-10-01 --to 2025-10-31 --tag work --tag urgent
```

Query by tags with OR semantics:

```powershell
cargo run -- query --any --tag personal --tag errands
```

Use a different file (only used if the path exists at startup; otherwise the default hardcoded path will be used):

```powershell
cargo run -- -f C:\temp\mylist.txt list
```

---

## Troubleshooting & notes

- If the program fails with an I/O error on startup, verify the chosen file exists and is readable. The program reads the file on startup; if it doesn't exist the process will exit early.
- If you want `add` to create a file that doesn't exist, we can change the startup behavior so the file is created lazily or `add` is allowed to create the file before the initial read. This is a small fix — tell me if you want that.
- Tag normalization (trimming and lower/upper-casing) is minimal — tags are stored as the user provided them, but queries are case-insensitive.

---

## Next steps / Suggested improvements

- Allow `--file` to be used even if the path doesn't exist and have `add` create the file (recommended UX improvement).
- Add structured `add` flags (`--date`, `--desc`, `--tags`) to avoid constructing a single line on the command line.
- Add unit tests for the parser and query logic (particularly `split_on_tab_or_spaces`, `parse_line`, and tag matching).
- Add a `remove` / `edit` command to modify existing entries.
- Add optional output formats (JSON, CSV) or more friendly pretty-printing.

---

If you want any of the suggested tweaks implemented, tell me which one and I will make the change and run the build/tests.
