
# IronList

A small CLI tool for managing a simple date-tagged to-do list stored in a plain text file.

---

## Summary

IronList stores each entry as a single, normalized line:

```
YYYY-MM-DD    Description    tag1,tag2
```

- Input parsing uses 4+ spaces as field separators (supposed to be able to use literal tabs, but does not currently work as intended across all terminals).
- When adding or editing a task, the program normalizes entries to tab-separated format. i.e. the text file itself uses tabs even though input in CLI is spaces.
- Tags are an optional comma-separated list in the third field; tags are case-insensitive when using the query option.

---

## Build

You need Rust and Cargo installed, which 

For day-to-day development, use `cargo build`. For fully optimized `.exe` file, add the `--release` flag.

For help with Rust, see its [documentation here.](https://doc.rust-lang.org/stable/book/title-page.html)

---

## Data file selection and configuration

IronList chooses a data file using the following precedence:

1. The program uses a persisted defaulted file path (created by the program on first run or set with `--set-default`).
    - Preferred: `$HOME/.ironlist_default` (the user's home directory).
    - Fallback: `./.ironlist_default` in the current working directory.
2. If no persisted default exists the program prompts you to enter one and saves it.
3. If you pass `-f/--file <PATH>` and that path exists at startup, it is used.

Commands to manage the saved default:
- `--set-default <PATH>` — saves the provided path and exits. If the path does not exist the program prompts to create it. 
- Passing `-` (a single dash) clears the saved default.
- `--show-default` — prints the currently saved default (or `No saved default`) and exits.

Examples:

```
# set default path (prompts to create file if missing)
cargo run -- --set-default C:\path\to\example.txt

# clear saved default
cargo run -- --set-default -

# show saved default
cargo run -- --show-default
```

Notes:
- The program reads the selected file at startup. If the final selected path does not exist, the program will error when reading entries.
- You can still use `--file` to temporarily point to a different file (only used if the path exists at startup).

---

## Usage

Top-level flags work without providing a subcommand. If no subcommand is given the default action is `list`.

```
# show help
cargo run -- --help

# list (no subcommand required)
cargo run --

# run a command explicitly
cargo run -- <command> [options]
```

Global options
- `-f`, `--file <FILE>` — Path to the to do file. The program will use this path only if it exists at startup; otherwise the persisted default will be used.

- `--show-all` — When provided, the program will include entries tagged `complete` in the output. By default completed entries are omitted from the main list.

---

## Commands

### list (default)

```
cargo run -- list
```

Prints a three-column table with headers and wrapped task descriptions:

- Column 1: `No.` — item number (right-aligned)
- Column 2: `Date` — `YYYY-MM-DD`
- Column 3: `Task` — description
- Column 4: `Tags` — comma-separated tags

The output is sorted by date ascending. Multi-line task descriptions are printed with continuation lines aligned under the `Task` column.

#### Completed items:
- By default, entries tagged `complete` are not shown in the main table.
- If you pass `--show-all`, the program prints two tables: first the incomplete items (numbered), then a second labeled `Completed:` containing completed items. 
> [!NOTE] There is no current way to add a due date column, but could be added as a tag.
> example: "2025-01-01    Go to grocery store    groceries, complete, 2025-01-01"

Example showing completed items in a second table:

```
 No   Date        Task                           Tags
---  ----------  ------------------------------  --------------------
  1. 2025-09-19  Make Dinner                     food, groceries
  2. 2025-10-19  Email someone                   home,priority,testing

Completed:
 No   Date        Task                           Tags
---  ----------  ------------------------------  --------------------
  1. 2025-09-19  Go to Kroger                    groceries,home,complete
```

### add

```
cargo run -- add "<LINE>"
```

Append a new entry. `LINE` must contain at least a date and a description. Expected input example:

```
YYYY-MM-DD    Description    tag1,tag2
```

Because tabs are inconvenient in some shells the parser also accepts runs of 4+ spaces as separator. As of 2025-10-28, literal tab characters does not work at all.

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

Mark the chosen (numbered) entry as complete by adding a `complete` tag (case-insensitive check prevents duplicates). This command rewrites the .txt file.

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

The follwoing example will return all entries on 2025-10-18 with both work and urgent tags:

```
cargo run -- query --date 2025-10-18 --tag work --tag urgent
```

---

## File format details

- Each entry is a single line: `YYYY-MM-DD    Description    tag1,tag2`.
- Accepted input separators when parsing: literal `\t` or runs of 4+ spaces (suggested to use 4+ spaces as literal tabs do not work in most if not all places).
- `--add` and `--edit` convert 4 space separators in the terminal to tabs in the .txt file.
- Tag matching for queries is case-insensitive.

- I would like to add a list that is undated. It would not be sorted by date, then, and instead by whatever the .txt file says. Everything else would function normally.

---

## Examples (quick)

Append:

```
cargo run -- add "2025-10-18    Buy iron rod    tools,home"
```

Show saved default:

```
cargo run -- --show-default
```

Set default:

```
cargo run -- --set-default assets\example.txt
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

- Add unit tests for all commands and options.
- Refactor, reorganize, de-duplicate, and get rid of the AI inefficiencies in this code.
- Add multiple entires at once
- Undated list for tasks that do not need a due date, but should still be written down.
- `cargo run -- list --show-all` does not work? 


