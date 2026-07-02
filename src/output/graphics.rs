use std::env;
use std::io::{self, Write};

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::gfx::Size;

/// Renders the browser framebuffer to the terminal using the kitty graphics
/// protocol: https://sw.kovidgoyal.net/kitty/graphics-protocol/
///
/// The default renderer downsamples every 2x4 pixel block to a single terminal
/// cell and quantizes it to two colors (see `quad.rs`), which is what makes the
/// output look blocky. On terminals that implement the kitty graphics protocol
/// we can instead hand the raw framebuffer to the terminal, which composites it
/// at full pixel resolution — crisp text and images, straight from Blink.
///
/// This requires the page (including text) to be present in the framebuffer,
/// so graphics mode implies bitmap mode (see `Bridge::BitmapMode`).
pub struct KittyGraphics {
    /// Latest framebuffer, packed as RGB (3 bytes per pixel).
    rgb: Vec<u8>,
    /// Size of the framebuffer in pixels.
    pixels: Size,
    /// Whether the framebuffer changed since the last `encode`.
    dirty: bool,
    /// Rotating counter for unique temp-file names.
    counter: u64,
    /// Recently written temp files, reaped once the terminal has read them.
    recent: std::collections::VecDeque<String>,
    /// Send pixels inline (t=d) instead of through a temp file. The file
    /// handoff only works when the terminal can read our filesystem; over
    /// SSH (or a tmux session that may be attached remotely) it cannot, and
    /// silently renders nothing.
    direct: bool,
    /// Wrap every graphics escape in a tmux passthrough envelope. tmux
    /// discards raw APC sequences even with allow-passthrough enabled; it
    /// only forwards them inside its own DCS wrapper.
    tmux: bool,
}

impl KittyGraphics {
    pub fn new() -> Self {
        let term = env::var("TERM").unwrap_or_default();
        let tmux =
            env::var("TMUX").is_ok() || term.starts_with("tmux") || term.starts_with("screen");
        let ssh = env::var("SSH_CONNECTION").is_ok()
            || env::var("SSH_CLIENT").is_ok()
            || env::var("SSH_TTY").is_ok();
        KittyGraphics {
            rgb: Vec::new(),
            pixels: Size::new(0, 0),
            dirty: false,
            counter: 0,
            recent: std::collections::VecDeque::new(),
            direct: ssh || tmux,
            tmux,
        }
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Store a new framebuffer. `pixels` is BGRA8888, as handed to us by
    /// Chromium in `Renderer::draw_background`.
    pub fn store(&mut self, pixels: &[u8], size: Size) {
        let count = (size.width as usize) * (size.height as usize);

        if count == 0 || pixels.len() < count * 4 {
            return;
        }

        self.rgb.clear();
        self.rgb.reserve(count * 3);

        for i in 0..count {
            // Source is BGRA, kitty's RGB format (f=24) wants R, G, B.
            self.rgb.push(pixels[i * 4 + 2]);
            self.rgb.push(pixels[i * 4 + 1]);
            self.rgb.push(pixels[i * 4 + 0]);
        }

        self.pixels = size;
        self.dirty = true;
    }

    /// tmux only forwards escapes wrapped in its passthrough DCS, with every
    /// ESC in the payload doubled.
    fn wrap(&self, seq: &[u8], out: &mut Vec<u8>) {
        if !self.tmux {
            out.extend_from_slice(seq);
            return;
        }

        out.extend_from_slice(b"\x1bPtmux;");
        for &byte in seq {
            if byte == 0x1b {
                out.push(0x1b);
            }
            out.push(byte);
        }
        out.extend_from_slice(b"\x1b\\");
    }

    /// Tell the terminal to forget our image and its data. Used on resize so a
    /// stale placement isn't left scaled to the wrong size. Scoped to image
    /// id 1 (`d=I,i=1`) so we never touch images other programs may have drawn.
    pub fn reset(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.wrap(b"\x1b_Ga=d,d=I,i=1\x1b\\", &mut out);
        out
    }

    /// Encode the escape codes that draw the framebuffer into a `cols`x`rows`
    /// cell box anchored at the 1-based terminal cell (`col`, `row`). The cursor
    /// is saved and restored so the navigation caret is left untouched.
    ///
    /// Locally the frame is handed over through a temporary file (`t=t`) to
    /// keep multi-MB payloads off the pty. In direct mode it is streamed
    /// inline (`t=d`) in 4096-byte base64 chunks. Both paths zlib-compress
    /// (`o=z`) first: pages are mostly flat color and shrink 10-20x, which
    /// direct mode needs to stay responsive over SSH.
    pub fn encode(&mut self, col: u32, row: u32, cols: u32, rows: u32) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();

        self.dirty = false;

        if self.pixels.width == 0 || self.pixels.height == 0 {
            return Ok(out);
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&self.rgb)?;
        let payload = encoder.finish()?;

        // Save cursor, move to the content origin.
        write!(out, "\x1b7\x1b[{};{}H", row, col)?;

        // a=T  transmit and display
        // f=24 raw RGB
        // o=z  payload is zlib-compressed
        // s,v  source size in pixels
        // c,r  number of cells to scale the image into
        // i=1  image id (reused each frame, so the terminal replaces the data)
        // p=1  placement id (reused, so the terminal replaces the placement)
        // q=2  suppress the terminal's success/error replies
        // C=1  do not move the cursor when placing the image
        let control = format!(
            "a=T,f=24,o=z,s={},v={},c={},r={},i=1,p=1,q=2,C=1",
            self.pixels.width, self.pixels.height, cols, rows
        );

        if self.direct {
            let b64 = base64(&payload);
            let bytes = b64.as_bytes();
            let mut offset = 0;
            let mut first = true;

            // kitty caps each escape's payload at 4096 bytes; m=1 announces
            // more chunks, the last one carries m=0.
            while offset < bytes.len() {
                let end = (offset + 4096).min(bytes.len());
                let more = if end < bytes.len() { 1 } else { 0 };
                let mut seq = Vec::new();

                if first {
                    first = false;
                    write!(seq, "\x1b_G{},t=d,m={};", control, more)?;
                } else {
                    write!(seq, "\x1b_Gm={};", more)?;
                }

                seq.extend_from_slice(&bytes[offset..end]);
                seq.extend_from_slice(b"\x1b\\");
                self.wrap(&seq, &mut out);

                offset = end;
            }
        } else {
            self.counter = self.counter.wrapping_add(1);
            let dir = env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
            // The name must contain "tty-graphics-protocol" for kitty to delete
            // the file itself after reading it.
            let path = format!(
                "{}/carbonyl-tty-graphics-protocol-{}-{}.z",
                dir.trim_end_matches('/'),
                std::process::id(),
                self.counter
            );

            std::fs::write(&path, &payload)?;

            // Reap files from earlier frames in case the terminal didn't (e.g.
            // it isn't kitty, or doesn't honor t=t deletion). By now they've
            // been read.
            self.recent.push_back(path.clone());
            while self.recent.len() > 2 {
                if let Some(old) = self.recent.pop_front() {
                    let _ = std::fs::remove_file(old);
                }
            }

            let mut seq = Vec::new();
            write!(seq, "\x1b_G{},t=t;{}\x1b\\", control, base64(path.as_bytes()))?;
            self.wrap(&seq, &mut out);
        }

        // Restore the cursor.
        write!(out, "\x1b8")?;

        Ok(out)
    }
}

/// Standard base64 (RFC 4648) encoder. Kept inline to avoid pulling in a crate.
fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((*chunk.get(1).unwrap_or(&0) as u32) << 8)
            | (*chunk.get(2).unwrap_or(&0) as u32);

        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }

    out
}
