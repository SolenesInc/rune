//! One running process per app on Linux: [`single_instance`](crate::single_instance).
//!
//! Linux starts a new process for every launch: a desktop launcher, `xdg-open myapp://…`, a
//! browser handing a link to the app's `.desktop` entry (`Exec=… %u`). The first process of the
//! app claims an abstract Unix socket named for the user and the app
//! (`rune-instance-<uid>-<id>`); the kernel lets go of it when the process ends, so no file is
//! left behind. A later launch finds the name taken, sends its links and its activation token
//! (`XDG_ACTIVATION_TOKEN` on Wayland, `DESKTOP_STARTUP_ID` on X11) and exits. The first process
//! takes them on a thread of its own: the links go to `App::on_open_url` as the links of its own
//! launch do, and the launch reopens the app and focuses its window with the token, so the
//! compositor lets it come forward.
//!
//! macOS keeps one process per app itself and hands links to it, so there this only says yes.

/// Makes this process the app's only running one, before it opens a window (Linux).
///
/// Returns false when another process of the app already runs: this launch's links went to its
/// `App::on_open_url` and its window came forward, so return from `main` without opening one.
/// Returns true when this is the first process, which from now on receives the launches that
/// follow, and also when the first can't be reached, so the launch still opens a window. `id`
/// names the app (its reverse-DNS identifier); builds that should run side by side use
/// different ids.
///
/// macOS and other platforms: always true.
pub fn single_instance(id: &str) -> bool {
    crate::relaunch::wait_for_previous();
    #[cfg(target_os = "linux")]
    {
        linux::claim(id)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = id;
        true
    }
}

/// Whether a launch argument is a link: `scheme://…`, with a scheme of at least two letters,
/// digits, `+`, `-` or `.`, starting with a letter. A path and a flag aren't.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn is_link(arg: &str) -> bool {
    let Some((scheme, _)) = arg.split_once("://") else { return false };
    scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// The links among this process's launch arguments: Linux passes a link to the process it
/// starts for it as an argument (`%u`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn launch_links() -> Vec<String> {
    std::env::args().skip(1).filter(|a| is_link(a)).collect()
}

/// A launch of the app that reached the running process.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Launch {
    /// Lets the running process take focus (`XDG_ACTIVATION_TOKEN`, `DESKTOP_STARTUP_ID`).
    pub token: Option<String>,
    pub links: Vec<String>,
}

#[cfg(target_os = "linux")]
pub(crate) use linux::on_launch;

#[cfg(target_os = "linux")]
mod linux {
    use std::io::{self, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
    use std::sync::Mutex;
    use std::time::Duration;

    use super::Launch;

    /// Opens every message, so a stray client's bytes aren't taken for a launch: the token
    /// (empty for none) and each link follow, each ended by a NUL, until the sender closes.
    const MAGIC: &[u8] = b"rune-launch 1\0";
    /// The running process took the launch.
    const ACCEPTED: u8 = b'A';
    /// The most a launch may send: Linux caps a process's arguments and environment together at
    /// `ARG_MAX`, 2 MiB (`getconf ARG_MAX`, Fedora 44 and Ubuntu 24.04), so twice that is a
    /// tripwire.
    const MAX_MESSAGE: u64 = 4 << 20;
    /// How long either side waits on the other. The running process answers from its own thread,
    /// not the UI's, in 0.27 ms (`a_later_launch_hands_its_links_and_token_to_the_first`,
    /// debug build, Linux x86_64); a side that takes this long is hung.
    const PATIENCE: Duration = Duration::from_secs(2);

    type OnLaunch = Box<dyn Fn(Option<String>) + Send>;

    /// Reopens the app and focuses its window with a launch's token; set once the loop runs.
    static ON_LAUNCH: Mutex<Option<OnLaunch>> = Mutex::new(None);

    /// The loop runs: a later launch reopens the app through `reopen` from now on. One that
    /// came before has no window to bring forward yet; its links still wait in the inbox.
    pub(crate) fn on_launch(reopen: impl Fn(Option<String>) + Send + 'static) {
        *ON_LAUNCH.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(reopen));
    }

    fn arrived(launch: Launch) {
        for url in launch.links {
            crate::open_url::incoming::receive(url);
        }
        if let Some(reopen) = ON_LAUNCH.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            reopen(launch.token);
        }
    }

    pub(super) fn claim(id: &str) -> bool {
        let launch = Launch { token: activation_token(), links: super::launch_links() };
        match claim_as(&name(id), &launch, arrived) {
            Claim::First => true,
            Claim::Handed => false,
            Claim::Unreachable(e) => {
                eprintln!("rune: couldn't reach the running {id}, so this launch runs on its own: {e}");
                true
            }
        }
    }

    fn name(id: &str) -> String {
        // SAFETY: getuid has no preconditions and can't fail.
        let uid = unsafe { libc::getuid() };
        format!("rune-instance-{uid}-{id}")
    }

    fn activation_token() -> Option<String> {
        ["XDG_ACTIVATION_TOKEN", "DESKTOP_STARTUP_ID"].into_iter().find_map(|k| std::env::var(k).ok().filter(|t| !t.is_empty()))
    }

    #[derive(Debug)]
    pub(super) enum Claim {
        /// This process holds the name and serves later launches.
        First,
        /// Another process holds it and took this launch.
        Handed,
        /// Another process holds it but didn't take the launch.
        Unreachable(io::Error),
    }

    /// Claims `name` and serves the launches that follow to `arrived`, or hands `launch` to the
    /// process that holds it.
    pub(super) fn claim_as(name: &str, launch: &Launch, arrived: impl Fn(Launch) + Send + 'static) -> Claim {
        let addr = match SocketAddr::from_abstract_name(name) {
            Ok(addr) => addr,
            Err(e) => return Claim::Unreachable(e),
        };
        match UnixListener::bind_addr(&addr) {
            Ok(listener) => {
                std::thread::Builder::new()
                    .name("rune-instance".into())
                    .spawn(move || serve(listener, arrived))
                    .map_or_else(Claim::Unreachable, |_| Claim::First)
            }
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => match hand_off(&addr, launch) {
                Ok(()) => Claim::Handed,
                Err(e) => Claim::Unreachable(e),
            },
            Err(e) => Claim::Unreachable(e),
        }
    }

    fn hand_off(addr: &SocketAddr, launch: &Launch) -> io::Result<()> {
        let mut stream = UnixStream::connect_addr(addr)?;
        stream.set_read_timeout(Some(PATIENCE))?;
        stream.set_write_timeout(Some(PATIENCE))?;
        if peer_uid(&stream)? != own_uid() {
            return Err(io::Error::other("another user holds the app's name"));
        }
        stream.write_all(&encode(launch))?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut answer = [0];
        stream.read_exact(&mut answer)?;
        if answer[0] == ACCEPTED { Ok(()) } else { Err(io::Error::other("the running process refused the launch")) }
    }

    fn serve(listener: UnixListener, arrived: impl Fn(Launch)) {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            if let Some(launch) = take(&mut stream) {
                let _ = stream.write_all(&[ACCEPTED]);
                drop(stream);
                arrived(launch);
            }
        }
    }

    /// The launch a client sent, if it is the user's own and well formed.
    fn take(stream: &mut UnixStream) -> Option<Launch> {
        stream.set_read_timeout(Some(PATIENCE)).ok()?;
        stream.set_write_timeout(Some(PATIENCE)).ok()?;
        if peer_uid(stream).ok()? != own_uid() {
            return None;
        }
        let mut message = Vec::new();
        stream.take(MAX_MESSAGE + 1).read_to_end(&mut message).ok()?;
        if message.len() as u64 > MAX_MESSAGE {
            return None;
        }
        decode(&message)
    }

    fn own_uid() -> u32 {
        // SAFETY: getuid has no preconditions and can't fail.
        unsafe { libc::getuid() }
    }

    fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
        let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: SO_PEERCRED fills a `ucred` of `len` bytes on a connected Unix socket.
        let got = unsafe {
            libc::getsockopt(stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, (&raw mut cred).cast(), &mut len)
        };
        if got == 0 { Ok(cred.uid) } else { Err(io::Error::last_os_error()) }
    }

    pub(super) fn encode(launch: &Launch) -> Vec<u8> {
        let mut message = MAGIC.to_vec();
        for field in std::iter::once(launch.token.as_deref().unwrap_or("")).chain(launch.links.iter().map(String::as_str)) {
            message.extend(field.bytes().filter(|&b| b != 0));
            message.push(0);
        }
        message
    }

    pub(super) fn decode(message: &[u8]) -> Option<Launch> {
        let body = std::str::from_utf8(message.strip_prefix(MAGIC)?).ok()?;
        let mut fields = body.strip_suffix('\0')?.split('\0');
        let token = fields.next().filter(|t| !t.is_empty()).map(str::to_owned);
        Some(Launch { token, links: fields.map(str::to_owned).collect() })
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "instance_tests.rs"]
mod tests;
