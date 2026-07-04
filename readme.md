<table align="center">
  <tbody>
    <tr>
      <td>
        <p></p>
        <pre>
   O    O
    \  /
O —— Cr —— O
    /  \
   O    O</pre>
      </td>
      <td><h1>Carbonyl<br><sub>genkio fork</sub></h1></td>
    </tr>
  </tbody>
</table>

Carbonyl is a Chromium based browser that runs in a terminal. It renders real
web pages (WebGL, WebGPU, audio/video, animations) to terminal cells, starts in
under a second, and works over SSH with no window server.

> **This is a fork of [`fathyb/carbonyl`](https://github.com/fathyb/carbonyl).**
> For the original project description, demos, comparisons (Lynx, Browsh), and
> the full story, see the [upstream README](https://github.com/fathyb/carbonyl#readme)
> and [blog post](https://fathy.fr/carbonyl). This README documents only what
> the fork adds on top.

## What this fork adds

Since forking at upstream `v0.0.3`, the changes cluster into two themes:
**terminal-native navigation & rendering**, and **distribution**.

| Area | Flag / entry point | What it does |
| ---- | ------------------ | ------------ |
| Vim navigation | `--vim` | Vimium-style keys: scroll, history, find-on-page, link hints. Opt-in. |
| Full-res graphics | `-g`, `--graphics` | Draw the page at full resolution via the kitty graphics protocol, inline over SSH and tmux. |
| Hybrid renderer | `--vim --graphics` | Full-res image *under* a terminal-text layer, so hints/find/status bar keep working. |
| Ad blocking | `--adblock` | Local filtering proxy backed by HaGeZi's ~230k-domain list. |
| Image blocking | `--no-images` | Stop all image downloads at the Blink layer (bandwidth saving). |
| Homebrew install | `brew install genkio/tap/carbonyl` | Single self-contained binary, no Docker or npm. |
| Single-file binary | `scripts/bundle.sh` / `tools/bundler` | Embed the whole runtime into one launcher that self-extracts on first run. |
| Fast Rust iteration | `scripts/dev-install.sh` | Rebuild `libcarbonyl` and drop it into an existing install in seconds. |

Everything is opt-in: with no new flags, this behaves like upstream Carbonyl.

## Install

### Homebrew (recommended)

```console
$ brew install genkio/tap/carbonyl
$ carbonyl https://github.com
```

Ships a single self-contained binary. First launch extracts the runtime
(~165 MB) to `$XDG_CACHE_HOME/carbonyl/<hash>/`; later launches `exec` straight
into it. Builds are published for macOS arm64, macOS x86_64, and Linux x86_64.

### Docker / npm (upstream)

The upstream distribution channels still work:

```console
$ docker run --rm -ti fathyb/carbonyl https://youtube.com
$ npm install --global carbonyl && carbonyl https://github.com
```

## New flags

```
    --adblock       block ads and trackers (bundled HaGeZi Pro list, ~230k domains)
    --no-images     block all image downloads (disables images in Blink)
-g, --graphics      render at full resolution using the kitty graphics protocol
    --vim           enable vimium-style keyboard navigation
```

All four also read an environment variable (`CARBONYL_ENV_ADBLOCK`,
`CARBONYL_ENV_NO_IMAGES`, `CARBONYL_ENV_GRAPHICS`, `CARBONYL_ENV_VIM`), handy for
Docker or shell aliases. Upstream flags (`-f/--fps`, `-z/--zoom`, `-b/--bitmap`,
`-d/--debug`, and most Chromium options) are unchanged.

## Keyboard navigation (vimium-style)

An opt-in keyboard layer modelled on [Vimium](https://github.com/philc/vimium).
Enable it with `--vim` (or `CARBONYL_ENV_VIM=1`); it is off by default so every
key reaches the page as usual. When enabled and the URL bar is unfocused, the
keys below are handled by Carbonyl rather than forwarded to the page; press `i`
(Insert mode) before typing into page inputs. Arrows, Enter and Tab always pass
through.

### Scrolling

| Key  | Action                       |
| ---- | ---------------------------- |
| `j`  | Scroll down one line         |
| `k`  | Scroll up one line           |
| `d`  | Scroll down half a page      |
| `u`  | Scroll up half a page        |
| `gg` | Scroll to the top            |
| `G`  | Scroll to the bottom         |
| `h`  | Send a Left arrow to the page (useful for carousels / horizontal scroll containers) |
| `l`  | Send a Right arrow to the page |

### History

| Key        | Action      |
| ---------- | ----------- |
| `H` or `[` | Go back     |
| `L` or `]` | Go forward  |
| `r`        | Reload page |

Going back with no history left quits Carbonyl, so `[` (or `H`) also works as a
quick way to close the browser.

### Find on page

| Key   | Action                                       |
| ----- | -------------------------------------------- |
| `/`   | Open the find prompt (status bar at bottom)  |
| Enter | Confirm the query and highlight matches      |
| `n`   | Cycle to the next match                      |
| `N`   | Cycle to the previous match                  |
| `Esc` | Clear the active query and highlights        |

Find searches the currently visible viewport only, ASCII case-insensitive.

### Link hints

| Key   | Action                                                                |
| ----- | --------------------------------------------------------------------- |
| `f`   | Show hint labels on the candidate clickables in the current viewport  |
| `Esc` | Cancel hint mode                                                      |

Type the label letters shown next to a target to send a synthetic click at that
position. Single-letter labels are used when there are 26 or fewer candidates;
longer labels appear otherwise. The status bar shows how many candidates were
found and what you have typed so far.

Because Carbonyl does not currently expose the DOM to the Rust side, link
candidates are detected from rendered text (short isolated runs, or runs whose
colour differs from the dominant body colour). The label set is therefore a
superset of the real anchors on the page: clicking a label that lands on plain
text is harmless and simply does nothing.

### Modes

| Mode    | How to enter                                            | How to leave |
| ------- | ------------------------------------------------------- | ------------ |
| Normal  | Default whenever the URL bar is unfocused                | n/a          |
| Insert  | `i`. Keys are passed through to the page (typing, etc.) | `Esc`        |
| Find    | `/`                                                     | `Esc` or `Enter` |
| Hint    | `f`                                                     | `Esc`, type a unique label, or type a prefix with no remaining matches |

`Esc` in Normal mode also clears any leftover find highlights and pending
prefixes (e.g. a half-typed `g`).

## Full-resolution graphics (`--graphics`)

`-g` / `--graphics` renders the page at full pixel resolution using the
[kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
instead of the block-character approximation. It streams the image inline (so it
works over SSH) and wraps escape sequences for tmux passthrough. The image is
deleted on exit so it does not linger in the scrollback.

Combining `--vim` with `--graphics` switches to a **hybrid renderer**: the page
image is drawn at full resolution under the text layer, while text comes in as
terminal glyphs, so hints, find and the status bar keep working on top of it.
Pass `--bitmap` as well to force the full-bitmap image instead; hints and find
have no text to scan there and are effectively disabled.

## Ad blocking (`--adblock`)

`--adblock` blocks ads and trackers by hostname. The launcher starts a local
filtering proxy and points chromium at it, so requests to listed hosts are
refused before any DNS lookup or connection leaves the machine. The bundled list
is [HaGeZi's Multi PRO](https://github.com/hagezi/dns-blocklists) (~230k domains
covering ads, tracking, telemetry, phishing and scam hosts, GPL-3.0). Besides
removing most display ads it stops their animations, which otherwise force
constant repaints; in `--graphics` mode over SSH that is the difference between a
calm page and permanent flashing.

Hostname blocking has no cosmetic filtering: an empty slot may remain where an ad
would have been, and first-party ads (e.g. YouTube's own) still load. A
user-supplied `--proxy-server` takes precedence and disables `--adblock`.

## Blocking images (`--no-images`)

`--no-images` stops all images from loading. It disables images in Blink
(`--blink-settings=imagesEnabled=false`), so the resource fetcher rejects every
image request (both `<img>` and CSS backgrounds) before it hits the network:
nothing is downloaded and then hidden, the fetch never happens. Since Carbonyl
renders images as low-resolution blocks anyway, this trades a little visual
context for a large bandwidth saving, useful on slow or metered connections.
Text, layout and links are unaffected.

## Known issues

- Fullscreen mode not supported yet

## Building

Carbonyl is split into the "core" (a Rust shared library, `libcarbonyl`) and the
"runtime" (a modified Chromium headless shell that loads the core). Only Rust
changes are needed for the features in this fork; the runtime is unchanged from
upstream. See the [upstream Contributing guide](https://github.com/fathyb/carbonyl#contributing)
for the full (multi-hour) Chromium build.

### Iterating on the Rust core

```console
$ cargo build
```

If you already have a working Carbonyl install (e.g. an unpacked release at
`~/Downloads/carbonyl-0.0.3`), `scripts/dev-install.sh` rebuilds
`libcarbonyl.dylib` and drops it into that install, so you can re-launch and test
in seconds instead of rebuilding the runtime:

```console
$ ./scripts/dev-install.sh
# point at a different install with:
$ CARBONYL_INSTALL_DIR=/path/to/install ./scripts/dev-install.sh
```

The script picks the target triple from the installed dylib's arch and sets the
install name to `@executable_path/libcarbonyl.dylib`.

### Producing a portable single-file binary

`scripts/bundle.sh` embeds the runtime files (`carbonyl`, the platform
libs/dylibs, `icudtl.dat`, the v8 snapshot) into a launcher binary at
`dist/carbonyl`. The target triple is auto-detected from the payload (macOS
arm64/x86_64 or Linux x86_64). On first run the launcher extracts to
`$XDG_CACHE_HOME/carbonyl/<hash>/` and `exec`s the real binary; later launches
`exec` directly. The `<hash>` is a sha256 prefix of the payload, so a new build
always gets a fresh cache directory.

```console
$ ./scripts/dev-install.sh   # make sure libcarbonyl.dylib is up to date
$ ./scripts/bundle.sh        # produces dist/carbonyl (~158 MB)
$ cp dist/carbonyl ~/.local/bin/carbonyl
```

Override the source install with `CARBONYL_INSTALL_DIR=...`. The bundler crate
lives under `tools/bundler/`.

### Release workflow

`.github/workflows/release.yml` builds all three targets (macOS arm64, macOS
x86_64, Linux x86_64). For each it downloads the upstream `v0.0.3` payload,
rebuilds `libcarbonyl` from source and injects it, then produces a single-file
launcher via the bundler. Those launchers are what the Homebrew tap installs.

## Credits & license

All credit for Carbonyl itself goes to [Fathy Boundjadj](https://github.com/fathyb)
and the upstream [`fathyb/carbonyl`](https://github.com/fathyb/carbonyl)
contributors. This fork keeps the upstream license; the bundled ad-block list is
[HaGeZi's Multi PRO](https://github.com/hagezi/dns-blocklists) (GPL-3.0).
