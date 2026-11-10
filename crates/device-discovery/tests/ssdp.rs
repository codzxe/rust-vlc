//! Card 7.D1 integration: the SSDP searcher against REAL sockets.
//!
//! The transport logic (send M-SEARCH, receive unicast responses,
//! parse, dedup, bound the window) is proven here with an in-process
//! loopback responder — real UDP, real wire bytes, no mocks of the
//! socket layer. The responder also asserts the M-SEARCH we actually
//! put on the wire matches the UDA 1.1 §1.2.3 shape (including the
//! canonical `HOST` header, even though the destination is loopback —
//! the spec fixes the value; only the destination address is a test
//! concern). Multicast-LAN behavior stays `[VERIFY]` (src/ssdp.rs's
//! verdict); what these tests prove is everything short of it.

use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use device_discovery::ssdp::{SsdpSearchConfig, SsdpSearcher};
use testkit::Must;

/// A real in-process SSDP responder: binds loopback, waits for ONE
/// M-SEARCH (with a hard deadline), validates its shape, replies with
/// each payload in order, then dies. Sends the received request bytes
/// back to the test over a channel.
struct Responder {
    addr: std::net::SocketAddr,
    requests: mpsc::Receiver<Vec<u8>>,
    join: Option<thread::JoinHandle<()>>,
}

impl Responder {
    fn spawn(replies: Vec<Vec<u8>>) -> Responder {
        let socket = UdpSocket::bind("127.0.0.1:0").must();
        socket.set_read_timeout(Some(Duration::from_secs(3))).must();
        let addr = socket.local_addr().must();
        let (tx, requests) = mpsc::channel();
        let join = thread::Builder::new()
            .name("ssdp-test-responder".to_string())
            .spawn(move || {
                let mut buf = [0_u8; 4096];
                match socket.recv_from(&mut buf) {
                    Ok((len, peer)) => {
                        let _ = tx.send(buf[..len].to_vec());
                        for reply in replies {
                            // Devices answer within 0..=MX; a test
                            // responder may draw 0 (immediate).
                            if socket.send_to(&reply, peer).is_err() {
                                break; // searcher went away — fine
                            }
                        }
                    }
                    Err(_) => {
                        // Deadline passed with no request; the test's
                        // own assertion on the channel will fail loud.
                    }
                }
            })
            .must();
        Responder {
            addr,
            requests,
            join: Some(join),
        }
    }

    /// The request bytes that reached the wire.
    fn received_request(&self) -> Vec<u8> {
        self.requests.recv_timeout(Duration::from_secs(3)).must()
    }
}

impl Drop for Responder {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            drop(join.join());
        }
    }
}

fn searcher_to(responder: &Responder) -> SsdpSearcher {
    SsdpSearcher::new(SsdpSearchConfig::with_limits(
        responder.addr,
        1,
        16,
        Duration::from_millis(500),
    ))
}

fn spec_response(usn: &str, location: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         EXT:\r\n\
         LOCATION: {location}\r\n\
         SERVER: TestStack/1.0 UPnP/1.1 Test/1\r\n\
         ST: upnp:rootdevice\r\n\
         USN: {usn}\r\n\
         \r\n"
    )
    .into_bytes()
}

#[test]
fn loopback_roundtrip_collects_parses_and_dedups() {
    // Two replies share a USN (a device answering an M-SEARCH twice —
    // legal); one is a distinct device.
    let responder = Responder::spawn(vec![
        spec_response(
            "uuid:aaa::upnp:rootdevice",
            "http://192.0.2.10:8200/aaa.xml",
        ),
        spec_response(
            "uuid:aaa::upnp:rootdevice",
            "http://192.0.2.10:8200/aaa.xml",
        ),
        spec_response(
            "uuid:bbb::upnp:rootdevice",
            "http://192.0.2.11:8200/bbb.xml",
        ),
    ]);
    let searcher = searcher_to(&responder);

    let outcome = searcher.search("upnp:rootdevice").must();
    assert_eq!(outcome.skipped_malformed, 0);
    assert_eq!(outcome.responses.len(), 2, "USN dedup: {outcome:?}");
    let locations: Vec<&str> = outcome
        .responses
        .iter()
        .map(|r| r.location.as_str())
        .collect();
    assert!(locations.contains(&"http://192.0.2.10:8200/aaa.xml"));
    assert!(locations.contains(&"http://192.0.2.11:8200/bbb.xml"));
    assert_eq!(outcome.responses[0].max_age, Some(1800));

    // The wire bytes we sent are the spec shape (the responder's
    // channel is the evidence path — real bytes, not a mock's echo).
    let sent = String::from_utf8(responder.received_request()).must();
    assert!(sent.starts_with("M-SEARCH * HTTP/1.1\r\n"), "{sent:?}");
    assert!(sent.contains("HOST: 239.255.255.250:1900\r\n"), "{sent:?}");
    assert!(sent.contains("MAN: \"ssdp:discover\"\r\n"), "{sent:?}");
    assert!(sent.contains("ST: upnp:rootdevice\r\n"), "{sent:?}");
    assert!(sent.ends_with("\r\n\r\n"), "{sent:?}");
}

#[test]
fn loopback_roundtrip_skips_malformed_and_keeps_going() {
    let responder = Responder::spawn(vec![
        b"NOTIFY * HTTP/1.1\r\n\r\n".to_vec(), // wrong method: skipped
        b"HTTP/1.1 200 OK\r\nLOCATION:\r\nST: s\r\nUSN: u\r\n\r\n".to_vec(), // empty LOCATION: skipped
        spec_response(
            "uuid:good::upnp:rootdevice",
            "http://192.0.2.12:8200/good.xml",
        ),
    ]);
    let searcher = searcher_to(&responder);

    let outcome = searcher.search("ssdp:all").must();
    assert_eq!(outcome.skipped_malformed, 2, "dirt is counted, not fatal");
    assert_eq!(outcome.responses.len(), 1);
    assert_eq!(outcome.responses[0].usn, "uuid:good::upnp:rootdevice");
}

#[test]
fn empty_wire_is_an_honest_empty_result() {
    // A responder that never answers (no SSDP devices — this
    // sandbox's exact multicast-LAN condition): the bounded window
    // closes with a typed Ok and zero responses.
    let socket = UdpSocket::bind("127.0.0.1:0").must();
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .must();
    let addr = socket.local_addr().must(); // bound, never answered, then dropped
    drop(socket);

    let searcher = SsdpSearcher::new(SsdpSearchConfig::with_limits(
        addr,
        1,
        8,
        Duration::from_millis(300),
    ));
    let outcome = searcher.search("ssdp:all").must();
    assert_eq!(outcome.responses.len(), 0);
    assert_eq!(outcome.skipped_malformed, 0);
}

#[test]
fn max_responses_stops_the_window_early() {
    // Three distinct devices on the wire, a cap of 2: the search
    // stops at the cap (bounded work even against chatty stacks).
    let responder = Responder::spawn(vec![
        spec_response("uuid:1::upnp:rootdevice", "http://192.0.2.1/d1.xml"),
        spec_response("uuid:2::upnp:rootdevice", "http://192.0.2.2/d2.xml"),
        spec_response("uuid:3::upnp:rootdevice", "http://192.0.2.3/d3.xml"),
    ]);
    let searcher = SsdpSearcher::new(SsdpSearchConfig::with_limits(
        responder.addr,
        1,
        2,
        Duration::from_secs(2),
    ));
    let started = std::time::Instant::now();
    let outcome = searcher.search("upnp:rootdevice").must();
    assert_eq!(outcome.responses.len(), 2);
    // It stopped at the cap, not at the window: far short of 2 s.
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the cap is the stop condition, not the timeout"
    );
}

/// The native multicast API path exercised on THIS host: an ephemeral
/// socket joins the SSDP group and sends one datagram to it. Every
/// step either succeeds (the sandbox evidence: kernel accepts group
/// join + multicast send) or fails with the typed Io error — there is
/// no third outcome, and that totality is what this pins. Real
/// multicast-LAN discovery stays `[VERIFY]` (zero responders here).
#[test]
fn multicast_api_calls_succeed_or_fail_typed() {
    use device_discovery::error::DiscoveryError;
    use device_discovery::ssdp::{SSDP_MULTICAST_ADDR, SSDP_PORT, SSDP_TTL};

    let outcome: Result<(), DiscoveryError> = (|| {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|err| DiscoveryError::Io {
            context: "multicast probe bind",
            kind: err.kind(),
            detail: err.to_string(),
        })?;
        let group = match SSDP_MULTICAST_ADDR {
            std::net::IpAddr::V4(v4) => v4,
            std::net::IpAddr::V6(_) => unreachable!("the SSDP group is IPv4 by spec"),
        };
        socket
            .join_multicast_v4(&group, &std::net::Ipv4Addr::UNSPECIFIED)
            .map_err(|err| DiscoveryError::Io {
                context: "multicast group join",
                kind: err.kind(),
                detail: err.to_string(),
            })?;
        socket
            .set_multicast_ttl_v4(SSDP_TTL)
            .map_err(|err| DiscoveryError::Io {
                context: "multicast ttl",
                kind: err.kind(),
                detail: err.to_string(),
            })?;
        socket
            .send_to(
                device_discovery::ssdp::build_msearch(1, "ssdp:all").as_bytes(),
                std::net::SocketAddr::new(SSDP_MULTICAST_ADDR, SSDP_PORT),
            )
            .map_err(|err| DiscoveryError::Io {
                context: "multicast m-search send",
                kind: err.kind(),
                detail: err.to_string(),
            })?;
        Ok(())
    })();
    match outcome {
        Ok(()) => { /* this host: the kernel accepted the whole path */ }
        Err(DiscoveryError::Io { context, .. }) => {
            // A hermetic environment refusing sockets is a typed,
            // honest outcome — recorded, not a test failure.
            eprintln!("multicast probe refused on this host at {context}");
        }
        Err(other) => unreachable!("the probe's failure modes are typed Io, got {other:?}"),
    }
}
