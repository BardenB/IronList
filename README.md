
# IronList

A small CLI tool for managing a simple date-tagged to-do list stored in a plain text file.

---

## Summary

IronList stores each entry as a single, normalized line:

```
YYYY-MM-DD    Description    tag1,tag2
```

- Input parsing accepts literal TAB characters or runs of 4+ spaces as field separators (helpful when shells make typing tabs awkward).
- On write (when adding or editing) the program normalizes entries to the canonical tab-separated format.
- Tags are an optional comma-separated list in the third field; queries match tags case-insensitively by default.

---

## Build

You need Rust and Cargo installed, which I assume will be done.

```
# build the project (crate folder)
cd iron-list
cargo build --release
```

For day-to-day development, use `cargo build`.

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

- `--show-all` — When provided, the program will include entries tagged `complete` in the output. By default completed entries are omitted from the main list and, when `--show-all` is used, they are printed in a separate "Completed:" table below the main list.

---

## Commands

### list (default)

```powershell
cargo run -- list
```

Prints a three-column table with headers and wrapped task descriptions:

- Column 1: `No` — item number (right-aligned)
- Column 2: `Date` — `YYYY-MM-DD`
- Column 3: `Task` — description
- Column 4: `Tags` — comma-separated tags

The output is sorted by date ascending. Multi-line task descriptions are printed with continuation lines aligned under the `Task` column.

Behavior regarding completed items:
- By default entries tagged `complete` are not shown in the main table.
- If you pass `--show-all`, the program prints two tables: first the incomplete items (numbered), then a second labeled `Completed:` containing completed items (also numbered independently).

Example showing completed items in a second table:

```
 No   Date        Task                           Tags
---  ----------  ------------------------------  --------------------
  1. 2025-09-19  out of order test               uhoh
  2. 2025-10-19  Email someone                   home,priority,testing

Completed:
 No   Date        Task                           Tags
---  ----------  ------------------------------  --------------------
  1. 2025-10-19  Go to Kroger                    groceries,home,complete
```

### add

```
cargo run -- add "<LINE>"
```

Append a new entry. `LINE` must contain at least a date and a description. Expected input example:

```
YYYY-MM-DD    Description    tag1,tag2
```

Because tabs are inconvenient in some shells the parser also accepts runs of 4+ spaces as separators. Valid example:


```
cargo run -- add "2025-10-18    Buy iron    tools,home"
```

On `add`, the program validates the date and presence of a description. If valid it writes a normalized tab-separated line to disk.

### edit

```
cargo run -- edit <INDEX> "<LINE>"
```

Replace the numbered entry shown by `list` with the provided normalized line. The replacement is validated before being written. At the moment the program rewrites the file with normalized entries when editing.

### complete

```
cargo run -- complete <INDEX>
```

Mark the chosen (numbered) entry as complete by adding a `complete` tag (case-insensitive check prevents duplicates). This operation currently rewrites the normalized file.

### query

```
cargo run -- query [--from DATE] [--to DATE] [--date DATE] [--any] [--tag TAG]...
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
- Tags are case-insensitive.

Example:

```
cargo run -- query --date 2025-10-18 --tag work --tag urgent
```

---

## File format details

- Each entry is a single line: `YYYY-MM-DD    Description    tag1,tag2`.
- Accepted input separators when parsing: literal `\t` or runs of 4+ spaces (suggested to use 4+ spaces).
- On write, entries are normalized to the canonical tab-separated form.
- Tag matching for queries is case-insensitive; stored tags preserve the user's casing.

---

## Examples (quick)

Append:

```
cargo run -- add "2025-10-18    Buy iron    tools,home"
```

Show saved default:

```
cargo run -- --show-default
```

Set default (creates file if you confirm):

```
cargo run -- --set-default path\to\ironlist.txt
```

Clear saved default:

```
cargo run -- --set-default -
```

List everything (no subcommand required):

```
cargo run --
```

Query with OR tags:

```
cargo run -- query --any --tag personal --tag errands
```

---

## Troubleshooting & notes

- If the program errors while reading the data file at startup, verify the selected file exists and is readable.
- The program prefers `--file` only when the provided path exists at startup; otherwise the persisted default is used.

---

## Next steps / Suggested improvements

- Add a non-interactive `--set-default --create` mode to create the file automatically without prompting.
- Make the saved-config file path configurable via `--config` or an environment variable.
- Add unit tests for the parser and query logic (`split_on_tab_or_spaces`, `parse_line`, tag matching).


