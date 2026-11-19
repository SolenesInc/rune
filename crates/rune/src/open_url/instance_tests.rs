use std::io::{Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::sync::mpsc::{Sender, channel};

use super::Launch;
use super::linux::{Claim, claim_as, decode, encode};

fn record(tx: &Sender<Launch>) -> impl Fn(Launch) + Send + 'static {
    let tx = tx.clone();
    move |launch| {
        let _ = tx.send(launch);
    }
}

fn unique(name: &str) -> String {
    format!("rune-instance-test-{}-{name}", std::process::id())
}

#[test]
fn a_later_launch_hands_its_links_and_token_to_the_first() {
    let (tx, arrived) = channel();
    let name = unique("handoff");
    let first = Launch {
        token: None,
        links: vec!["myapp://first".into()],
    };
    assert!(matches!(claim_as(&name, &first, record(&tx)), Claim::First));

    let later = Launch {
        token: Some("token-1".into()),
        links: vec![
            "myapp://spawn?cwd=%2Ftmp&label=a".into(),
            "myapp://two".into(),
        ],
    };
    assert!(matches!(
        claim_as(&name, &later, record(&tx)),
        Claim::Handed
    ));
    assert_eq!(arrived.recv().unwrap(), later);

    let plain = Launch::default();
    assert!(matches!(
        claim_as(&name, &plain, record(&tx)),
        Claim::Handed
    ));
    assert_eq!(arrived.recv().unwrap(), plain);
}

#[test]
fn bytes_that_are_not_a_launch_reach_nothing_and_get_no_answer() {
    let (tx, arrived) = channel();
    let name = unique("stray");
    assert!(matches!(
        claim_as(&name, &Launch::default(), record(&tx)),
        Claim::First
    ));

    let mut stray =
        UnixStream::connect_addr(&SocketAddr::from_abstract_name(&name).unwrap()).unwrap();
    stray.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
    stray.shutdown(std::net::Shutdown::Write).unwrap();
    let mut answer = Vec::new();
    stray.read_to_end(&mut answer).unwrap();
    assert!(answer.is_empty());

    let later = Launch {
        token: None,
        links: vec!["myapp://after".into()],
    };
    assert!(matches!(
        claim_as(&name, &later, record(&tx)),
        Claim::Handed
    ));
    assert_eq!(arrived.recv().unwrap(), later);
}

#[test]
fn a_launch_survives_its_encoding_whatever_it_holds() {
    for launch in [
        Launch::default(),
        Launch {
            token: Some("t".into()),
            links: vec![],
        },
        Launch {
            token: None,
            links: vec!["a://".into(), "b://x?y=ü".into()],
        },
        Launch {
            token: Some("tok".into()),
            links: vec!["myapp://one".into()],
        },
    ] {
        assert_eq!(decode(&encode(&launch)), Some(launch));
    }
    assert_eq!(decode(b"rune-launch 1\0tok"), None);
    assert_eq!(decode(b"other\0\0"), None);
}
