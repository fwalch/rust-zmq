//! Module: zmq

#![cfg_attr(feature = "unstable", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![allow(trivial_numeric_casts)]
#![cfg_attr(feature = "clippy", allow(needless_lifetimes))]

#[macro_use]
extern crate log;

extern crate libc;
extern crate zmq_sys;

use libc::{c_int, c_long, size_t};
use std::ffi;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_void;
use std::result;
use std::string::FromUtf8Error;
use std::{mem, ptr, str, slice};
use std::sync::Arc;

use zmq_sys::{errno, RawSocket};

macro_rules! zmq_try {
    ($($tt:tt)*) => {{
        let rc = $($tt)*;
        if rc == -1 {
            return Err(::errno_to_error());
        }
        rc
    }}
}

mod sockopt;

pub use SocketType::*;

/// `zmq`-specific Result type.
pub type Result<T> = result::Result<T, Error>;

/// Socket types
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq)]
pub enum SocketType {
    PAIR   = 0,
    PUB    = 1,
    SUB    = 2,
    REQ    = 3,
    REP    = 4,
    DEALER = 5,
    ROUTER = 6,
    PULL   = 7,
    PUSH   = 8,
    XPUB   = 9,
    XSUB   = 10,
    STREAM = 11,
}

impl Copy for SocketType {}

/// Flag for socket `send` methods that specifies non-blocking mode.
pub static DONTWAIT: i32 = 1;
/// Flag for socket `send` methods that specifies that more frames of a
/// multipart message will follow.
pub static SNDMORE: i32 = 2;

/// Raw 0MQ socket option constants.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq)]
pub enum Constants {
    ZMQ_AFFINITY                 = 4,
    ZMQ_IDENTITY                 = 5,
    ZMQ_SUBSCRIBE                = 6,
    ZMQ_UNSUBSCRIBE              = 7,
    ZMQ_RATE                     = 8,
    ZMQ_RECOVERY_IVL             = 9,
    ZMQ_SNDBUF                   = 11,
    ZMQ_RCVBUF                   = 12,
    ZMQ_RCVMORE                  = 13,
    ZMQ_FD                       = 14,
    ZMQ_EVENTS                   = 15,
    ZMQ_TYPE                     = 16,
    ZMQ_LINGER                   = 17,
    ZMQ_RECONNECT_IVL            = 18,
    ZMQ_BACKLOG                  = 19,
    ZMQ_RECONNECT_IVL_MAX        = 21,
    ZMQ_MAXMSGSIZE               = 22,
    ZMQ_SNDHWM                   = 23,
    ZMQ_RCVHWM                   = 24,
    ZMQ_MULTICAST_HOPS           = 25,
    ZMQ_RCVTIMEO                 = 27,
    ZMQ_SNDTIMEO                 = 28,
    ZMQ_LAST_ENDPOINT            = 32,
    ZMQ_ROUTER_MANDATORY         = 33,
    ZMQ_TCP_KEEPALIVE            = 34,
    ZMQ_TCP_KEEPALIVE_CNT        = 35,
    ZMQ_TCP_KEEPALIVE_IDLE       = 36,
    ZMQ_TCP_KEEPALIVE_INTVL      = 37,
    ZMQ_IMMEDIATE                = 39,
    ZMQ_XPUB_VERBOSE             = 40,
    ZMQ_ROUTER_RAW               = 41,
    ZMQ_IPV6                     = 42,
    ZMQ_MECHANISM                = 43,
    ZMQ_PLAIN_SERVER             = 44,
    ZMQ_PLAIN_USERNAME           = 45,
    ZMQ_PLAIN_PASSWORD           = 46,
    ZMQ_CURVE_SERVER             = 47,
    ZMQ_CURVE_PUBLICKEY          = 48,
    ZMQ_CURVE_SECRETKEY          = 49,
    ZMQ_CURVE_SERVERKEY          = 50,
    ZMQ_PROBE_ROUTER             = 51,
    ZMQ_REQ_CORRELATE            = 52,
    ZMQ_REQ_RELAXED              = 53,
    ZMQ_CONFLATE                 = 54,
    ZMQ_ZAP_DOMAIN               = 55,
    ZMQ_ROUTER_HANDOVER          = 56,
    ZMQ_TOS                      = 57,
    ZMQ_CONNECT_RID              = 61,
    ZMQ_GSSAPI_SERVER            = 62,
    ZMQ_GSSAPI_PRINCIPAL         = 63,
    ZMQ_GSSAPI_SERVICE_PRINCIPAL = 64,
    ZMQ_GSSAPI_PLAINTEXT         = 65,
    ZMQ_HANDSHAKE_IVL            = 66,
    ZMQ_SOCKS_PROXY              = 68,
    ZMQ_XPUB_NODROP              = 69,

    ZMQ_MSG_MORE                 = 1,
    ZMQ_MSG_SHARED               = 128,
    ZMQ_MSG_MASK                 = 129,
}

impl Copy for Constants {}

impl Constants {
    pub fn to_raw(&self) -> i32 {
        *self as i32
    }

    pub fn from_raw(raw: i32) -> Option<Constants> {
        // fails if `raw` is not a valid value
        match raw {
            4   => Some(Constants::ZMQ_AFFINITY),
            5   => Some(Constants::ZMQ_IDENTITY),
            6   => Some(Constants::ZMQ_SUBSCRIBE),
            7   => Some(Constants::ZMQ_UNSUBSCRIBE),
            8   => Some(Constants::ZMQ_RATE),
            9   => Some(Constants::ZMQ_RECOVERY_IVL),
            11  => Some(Constants::ZMQ_SNDBUF),
            12  => Some(Constants::ZMQ_RCVBUF),
            13  => Some(Constants::ZMQ_RCVMORE),
            14  => Some(Constants::ZMQ_FD),
            15  => Some(Constants::ZMQ_EVENTS),
            16  => Some(Constants::ZMQ_TYPE),
            17  => Some(Constants::ZMQ_LINGER),
            18  => Some(Constants::ZMQ_RECONNECT_IVL),
            19  => Some(Constants::ZMQ_BACKLOG),
            21  => Some(Constants::ZMQ_RECONNECT_IVL_MAX),
            22  => Some(Constants::ZMQ_MAXMSGSIZE),
            23  => Some(Constants::ZMQ_SNDHWM),
            24  => Some(Constants::ZMQ_RCVHWM),
            25  => Some(Constants::ZMQ_MULTICAST_HOPS),
            27  => Some(Constants::ZMQ_RCVTIMEO),
            28  => Some(Constants::ZMQ_SNDTIMEO),
            32  => Some(Constants::ZMQ_LAST_ENDPOINT),
            33  => Some(Constants::ZMQ_ROUTER_MANDATORY),
            34  => Some(Constants::ZMQ_TCP_KEEPALIVE),
            35  => Some(Constants::ZMQ_TCP_KEEPALIVE_CNT),
            36  => Some(Constants::ZMQ_TCP_KEEPALIVE_IDLE),
            37  => Some(Constants::ZMQ_TCP_KEEPALIVE_INTVL),
            39  => Some(Constants::ZMQ_IMMEDIATE),
            40  => Some(Constants::ZMQ_XPUB_VERBOSE),
            41  => Some(Constants::ZMQ_ROUTER_RAW),
            42  => Some(Constants::ZMQ_IPV6),
            43  => Some(Constants::ZMQ_MECHANISM),
            44  => Some(Constants::ZMQ_PLAIN_SERVER),
            45  => Some(Constants::ZMQ_PLAIN_USERNAME),
            46  => Some(Constants::ZMQ_PLAIN_PASSWORD),
            47  => Some(Constants::ZMQ_CURVE_SERVER),
            48  => Some(Constants::ZMQ_CURVE_PUBLICKEY),
            49  => Some(Constants::ZMQ_CURVE_SECRETKEY),
            50  => Some(Constants::ZMQ_CURVE_SERVERKEY),
            51  => Some(Constants::ZMQ_PROBE_ROUTER),
            52  => Some(Constants::ZMQ_REQ_CORRELATE),
            53  => Some(Constants::ZMQ_REQ_RELAXED),
            54  => Some(Constants::ZMQ_CONFLATE),
            55  => Some(Constants::ZMQ_ZAP_DOMAIN),
            56  => Some(Constants::ZMQ_ROUTER_HANDOVER),
            57  => Some(Constants::ZMQ_TOS),
            61  => Some(Constants::ZMQ_CONNECT_RID),
            62  => Some(Constants::ZMQ_GSSAPI_SERVER),
            63  => Some(Constants::ZMQ_GSSAPI_PRINCIPAL),
            64  => Some(Constants::ZMQ_GSSAPI_SERVICE_PRINCIPAL),
            65  => Some(Constants::ZMQ_GSSAPI_PLAINTEXT),
            66  => Some(Constants::ZMQ_HANDSHAKE_IVL),
            68  => Some(Constants::ZMQ_SOCKS_PROXY),
            69  => Some(Constants::ZMQ_XPUB_NODROP),

            1    => Some(Constants::ZMQ_MSG_MORE),
            128  => Some(Constants::ZMQ_MSG_SHARED),
            129  => Some(Constants::ZMQ_MSG_MASK),

            _   => None,
        }
    }
}

/// Security Mechanism
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq)]
pub enum Mechanism {
    ZMQ_NULL   = 0,
    ZMQ_PLAIN  = 1,
    ZMQ_CURVE  = 2,
    ZMQ_GSSAPI = 3,
}

impl Copy for Mechanism {}


/// An error returned by a 0MQ API function.
#[derive(Clone, Eq, PartialEq)]
pub enum Error {
    EACCES          = errno::EACCES as isize,
    EADDRINUSE      = errno::EADDRINUSE as isize,
    EAGAIN          = errno::EAGAIN as isize,
    EBUSY           = errno::EBUSY as isize,
    ECONNREFUSED    = errno::ECONNREFUSED as isize,
    EFAULT          = errno::EFAULT as isize,
    EINTR           = errno::EINTR as isize,
    EHOSTUNREACH    = errno::EHOSTUNREACH as isize,
    EINPROGRESS     = errno::EINPROGRESS as isize,
    EINVAL          = errno::EINVAL as isize,
    EMFILE          = errno::EMFILE as isize,
    EMSGSIZE        = errno::EMSGSIZE as isize,
    ENAMETOOLONG    = errno::ENAMETOOLONG as isize,
    ENODEV          = errno::ENODEV as isize,
    ENOENT          = errno::ENOENT as isize,
    ENOMEM          = errno::ENOMEM as isize,
    ENOTCONN        = errno::ENOTCONN as isize,
    ENOTSOCK        = errno::ENOTSOCK as isize,
    EPROTO          = errno::EPROTO as isize,
    EPROTONOSUPPORT = errno::EPROTONOSUPPORT as isize,
    ENOTSUP         = errno::ENOTSUP as isize,
    ENOBUFS         = errno::ENOBUFS as isize,
    ENETDOWN        = errno::ENETDOWN as isize,
    EADDRNOTAVAIL   = errno::EADDRNOTAVAIL as isize,

    // native zmq error codes
    EFSM            = errno::EFSM as isize,
    ENOCOMPATPROTO  = errno::ENOCOMPATPROTO as isize,
    ETERM           = errno::ETERM as isize,
    EMTHREAD        = errno::EMTHREAD as isize,
}

impl Copy for Error {}

impl Error {
    pub fn to_raw(&self) -> i32 {
        *self as i32
    }

    pub fn from_raw(raw: i32) -> Error {
        #![cfg_attr(feature = "clippy", allow(match_same_arms))]
        match raw {
            errno::EACCES             => Error::EACCES,
            errno::EADDRINUSE         => Error::EADDRINUSE,
            errno::EAGAIN             => Error::EAGAIN,
            errno::EBUSY              => Error::EBUSY,
            errno::ECONNREFUSED       => Error::ECONNREFUSED,
            errno::EFAULT             => Error::EFAULT,
            errno::EHOSTUNREACH       => Error::EHOSTUNREACH,
            errno::EINPROGRESS        => Error::EINPROGRESS,
            errno::EINVAL             => Error::EINVAL,
            errno::EMFILE             => Error::EMFILE,
            errno::EMSGSIZE           => Error::EMSGSIZE,
            errno::ENAMETOOLONG       => Error::ENAMETOOLONG,
            errno::ENODEV             => Error::ENODEV,
            errno::ENOENT             => Error::ENOENT,
            errno::ENOMEM             => Error::ENOMEM,
            errno::ENOTCONN           => Error::ENOTCONN,
            errno::ENOTSOCK           => Error::ENOTSOCK,
            errno::EPROTO             => Error::EPROTO,
            errno::EPROTONOSUPPORT    => Error::EPROTONOSUPPORT,
            errno::ENOTSUP            => Error::ENOTSUP,
            errno::ENOBUFS            => Error::ENOBUFS,
            errno::ENETDOWN           => Error::ENETDOWN,
            errno::EADDRNOTAVAIL      => Error::EADDRNOTAVAIL,
            156384714                => Error::EPROTONOSUPPORT,
            156384715                => Error::ENOBUFS,
            156384716                => Error::ENETDOWN,
            156384717                => Error::EADDRINUSE,
            156384718                => Error::EADDRNOTAVAIL,
            156384719                => Error::ECONNREFUSED,
            156384720                => Error::EINPROGRESS,
            156384721                => Error::ENOTSOCK,
            156384763                => Error::EFSM,
            156384764                => Error::ENOCOMPATPROTO,
            156384765                => Error::ETERM,
            156384766                => Error::EMTHREAD,

            x => {
                unsafe {
                    let s = zmq_sys::zmq_strerror(x);
                    panic!("unknown error [{}]: {}",
                        x,
                        str::from_utf8(ffi::CStr::from_ptr(s).to_bytes()).unwrap()
                    )
                }
            }
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unsafe {
            let s = zmq_sys::zmq_strerror(*self as c_int);
            let v: &'static [u8] =
                mem::transmute(ffi::CStr::from_ptr(s).to_bytes());
            str::from_utf8(v).unwrap()
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            let s = zmq_sys::zmq_strerror(*self as c_int);
            let v: &'static [u8] =
                mem::transmute(ffi::CStr::from_ptr(s).to_bytes());
            write!(f, "{}", str::from_utf8(v).unwrap())
        }
    }
}

impl fmt::Debug for Error {
    /// Return the error string for an error.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            let s = zmq_sys::zmq_strerror(*self as c_int);
            write!(f, "{}",
                   str::from_utf8(ffi::CStr::from_ptr(s).to_bytes()).unwrap())
        }
    }
}

fn errno_to_error() -> Error {
    Error::from_raw(unsafe { zmq_sys::zmq_errno() })
}

/// Return the current zeromq version, as `(major, minor, patch)`.
pub fn version() -> (i32, i32, i32) {
    let mut major = 0;
    let mut minor = 0;
    let mut patch = 0;

    unsafe {
        zmq_sys::zmq_version(&mut major, &mut minor, &mut patch);
    }

    (major as i32, minor as i32, patch as i32)
}

struct RawContext {
    ctx: *mut c_void,
}

impl RawContext {
    fn destroy(&self) -> Result<()> {
        zmq_try!(unsafe { zmq_sys::zmq_ctx_destroy(self.ctx) });
        Ok(())
    }
}

unsafe impl Send for RawContext {}
unsafe impl Sync for RawContext {}

impl Drop for RawContext {
    fn drop(&mut self) {
        debug!("context dropped");
        let mut e = self.destroy();
        while e == Err(Error::EINTR) {
            e = self.destroy();
        }
    }
}

/// Handle for a 0MQ context, used to create sockets.
///
/// It is thread safe, and can be safely cloned and shared. Each clone
/// references the same underlying C context. Internally, an `Arc` is
/// used to implement this in a threadsafe way.
///
/// Also note that this binding deviates from the C API in that each
/// socket created from a context initially owns a clone of that
/// context. This reference is kept to avoid a potential deadlock
/// situation that would otherwise occur:
///
/// Destroying the underlying C context is an operation which
/// blocks waiting for all sockets created from it to be closed
/// first. If one of the sockets belongs to thread issuing the
/// destroy operation, you have established a deadlock.
///
/// You can still deadlock yourself (or intentionally close sockets in
/// other threads, see `zmq_ctx_destroy`(3)) by explicitly calling
/// `Context::destroy`.
///
#[derive(Clone)]
pub struct Context {
    raw: Arc<RawContext>,
}

impl Context {
    /// Create a new reference-counted context handle.
    pub fn new() -> Context {
        Context {
            raw: Arc::new(RawContext {
                ctx: unsafe { zmq_sys::zmq_ctx_new() }
            })
        }
    }

    /// Create a new socket.
    ///
    /// Note that the returned socket keeps a an `Arc` reference to
    /// the context it was created from, and will keep that context
    /// from being dropped while being live.
    pub fn socket(&self, socket_type: SocketType) -> Result<Socket> {
        let sock = unsafe { zmq_sys::zmq_socket(self.raw.ctx, socket_type as c_int) };

        if sock.is_null() {
            return Err(errno_to_error());
        }

        Ok(Socket {
            sock: sock,
            context: Some(self.clone()),
            owned: true,
        })
    }

    /// Try to destroy the context. This is different than the destructor; the
    /// destructor will loop when zmq_ctx_destroy returns EINTR.
    pub fn destroy(&mut self) -> Result<()> {
        self.raw.destroy()
    }
}

impl Default for Context {
    fn default() -> Self {
        Context::new()
    }
}

/// A socket, the central object in 0MQ.
pub struct Socket {
    sock: *mut c_void,
    // The `context` field is never accessed, but implicitly does
    // reference counting via the `Drop` trait.
    #[allow(dead_code)]
    context: Option<Context>,
    owned: bool,
}

unsafe impl Send for Socket {}

impl Drop for Socket {
    fn drop(&mut self) {
        if self.owned {
            if unsafe { zmq_sys::zmq_close(self.sock) } == -1 {
                panic!(errno_to_error());
            } else {
                debug!("socket dropped");
            }
        }
    }
}

macro_rules! sockopt_getter {
    ( $(#[$meta:meta])*
      pub $getter:ident => $constant_name:ident as $ty:ty
    ) => {
        $(#[$meta])*
        pub fn $getter(&self) -> Result<$ty> {
            <$ty as sockopt::Getter>::get(self.sock, Constants::$constant_name.to_raw())
        }
    };
}

macro_rules! sockopt_setter {
    ( $(#[$meta:meta])*
      pub $setter:ident => $constant_name:ident as $ty:ty
    ) => {
        $(#[$meta])*
        pub fn $setter(&self, value: $ty) -> Result<()> {
            <$ty as sockopt::Setter>::set(self.sock, Constants::$constant_name.to_raw(), value)
        }
    };
}

macro_rules! sockopt_seq {
    ( META { $($meta:meta)* }, ) => ();
    ( META { $($meta:meta)* }, if $feature:ident { $($inner:tt)* },
      $($rest:tt)*
    ) => {
        sockopt_seq!(META { cfg($feature = "1") $($meta)* }, $($inner)*);
        sockopt_seq!(META { $($meta)* }, $($rest)*);
    };
    ( META { $($meta:meta)* }, $(#[$item_meta:meta])* (_, $setter:ident) => $constant_name:ident as $ty:ty,
      $($rest:tt)*
    ) => {
        sockopt_setter! {
            $(#[$meta])* $(#[$item_meta])*
            pub $setter => $constant_name as $ty
        }
        sockopt_seq!(META { $($meta)* }, $($rest)*);
    };
    ( META { $($meta:meta)* }, $(#[$item_meta:meta])* ($getter:ident) => $constant_name:ident as $ty:ty,
      $($rest:tt)*
    ) => {
        sockopt_getter! {
            $(#[$meta])* $(#[$item_meta])*
            pub $getter => $constant_name as $ty
        }
        sockopt_seq!(META { $($meta)* }, $($rest)*);
    };
    ( META { $($meta:meta)* }, $(#[$item_meta:meta])* ($getter:ident, $setter:ident) => $constant_name:ident as $ty:ty,
      $($rest:tt)*
    ) => {
        sockopt_getter! {
            $(#[$meta])* $(#[$item_meta])*
            pub $getter => $constant_name as $ty
        }
        sockopt_setter! {
            $(#[$meta])* $(#[$item_meta])*
            pub $setter => $constant_name as $ty
        }
        sockopt_seq!(META { $($meta)* }, $($rest)*);
    };
}

macro_rules! sockopts {
    () => ();
    ( $($rest:tt)* ) => {
        sockopt_seq!(META {}, $($rest)*);
    };
}

impl Socket {
    /// Consume the Socket and return the raw socket pointer.
    ///
    /// Failure to close the raw socket manually or call `from_raw`
    /// will lead to a memory leak. Also note that is function
    /// relinquishes the reference on the context is was created from.
    pub fn into_raw(mut self) -> *mut c_void {
        self.owned = false;
        self.sock
    }

    /// Create a Socket from a raw socket pointer.
    ///
    /// The Socket assumes ownership of the pointer and will close the socket
    /// when it is dropped. The returned socket will not reference any context.
    pub unsafe fn from_raw(sock: *mut c_void) -> Socket {
        Socket {
            sock: sock,
            context: None,
            owned: true,
        }
    }

    /// Return the inner pointer to this Socket.
    ///
    /// **WARNING**:
    /// It is your responsibility to make sure that the underlying
    /// memory is not freed too early.
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.sock
    }

    /// Accept connections on a socket.
    pub fn bind(&self, endpoint: &str) -> Result<()> {
        let c_str = ffi::CString::new(endpoint.as_bytes()).unwrap();
        zmq_try!(unsafe { zmq_sys::zmq_bind(self.sock, c_str.as_ptr()) });
        Ok(())
    }

    /// Connect a socket.
    pub fn connect(&self, endpoint: &str) -> Result<()> {
        let c_str = ffi::CString::new(endpoint.as_bytes()).unwrap();
        zmq_try!(unsafe { zmq_sys::zmq_connect(self.sock, c_str.as_ptr()) });
        Ok(())
    }

    /// Send a `&[u8]` message.
    pub fn send(&self, data: &[u8], flags: i32) -> Result<()> {
        let msg = try!(Message::from_slice(data));
        self.send_msg(msg, flags)
    }

    /// Send a `Message` message.
    pub fn send_msg(&self, mut msg: Message, flags: i32) -> Result<()> {
        zmq_try!(unsafe { zmq_sys::zmq_msg_send(&mut msg.msg, self.sock, flags as c_int) });
        Ok(())
    }

    pub fn send_str(&self, data: &str, flags: i32) -> Result<()> {
        self.send(data.as_bytes(), flags)
    }

    pub fn send_multipart(&self, parts: &[&[u8]], flags: i32) -> Result<()> {
        if parts.is_empty() {
            return Ok(());
        }
        let (last_part, first_parts) = parts.split_last().unwrap();

        for part in first_parts.iter() {
            try!(self.send(part, flags | SNDMORE));
        }
        try!(self.send(last_part, flags));

        Ok(())
    }

    /// Receive a message into a `Message`. The length passed to zmq_msg_recv
    /// is the length of the buffer.
    pub fn recv(&self, msg: &mut Message, flags: i32) -> Result<()> {
        zmq_try!(unsafe { zmq_sys::zmq_msg_recv(&mut msg.msg, self.sock, flags as c_int) });
        Ok(())
    }

    /// Receive bytes into a slice. The length passed to `zmq_recv` is the length of the slice. The
    /// return value is the number of bytes in the message, which may be larger than the length of
    /// the slice, indicating truncation.
    pub fn recv_into(&self, bytes: &mut [u8], flags: i32) -> Result<usize> {
        let bytes_ptr = bytes.as_mut_ptr() as *mut c_void;
        let rc = zmq_try!(unsafe { zmq_sys::zmq_recv(self.sock, bytes_ptr, bytes.len(), flags as c_int) });
        Ok(rc as usize)
    }

    /// Receive a message into a fresh `Message`.
    pub fn recv_msg(&self, flags: i32) -> Result<Message> {
        let mut msg = try!(Message::new());
        self.recv(&mut msg, flags).map(|_| msg)
    }

    /// Receive a message as a byte vector.
    pub fn recv_bytes(&self, flags: i32) -> Result<Vec<u8>> {
        self.recv_msg(flags).map(|msg| msg.to_vec())
    }

    /// Receive a `String` from the socket.
    ///
    /// If the received message is not valid UTF-8, it is returned as the original
    /// Vec in the `Err` part of the inner result.
    pub fn recv_string(&self, flags: i32) -> Result<result::Result<String, Vec<u8>>> {
        self.recv_bytes(flags).map(|bytes| String::from_utf8(bytes).map_err(|e| e.into_bytes()))
    }

    /// Receive a multipart message from the socket.
    ///
    /// Note that this will allocate a new vector for each message part; for
    /// many applications it will be possible to process the different parts
    /// sequentially and reuse allocations that way.
    pub fn recv_multipart(&self, flags: i32) -> Result<Vec<Vec<u8>>> {
        let mut parts: Vec<Vec<u8>> = vec![];
        loop {
            let part = try!(self.recv_bytes(flags));
            parts.push(part);

            let more_parts = try!(self.get_rcvmore());
            if !more_parts {
                break;
            }
        }
        Ok(parts)
    }

    sockopts! {
        /// Accessor for the `ZMQ_IPV6` option.
        (is_ipv6, set_ipv6) => ZMQ_IPV6 as bool,
        /// Accessor for the `ZMQ_IMMEDIATE` option.
        (is_immediate, set_immediate) => ZMQ_IMMEDIATE as bool,
        /// Accessor for the `ZMQ_PLAIN_SERVER` option.
        (is_plain_server, set_plain_server) => ZMQ_PLAIN_SERVER as bool,
        /// Accessor for the `ZMQ_CONFLATE` option.
        (is_conflate, set_conflate) => ZMQ_CONFLATE as bool,
        if ZMQ_HAS_CURVE {
            (is_curve_server, set_curve_server) => ZMQ_CURVE_SERVER as bool,
        },
        if ZMQ_HAS_GSSAPI {
            (is_gssapi_server, set_gssapi_server) => ZMQ_GSSAPI_SERVER as bool,
            (is_gssapi_plaintext, set_gssapi_plaintext) => ZMQ_GSSAPI_PLAINTEXT as bool,
        },
    }

    /// Return the type of this socket.
    pub fn get_socket_type(&self) -> Result<SocketType> {
        sockopt::get(self.sock, Constants::ZMQ_TYPE.to_raw()).map(|ty| {
            match ty {
                0 => SocketType::PAIR,
                1 => SocketType::PUB,
                2 => SocketType::SUB,
                3 => SocketType::REQ,
                4 => SocketType::REP,
                5 => SocketType::DEALER,
                6 => SocketType::ROUTER,
                7 => SocketType::PULL,
                8 => SocketType::PUSH,
                9 => SocketType::XPUB,
                10 => SocketType::XSUB,
                11 => SocketType::STREAM,
                _ => panic!("socket type is out of range!")
            }
        })
    }

    /// Return true if there are more frames of a multipart message to receive.
    pub fn get_rcvmore(&self) -> Result<bool> {
        sockopt::get(self.sock, Constants::ZMQ_RCVMORE.to_raw())
            .map(|o: i64| o == 1i64 )
    }

    sockopts! {
        (get_maxmsgsize, set_maxmsgsize) => ZMQ_MAXMSGSIZE as i64,
        (get_sndhwm, set_sndhwm) => ZMQ_SNDHWM as i32,
        (get_rcvhwm, set_rcvhwm) => ZMQ_RCVHWM as i32,
        (get_affinity, set_affinity) => ZMQ_AFFINITY as u64,
        (get_rate, set_rate) => ZMQ_RATE as i32,
        (get_recovery_ivl, set_recovery_ivl) => ZMQ_RECOVERY_IVL as i32,
        (get_sndbuf, set_sndbuf) => ZMQ_SNDBUF as i32,
        (get_rcvbuf, set_rcvbuf) => ZMQ_RCVBUF as i32,
        (get_tos, set_tos) => ZMQ_TOS as i32,
        (get_linger, set_linger) => ZMQ_LINGER as i32,
        (get_reconnect_ivl, set_reconnect_ivl) => ZMQ_RECONNECT_IVL as i32,
        (get_reconnect_ivl_max, set_reconnect_ivl_max) => ZMQ_RECONNECT_IVL_MAX as i32,
        (get_backlog, set_backlog) => ZMQ_BACKLOG as i32,

        /// Get the underlying file descriptor.
        ///
        /// Getter for the `ZMQ_FD` option. Note that the returned
        /// type is platform-specific; it aliases either
        /// `std::os::unix::io::RawFd` and or
        /// `std::os::windows::io::RawSocket`.
        (get_fd) => ZMQ_FD as RawSocket,

        (get_events) => ZMQ_EVENTS as i32,
        (get_multicast_hops, set_multicast_hops) => ZMQ_MULTICAST_HOPS as i32,
        (get_rcvtimeo, set_rcvtimeo) => ZMQ_RCVTIMEO as i32,
        (get_sndtimeo, set_sndtimeo) => ZMQ_SNDTIMEO as i32,
        (get_tcp_keepalive, set_tcp_keepalive) => ZMQ_TCP_KEEPALIVE as i32,
        (get_tcp_keepalive_cnt, set_tcp_keepalive_cnt) => ZMQ_TCP_KEEPALIVE_CNT as i32,
        (get_tcp_keepalive_idle, set_tcp_keepalive_idle) => ZMQ_TCP_KEEPALIVE_IDLE as i32,
        (get_tcp_keepalive_intvl, set_tcp_keepalive_intvl) => ZMQ_TCP_KEEPALIVE_INTVL as i32,
        (get_handshake_ivl, set_handshake_ivl) => ZMQ_HANDSHAKE_IVL as i32,
        (_, set_identity) => ZMQ_IDENTITY as &[u8],
        (_, set_subscribe) => ZMQ_SUBSCRIBE as &[u8],
        (_, set_unsubscribe) => ZMQ_UNSUBSCRIBE as &[u8],
    }

    pub fn get_identity(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 255 = identity max length
        sockopt::get_string(self.sock, Constants::ZMQ_IDENTITY.to_raw(), 255, false)
    }

    pub fn get_socks_proxy(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 255 = longest allowable domain name is 253 so this should
        // be a reasonable size.
        sockopt::get_string(self.sock, Constants::ZMQ_SOCKS_PROXY.to_raw(), 255, true)
    }

    pub fn get_mechanism(&self) -> Result<Mechanism> {
        sockopt::get(self.sock, Constants::ZMQ_MECHANISM.to_raw()).map(|mech| {
            match mech {
                0 => Mechanism::ZMQ_NULL,
                1 => Mechanism::ZMQ_PLAIN,
                2 => Mechanism::ZMQ_CURVE,
                3 => Mechanism::ZMQ_GSSAPI,
                _ => panic!("Mechanism is out of range!")
            }
        })
    }

    pub fn get_plain_username(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 255 = arbitrary size
        sockopt::get_string(self.sock, Constants::ZMQ_PLAIN_USERNAME.to_raw(), 255, true)
    }

    pub fn get_plain_password(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 256 = arbitrary size based on std crypto key size
        sockopt::get_string(self.sock, Constants::ZMQ_PLAIN_PASSWORD.to_raw(), 256, true)
    }

    pub fn get_zap_domain(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 255 = arbitrary size
        sockopt::get_string(self.sock, Constants::ZMQ_ZAP_DOMAIN.to_raw(), 255, true)
    }

    pub fn get_last_endpoint(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 256 + 9 + 1 = maximum inproc name size (= 256) + "inproc://".len() (= 9), plus null byte
        sockopt::get_string(self.sock, Constants::ZMQ_LAST_ENDPOINT.to_raw(), 256 + 9 + 1, true)
    }

    #[cfg(ZMQ_HAS_CURVE = "1")]
    // FIXME: there should be no decoding errors, hence the return type can be simplified
    pub fn get_curve_publickey(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 41 = Z85 encoded keysize + 1 for null byte
        sockopt::get_string(self.sock, Constants::ZMQ_CURVE_PUBLICKEY.to_raw(), 41, true)
    }

    #[cfg(ZMQ_HAS_CURVE = "1")]
    // FIXME: there should be no decoding errors, hence the return type can be simplified
    pub fn get_curve_secretkey(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 41 = Z85 encoded keysize + 1 for null byte
        sockopt::get_string(self.sock, Constants::ZMQ_CURVE_SECRETKEY.to_raw(), 41, true)
    }

    #[cfg(ZMQ_HAS_CURVE = "1")]
    // FIXME: there should be no decoding errors, hence the return type can be simplified
    pub fn get_curve_serverkey(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 41 = Z85 encoded keysize + 1 for null byte
        sockopt::get_string(self.sock, Constants::ZMQ_CURVE_SERVERKEY.to_raw(), 41, true)
    }

    #[cfg(ZMQ_HAS_GSSAPI = "1")]
    pub fn get_gssapi_principal(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 260 = best guess of max length based on docs.
        sockopt::get_string(self.sock, Constants::ZMQ_GSSAPI_PRINCIPAL.to_raw(), 260, true)
    }

    #[cfg(ZMQ_HAS_GSSAPI = "1")]
    pub fn get_gssapi_service_principal(&self) -> Result<result::Result<String, Vec<u8>>> {
        // 260 = best guess of max length based on docs.
        sockopt::get_string(self.sock, Constants::ZMQ_GSSAPI_SERVICE_PRINCIPAL.to_raw(), 260, true)
    }

    sockopts! {
        (_, set_socks_proxy) => ZMQ_SOCKS_PROXY as Option<&str>,
        (_, set_plain_username) => ZMQ_PLAIN_USERNAME as Option<&str>,
        (_, set_plain_password) => ZMQ_PLAIN_PASSWORD as Option<&str>,
        (_, set_zap_domain) => ZMQ_ZAP_DOMAIN as &str,

        if ZMQ_HAS_CURVE {
            (_, set_curve_publickey) => ZMQ_CURVE_PUBLICKEY as &str,
            (_, set_curve_secretkey) => ZMQ_CURVE_SECRETKEY as &str,
            (_, set_curve_serverkey) => ZMQ_CURVE_SERVERKEY as &str,
        },
        if ZMQ_HAS_GSSAPI {
            (_, set_gssapi_principal) => ZMQ_GSSAPI_PRINCIPAL as &str,
            (_, set_gssapi_service_principal) => ZMQ_GSSAPI_SERVICE_PRINCIPAL as &str,
        },
    }

    /// Create a `PollItem` from the socket.
    pub fn as_poll_item(&self, events: i16) -> PollItem {
        PollItem {
            socket: self.sock,
            fd: 0,
            events: events,
            revents: 0,
            marker: PhantomData
        }
    }

    /// Do a call to `zmq_poll` with only this socket.
    ///
    /// The return value on success will be either zero (no event) or one (some
    /// event was signaled).
    pub fn poll(&self, events: i16, timeout_ms: i64) -> Result<i32> {
        poll(&mut [self.as_poll_item(events)], timeout_ms)
    }
}

/// Holds a 0MQ message.
///
/// A message is a single frame, either received or created locally and then
/// sent over the wire. Multipart messages are transmitted as multiple
/// `Message`s.
///
/// In rust-zmq, you aren't required to create message objects if you use the
/// convenience APIs provided (e.g. `Socket::recv_bytes()` or
/// `Socket::send_str()`). However, using message objects can make multiple
/// operations in a loop more efficient, since allocated memory can be reused.
pub struct Message {
    msg: zmq_sys::zmq_msg_t,
}

impl Drop for Message {
    fn drop(&mut self) {
        unsafe {
            let rc = zmq_sys::zmq_msg_close(&mut self.msg);
            assert_eq!(rc, 0);
        }
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl Message {
    /// Create an empty `Message`.
    pub fn new() -> Result<Message> {
        let mut msg = zmq_sys::zmq_msg_t::default();
        zmq_try!(unsafe { zmq_sys::zmq_msg_init(&mut msg) });
        Ok(Message { msg: msg })
    }

    /// Create a `Message` preallocated with `len` uninitialized bytes.
    pub unsafe fn with_capacity_unallocated(len: usize) -> Result<Message> {
        let mut msg = zmq_sys::zmq_msg_t::default();
        zmq_try!(zmq_sys::zmq_msg_init_size(&mut msg, len as size_t));
        Ok(Message { msg: msg })
    }

    /// Create a `Message` with space for `len` bytes that are initialized to 0.
    pub fn with_capacity(len: usize) -> Result<Message> {
        unsafe {
            let mut msg = try!(Message::with_capacity_unallocated(len));
            ptr::write_bytes(msg.as_mut_ptr(), 0, len);
            Ok(msg)
        }
    }

    /// Create a `Message` from a `&[u8]`. This will copy `data` into the message.
    pub fn from_slice(data: &[u8]) -> Result<Message> {
        unsafe {
            let mut msg = try!(Message::with_capacity_unallocated(data.len()));
            ptr::copy_nonoverlapping(data.as_ptr(), msg.as_mut_ptr(), data.len());
            Ok(msg)
        }
    }

    /// Return the message content as a string slice if it is valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        str::from_utf8(self).ok()
    }

    /// Return the `ZMQ_MORE` flag, which indicates if more parts of a multipart
    /// message will follow.
    pub fn get_more(&self) -> bool {
        let rc = unsafe { zmq_sys::zmq_msg_more(&self.msg as *const _ as *mut _, ) };
        rc != 0
    }

    /// Query a message metadata property.
    pub fn gets<'a>(&'a mut self, property: &str) -> Option<&'a str> {
        let c_str = ffi::CString::new(property.as_bytes()).unwrap();

        let value = unsafe {
            zmq_sys::zmq_msg_gets(&mut self.msg, c_str.as_ptr())
        };

        if value.is_null() {
            None
        } else {
            Some(unsafe { str::from_utf8(ffi::CStr::from_ptr(value).to_bytes()).unwrap() })
        }
    }
}

impl Deref for Message {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // This is safe because we're constraining the slice to the lifetime of
        // this message.
        unsafe {
            let ptr = &self.msg as *const _ as *mut _;
            let data = zmq_sys::zmq_msg_data(ptr);
            let len = zmq_sys::zmq_msg_size(ptr) as usize;
            slice::from_raw_parts(mem::transmute(data), len)
        }
    }
}

impl DerefMut for Message {
    fn deref_mut(&mut self) -> &mut [u8] {
        // This is safe because we're constraining the slice to the lifetime of
        // this message.
        unsafe {
            let data = zmq_sys::zmq_msg_data(&mut self.msg);
            let len = zmq_sys::zmq_msg_size(&mut self.msg) as usize;
            slice::from_raw_parts_mut(mem::transmute(data), len)
        }
    }
}

/// For `poll()`, specifies to signal when a message/some data can be read from
/// a socket.
pub static POLLIN: i16 = 1i16;
/// For `poll()`, specifies to signal when a message/some data can be written to
/// a socket.
pub static POLLOUT: i16 = 2i16;
/// For `poll()`, specifies to signal when an error condition is present on a
/// socket.  This only applies to non-0MQ sockets.
pub static POLLERR: i16 = 4i16;

/// Represents a handle that can be `poll()`ed.
///
/// This is either a reference to a 0MQ socket, or a standard socket.
/// Apart from that it contains the requested event mask, and is updated
/// with the occurred events after `poll()` finishes.
#[repr(C)]
pub struct PollItem<'a> {
    socket: *mut c_void,
    fd: c_int,
    events: i16,
    revents: i16,
    marker: PhantomData<&'a Socket>
}

impl<'a> PollItem<'a> {
    /// Construct a PollItem from a non-0MQ socket, given by its file
    /// descriptor.
    pub fn from_fd(fd: c_int) -> PollItem<'a> {
        PollItem {
            socket: ptr::null_mut(),
            fd: fd,
            events: 0,
            revents: 0,
            marker: PhantomData
        }
    }

    /// Retrieve the events that occurred for this handle.
    pub fn get_revents(&self) -> i16 {
        self.revents
    }
}

/// Poll for events on multiple sockets.
///
/// For every poll item given, the events given in the `events` bitmask are
/// monitored, and signaled in `revents` when they occur. Any number of poll
/// items can have events signaled when the function returns.
///
/// The given timeout is in milliseconds and can be zero. A timeout of `-1`
/// indicates to block indefinitely until an event has occurred.
///
/// The result, if not `Err`, indicates the number of poll items that have
/// events signaled.
pub fn poll(items: &mut [PollItem], timeout: i64) -> Result<i32> {
    let rc = zmq_try!(unsafe {
        zmq_sys::zmq_poll(items.as_mut_ptr() as *mut zmq_sys::zmq_pollitem_t,
                          items.len() as c_int,
                          timeout as c_long)
    });
    Ok(rc as i32)
}

/// Start a 0MQ proxy in the current thread.
///
/// A proxy connects a frontend socket with a backend socket, where the exact
/// behavior depends on the type of both sockets.
///
/// This function only returns (always with an `Err`) when the sockets' context
/// has been closed.
pub fn proxy(frontend: &mut Socket,
             backend: &mut Socket) -> Result<()> {
    zmq_try!(unsafe { zmq_sys::zmq_proxy(frontend.sock, backend.sock, ptr::null_mut()) });
    Ok(())
}

/// Start a 0MQ proxy in the current thread, with a capture socket.
///
/// The capture socket is sent all messages received on the frontend and backend
/// sockets.
pub fn proxy_with_capture(frontend: &mut Socket,
                          backend: &mut Socket,
                          capture: &mut Socket) -> Result<()> {
    zmq_try!(unsafe { zmq_sys::zmq_proxy(frontend.sock, backend.sock, capture.sock) });
    Ok(())
}

/// Return true if the used 0MQ library has the given capability.
///
/// For older versions of 0MQ that don't have the wrapped `zmq_has` function,
/// returns `None`.
///
/// For a list of capabilities, please consult the `zmq_has` manual page.
pub fn has(capability: &str) -> Option<bool> {
    if cfg!(ZMQ_HAS_ZMQ_HAS) {
        let c_str = ffi::CString::new(capability).unwrap();
        unsafe {
            Some(zmq_sys::zmq_has(c_str.as_ptr()) == 1)
        }
    } else {
        None
    }
}

/// A CURVE key pair generated by 0MQ.
#[cfg(ZMQ_HAS_CURVE = "1")]
#[derive(Debug)]
pub struct CurveKeyPair {
    pub public_key: String,
    pub secret_key: String,
}

#[cfg(ZMQ_HAS_CURVE = "1")]
impl CurveKeyPair {
    /// Create a new key pair.
    pub fn new() -> Result<CurveKeyPair> {
        // Curve keypairs are currently 40 bytes long, plus terminating NULL.
        let mut ffi_public_key = vec![0u8; 41];
        let mut ffi_secret_key = vec![0u8; 41];

        zmq_try!(unsafe {
            zmq_sys::zmq_curve_keypair(
                ffi_public_key.as_mut_ptr() as *mut libc::c_char,
                ffi_secret_key.as_mut_ptr() as *mut libc::c_char)
        });

        ffi_public_key.truncate(40);
        ffi_secret_key.truncate(40);

        let public_key = String::from_utf8(ffi_public_key).expect("key not utf8");
        let secret_key = String::from_utf8(ffi_secret_key).expect("key not utf8");

        Ok(CurveKeyPair {
            public_key: public_key,
            secret_key: secret_key,
        })
    }
}

/// Errors that can occur while encoding Z85.
#[derive(Debug)]
pub enum EncodeError {
    BadLength,
    FromUtf8Error(FromUtf8Error),
}

impl From<FromUtf8Error> for EncodeError {
    fn from(err: FromUtf8Error) -> Self {
        EncodeError::FromUtf8Error(err)
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EncodeError::BadLength => write!(f, "Invalid data length. Should be multiple of 4."),
            EncodeError::FromUtf8Error(ref e) => write!(f, "UTF8 conversion error: {}", e),
        }
    }
}

impl std::error::Error for EncodeError {
    fn description(&self) -> &str {
        match *self {
            EncodeError::BadLength => "invalid data length",
            EncodeError::FromUtf8Error(ref e) => e.description(),
        }
    }
}

/// Encode a binary key as Z85 printable text.
///
/// Z85 is an encoding similar to Base64, but operates on 4-byte chunks,
/// which are encoded into 5-byte sequences.
///
/// The input slice *must* have a length divisible by 4.
pub fn z85_encode(data: &[u8]) -> result::Result<String, EncodeError> {
    if data.len() % 4 != 0 {
        return Err(EncodeError::BadLength);
    }

    let len = data.len() * 5 / 4 + 1;
    let mut dest = vec![0u8; len];

    unsafe {
        zmq_sys::zmq_z85_encode(
            dest.as_mut_ptr() as *mut libc::c_char,
            data.as_ptr(),
            data.len());
    }

    dest.truncate(len-1);
    String::from_utf8(dest).map_err(EncodeError::FromUtf8Error)
}

/// Errors that can occur while decoding Z85.
#[derive(Debug)]
pub enum DecodeError {
    /// The input string slice's length was not a multiple of 5.
    BadLength,
    /// The input string slice had embedded NUL bytes.
    NulError(ffi::NulError),
}

impl From<ffi::NulError> for DecodeError {
    fn from(err: ffi::NulError) -> Self {
        DecodeError::NulError(err)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::BadLength => write!(f, "Invalid data length. Should be multiple of 5."),
            DecodeError::NulError(ref e) => write!(f, "Nul byte error: {}", e),
        }
    }
}

impl std::error::Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::BadLength => "invalid data length",
            DecodeError::NulError(ref e) => e.description(),
        }
    }
}

/// Decode a binary key from Z85-encoded text.
///
/// The input string must have a length divisible by 5.
///
/// Note that 0MQ silently accepts characters outside the range defined for
/// the Z85 encoding.
pub fn z85_decode(data: &str) -> result::Result<Vec<u8>, DecodeError> {
    if data.len() % 5 != 0 {
        return Err(DecodeError::BadLength);
    }

    let len = data.len() * 4 / 5;
    let mut dest = vec![0u8; len];

    let c_str = try!(ffi::CString::new(data));

    unsafe {
        zmq_sys::zmq_z85_decode(dest.as_mut_ptr(), c_str.into_raw());
    }

    Ok(dest)
}
