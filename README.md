# hypsnip

Rust implementation of [steipete/Trimmy](https://github.com/steipete/Trimmy) for Waybar/Hyprland and, more generally, Wayland compositors.

## What it does

- Lives in your Waybar or in the terminal (pipe to `hyprsnip`)
- Watches the clipboard, and, when it looks like a shell command, removes newlines (respects `\` continuations) and rewrites the clipboard automatically
- Strips leading shell prompts (`#`/`%`/`>`/`$`) when the line looks like a command, while leaving Markdown headings untouched
- Aggressiveness levels (Low/Normal/High) to control how eagerly it detects commands:
  - Low: only flattens when it’s obviously a command. Example: a long kubectl ... | jq ... multi-line snippet.
  - Normal (default): balances caution and helpfulness. Example: a brew update \ && brew upgrade copy from a blog post.
  - High: flattens almost any multi-line text that could be a command. Example: a quick two-line ls + cd copied from chat.
- Optional "Keep blank lines" so scripts with intentional spacing stay readable.
- Optional "Remove box drawing chars (│┃)" to strip prompt-style gutters (any count, leading or trailing) and collapse the leftover whitespace.
- "Paste Trimmed" button + hotkey trims on-the-fly and pastes without permanently altering the clipboard (uses High aggressiveness); shows the target app (e.g., “Paste Trimmed to Ghostty”) and strikes out removed chars in the preview.
- "Paste Original" button + hotkey pastes the untouched copy even after auto-trim.
- Optional "Launch at login" toggle (installs systemd user service).
- Uses a marker pasteboard type to avoid reprocessing its own writes; polls with a lightweight timer and a small grace delay to catch promised pasteboard data.
- Safety valve: skips auto-flatten if the copy is more than 10 lines (even on High) to avoid mangling big blobs.

## Aggressiveness levels & examples

- **Low (safer)** — needs strong command cues (pipes, redirects, continuations).  
  Before:
  ```
  ls -la \
    | grep '^d' \
    > dirs.txt
  ```
  After: `ls -la | grep '^d' > dirs.txt`
- **Normal (default)** — README/blog-ready: handles typical multi-line commands with flags.  
  Before:
  ```
  kubectl get pods \
    -n kube-system \
    | jq '.items[].metadata.name'
  ```
  After: `kubectl get pods -n kube-system | jq '.items[].metadata.name'`
- **High (eager)** — flattens almost anything command-shaped, plus the manual “Paste Trimmed” hotkey always uses this level.  
  Before:
  ```
  echo "hello"
  print status
  ```
  After: `echo "hello" print status`
- **Prompt cleanup** — copies that start with `# ` or `$ ` are de-promoted when they look like shell commands, e.g. `# brew install foo` → `brew install foo`; Markdown headings like `# Release Notes` remain untouched.

  Before:

  ```
    ┃  Can you plan out the application @README.md but in
  ┃  rust?
  ```

  After:

  ```
  Can you plan out the application @README.md but in rust?
  ```

## Tech

- Rust
- wl-copy

## Arch

- Crates:
  - hyprsnip-utils
  - hyprsnip-cli
  - hyprsnip-config

## Test it out

```bash
# with some text in your clipboard (just copy below)
wl-paste \
  | cargo run -p hyprsnip-cli -- trim --keep-blank-lines --max-auto-lines 0 \
                        | wl-copy
```
