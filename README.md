
# IronList

A minimalist CLI tool for managing a simple date-tagged to-do list stored in a plain text file.

---

## Summary

IronList stores each entry as a single, normalized line:

```
YYYY-MM-DD    Description    tag1,tag2
```

- Input parsing accepts literal TAB characters `\t` or runs of 4+ spaces as field separators. Input using spaces is recommended.
- On write (when adding or editing) the program normalizes entries to the canonical tab-separated format.
- Tags are an optional comma-separated list in the third field; queries match tags.

---

## Build

You need Rust and Cargo installed, which I assume will already be done.

```
# build the project (crate folder)
cd iron-list
cargo build --release
```

For day-to-day development, use `cargo build`.

---

## Data File Configuration

IronList chooses a data file using the following operations:

1. On first run (or set with `--set-default`), a persisted default path is established.
2. Optionally, `-f/--file <PATH>` can be used if that path exists at startup. This will override the default path for that run only.
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
- `--from <DATE>` - Inclusive start date (YYYY-MM-DD).
- `--to <DATE>` - Inclusive end date (YYYY-MM-DD).
- `--date <DATE>` - Shorthand exact-date match (sets both `from` and `to`).
- `--tag <TAG>` - Repeatable tag filter (case-insensitive).
- `--any` - Switch tag filtering from AND (default) to OR semantics.

Behavior notes:
- Date filtering is inclusive and combined with tag filtering.
- Tags are case-insensitive.

Example:

```
cargo run -- query --date 2025-10-18 --tag work --tag urgent
```

### notify

```
cargo run -- notify [-- time HH:MM] [--interval MM] [--install] [--uninstall]
```

Options:
- `--time <HH:MM>` - sets the time of day for the notification to be sent to the computer. Note: it must be in 24 hour time, with a leading zero. e.g. 08:30, 16:45.
- `--interval <MM>` - should notify every MM minutes. I don't think it works.
- `--install` - if used, will allow the notification to pop up in the background without keeping the terminal occupied.
- `--uninstall` - if used, will uninstall the scheduled notification.

if `--install` is not used when making a schedule, notify will stay active in the terminal, not in the background. `CTRL + C` is the only way to end that.

#### Windows notifications
The `notify` system has only been tested on Windows 11 thus far. 

`--interval MM` does not work as intended. Because the terminal appears and minimizes, the next notification will not show up. Only if you close the terminal that appears will this work as intended. If the time period has elapsed with the terminal open, and then it is closed, the notification will appear on time. This issue is not expected to occur on macOS or linux systems.

---

## File format details

- Each entry is a single line: `YYYY-MM-DD    Description    tag1,tag2`.
- Accepted input separators when parsing: literal `\t` or runs of 4+ spaces (suggested to use 4+ spaces).
- On write, entries are normalized to the canonical tab-separated form.
- Tag matching for queries is case-insensitive; stored tags preserve the user's casing.

---

## Examples

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


---

## Future plans

- Continue adding unit tests.
- Test cross-platform compatibility
- Exit terminal window on Windows `notify` work.
- Refactor, organize, de-duplicate, get rid of the AI inefficiencies in this code.


