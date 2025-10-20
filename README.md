
# IronList

A small CLI tool for managing a simple date-tagged todo list stored in a plain text file.

This README documents the current commands, flags and behaviors implemented in the codebase.

---

## Summary

IronList stores each entry as a single, normalized line:

```
YYYY-MM-DD<TAB>Description<TAB>tag1,tag2
```

- Input parsing accepts literal TAB characters or runs of 4+ spaces as field separators (helpful when shells make typing tabs awkward).
- On write (when adding or editing) the program normalizes entries to the canonical tab-separated format.
- Tags are an optional comma-separated list in the third field; queries match tags case-insensitively by default.

---

## Build

You need Rust and Cargo installed.

```powershell
# build the project (crate folder)
cd iron-list
cargo build --release
```

For day-to-day development use `cargo build`.

---

## Data file selection and configuration

The program chooses a data file using the following precedence:

1. If you pass `-f/--file <PATH>` and that path exists at startup, it is used.
2. Otherwise the program uses a persisted default path (created by the program on first run or set with `--set-default`).
3. If no persisted default exists the program prompts you to enter one interactively and saves it.

Persistence location:
- Preferred: `$HOME/.ironlist_default` (the user's home directory).
- Fallback: `./.ironlist_default` in the current working directory.

Commands to manage the saved default:
- `--set-default <PATH>` — saves the provided path and exits. If the path does not exist the program prompts to create it. Passing `-` (a single dash) clears the saved default.
- `--show-default` — prints the currently saved default (or `No saved default`) and exits.

Examples:

```powershell
# persist a default path (prompts to create file if missing)
cargo run -- --set-default C:\path\to\ironlist.txt

# clear saved default
cargo run -- --set-default -

# show saved default
cargo run -- --show-default
```

Notes:
- The program reads the selected file at startup. If the final selected path does not exist, the program will error when reading entries (unless you created the file earlier or chose to create it during `--set-default`).
- You can still use `--file` to temporarily point to a different file (only used if the path exists at startup).

---

## Usage

Top-level flags work without providing a subcommand. If no subcommand is given the default action is `list`.

```powershell
# show help
cargo run -- --help

# show saved default
cargo run -- --show-default

# list (no subcommand required)
cargo run --

# run a command explicitly
cargo run -- <command> [options]
```

Global option
- `-f`, `--file <FILE>` — Path to the todo file. The program will use this path only if it exists at startup; otherwise the persisted default will be used.

---

## Commands

### list (default)

```powershell
cargo run -- list
```

Prints a three-column table with headers and wrapped task descriptions:

- Column 1: `No` — item number (right-aligned).
- Column 2: `Date` — `YYYY-MM-DD` (10 chars).
- Column 3: `Task` — description (30 characters width, word-wrapped).
- Column 4: `Tags` — comma-separated tags (left-aligned; width ~20 in the current layout).

The output is sorted by date ascending. Multi-line task descriptions are printed with continuation lines aligned under the `Task` column.

Example:

```
 No   Date        Task                           Tags
---  ----------  ------------------------------  --------------------
  1. 2025-10-18  Buy iron and supplies           tools,home
     2025-10-18  (continued task text wraps here)
```

### add

```powershell
cargo run -- add "<LINE>"
```

Append a new entry. `LINE` must contain at least a date and a description. Expected input example:

```
YYYY-MM-DD<TAB>Description<TAB>tag1,tag2
```

Because tabs are inconvenient in some shells the parser also accepts runs of 4+ spaces as separators. Valid examples:

```powershell
cargo run -- add "2025-10-18\tBuy iron\ttools,home"
cargo run -- add "2025-10-18    Buy iron    tools,home"
```

On add the program validates the date and presence of a description. If valid it writes a normalized tab-separated line to disk.

### edit

```powershell
cargo run -- edit <INDEX> "<LINE>"
```

Replace the numbered entry shown by `list` with the provided normalized line. The replacement is validated before being written. At the moment the program rewrites the file with normalized entries when editing.

### complete

```powershell
cargo run -- complete <INDEX>
```

Mark the chosen (numbered) entry as complete by adding a `complete` tag (case-insensitive check prevents duplicates). This operation currently rewrites the normalized file.

### query

```powershell
cargo run -- query [--from DATE] [--to DATE] [--date DATE] [--tag TAG]... [--any]
```

Filter by date range and/or tags. At least one of `--from`, `--to`, `--date`, or `--tag` must be provided.

Options:
- `--from <DATE>` — Inclusive start date (YYYY-MM-DD).
- `--to <DATE>` — Inclusive end date (YYYY-MM-DD).
- `--date <DATE>` — Shorthand exact-date match (sets both `from` and `to`).
- `--tag <TAG>` — Repeatable tag filter (case-insensitive).
- `--any` — Switch tag filtering from AND (default) to OR semantics.

Behavior notes:
- Date filtering is inclusive and combined with tag filtering.
- Tags are matched case-insensitively.

Example:

```powershell
cargo run -- query --date 2025-10-18 --tag work --tag urgent
```

---

## File format details

- Each entry is a single line: `YYYY-MM-DD<TAB>Description<TAB>tag1,tag2`.
- Accepted input separators when parsing: literal `\t` or runs of 4+ spaces.
- On write, entries are normalized to the canonical tab-separated form.
- Tag matching for queries is case-insensitive; stored tags preserve the user's casing.

---

## Examples (quick)

Append using spaces:

```powershell
cargo run -- add "2025-10-18    Buy iron    tools,home"
```

Show saved default:

```powershell
cargo run -- --show-default
```

Set default (creates file if you confirm):

```powershell
cargo run -- --set-default C:\Users\barde\IronList\ironlist.txt
```

Clear saved default:

```powershell
cargo run -- --set-default -
```

List everything (no subcommand required):

```powershell
cargo run --
```

Query with OR tags:

```powershell
cargo run -- query --any --tag personal --tag errands
```

---

## Troubleshooting & notes

- If the program errors while reading the data file at startup, verify the selected file exists and is readable.
- The program prefers `--file` only when the provided path exists at startup; otherwise the persisted default is used.
- Editing and completing entries currently rewrite the normalized file. If you want in-place single-line edits that preserve file order, I can implement a safer in-place update that maps printed indices to physical lines.

---

## Next steps / Suggested improvements

- Add a non-interactive `--set-default --create` mode to create the file automatically without prompting.
- Make the saved-config file path configurable via `--config` or an environment variable.
- Add structured `add` flags (`--date`, `--desc`, `--tags`).
- Add unit tests for the parser and query logic (`split_on_tab_or_spaces`, `parse_line`, tag matching).
- Add optional output formats (JSON/CSV) and more flexible table layout.

If you want any of these implemented, tell me which one and I will make the change and run the build/tests.
