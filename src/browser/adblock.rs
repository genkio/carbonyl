use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

use flate2::read::GzDecoder;

use crate::utils::log;

/// Ad/tracker blocking through a local filtering proxy: chromium is pointed
/// at it with --proxy-server, so blocked hosts fail before any DNS lookup or
/// connection leaves the machine. A proxy instead of --host-resolver-rules
/// because Linux caps a single exec argument at 128KiB, which fits ~2k
/// domains; the bundled list has ~230k.
///
/// List: HaGeZi Multi PRO (ads, tracking, telemetry, phishing), wildcard
/// domains variant, fetched 2026-07-02.
/// https://github.com/hagezi/dns-blocklists (GPL-3.0)
const LIST_GZ: &[u8] = include_bytes!("adblock.list.gz");

/// Start the proxy on an ephemeral port and return it.
pub fn serve() -> io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let blocked: &'static [&'static str] = load_list();

    thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(conn) = conn else { continue };

            thread::spawn(move || {
                if let Err(error) = handle(conn, blocked) {
                    log::debug!("adblock proxy connection error: {error}")
                }
            });
        }
    });

    Ok(port)
}

/// Leaked on purpose: the list lives for the whole process and a &'static
/// slice avoids refcounting across connection threads.
fn load_list() -> &'static [&'static str] {
    let mut text = String::new();

    if let Err(error) = GzDecoder::new(LIST_GZ).read_to_string(&mut text) {
        log::error!("adblock: failed to decompress blocklist: {error}");
    }

    let text: &'static str = Box::leak(text.into_boxed_str());
    let mut list: Vec<&'static str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    list.sort_unstable();
    list.dedup();

    Box::leak(list.into_boxed_slice())
}

/// Suffix walk so subdomains of a listed domain are blocked too.
fn is_blocked(host: &str, list: &[&str]) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let mut suffix = host.as_str();

    loop {
        if list.binary_search(&suffix).is_ok() {
            return true;
        }

        match suffix.find('.') {
            Some(dot) => suffix = &suffix[dot + 1..],
            None => return false,
        }
    }
}

fn handle(client: TcpStream, blocked: &[&str]) -> io::Result<()> {
    let mut reader = BufReader::new(client.try_clone()?);
    let mut request_line = String::new();

    reader.read_line(&mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let target = parts.next().unwrap_or("").to_owned();

    if method == "CONNECT" {
        drain_headers(&mut reader)?;

        let host = target.rsplit_once(':').map(|(h, _)| h).unwrap_or(&target);

        if is_blocked(host, blocked) {
            return refuse(&client);
        }

        let upstream = match TcpStream::connect(&target) {
            Ok(upstream) => upstream,
            Err(_) => return refuse(&client),
        };

        (&client).write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
        splice(reader, client, upstream)
    } else {
        // Absolute-form request: METHOD http://host[:port]/path HTTP/1.1
        let rest = target.strip_prefix("http://").unwrap_or(&target);
        let (host_port, path) = match rest.find('/') {
            Some(slash) => (&rest[..slash], &rest[slash..]),
            None => (rest, "/"),
        };
        let host = host_port.rsplit_once(':').map(|(h, _)| h).unwrap_or(host_port);

        if is_blocked(host, blocked) {
            return refuse(&client);
        }

        let addr = if host_port.contains(':') {
            host_port.to_owned()
        } else {
            format!("{host_port}:80")
        };
        let mut upstream = match TcpStream::connect(&addr) {
            Ok(upstream) => upstream,
            Err(_) => return refuse(&client),
        };

        write!(upstream, "{method} {path} HTTP/1.1\r\n")?;

        // Connection: close forced because a kept-alive proxy connection may
        // be reused for a different host, which this one-host-per-connection
        // forwarder can't route.
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line)?;

            let lower = line.to_ascii_lowercase();
            if lower.starts_with("connection:") || lower.starts_with("proxy-connection:") {
                continue;
            }
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }

            upstream.write_all(line.as_bytes())?;
        }
        upstream.write_all(b"Connection: close\r\n\r\n")?;

        splice(reader, client, upstream)
    }
}

fn drain_headers(reader: &mut BufReader<TcpStream>) -> io::Result<()> {
    let mut line = String::new();

    loop {
        line.clear();

        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            return Ok(());
        }
    }
}

fn refuse(mut client: &TcpStream) -> io::Result<()> {
    client.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")?;
    client.shutdown(Shutdown::Both)
}

/// Bidirectional copy until either side closes. `reader` may hold bytes
/// already buffered past the headers; they go upstream first.
fn splice(reader: BufReader<TcpStream>, client: TcpStream, upstream: TcpStream) -> io::Result<()> {
    let mut up_writer = upstream.try_clone()?;
    let client_writer = client;

    let uplink = thread::spawn(move || {
        let mut reader = reader;
        let _ = io::copy(&mut reader, &mut up_writer);
        let _ = up_writer.shutdown(Shutdown::Write);
    });

    let mut down_reader = upstream;
    let mut down_writer = client_writer;
    let _ = io::copy(&mut down_reader, &mut down_writer);
    let _ = down_writer.shutdown(Shutdown::Both);
    let _ = uplink.join();

    Ok(())
}
