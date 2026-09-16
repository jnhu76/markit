//! MD4C FFI — the M0-B full-parse baseline (issue #19 plan §9).
//!
//! Research-only adapter over the vendored MD4C 0.5.3 (see
//! vendor/md4c/PROVENANCE.md). Two measurement surfaces:
//!
//! - `count_events` — parser-native cost: every callback fires exactly
//!   once per event, callbacks only increment a counter.
//! - `normalize` — native cost + semantic normalization: builds the
//!   same normalized block-kind sequence ORACLE-B compares
//!   (BlockQuote→BQ, UL/OL, Heading{level}→H(n), Code→CODE, P→P; LI,
//!   HR, HTML, DOC and spans ignored — they are either interior or
//!   outside the compared vocabulary).
//!
//! Configuration: `flags = 0` (no MD_FLAG extensions → pure
//! CommonMark-style dialect). The normalization delta (normalize −
//! count) is the adapter/normalization cost, reported separately from
//! the parser-native cost, per plan §9.

use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

use crate::cmoracle::Norm;

const MD_BLOCK_QUOTE: c_uint = 1;
const MD_BLOCK_UL: c_uint = 2;
const MD_BLOCK_OL: c_uint = 3;
const MD_BLOCK_H: c_uint = 6;
const MD_BLOCK_CODE: c_uint = 7;
const MD_BLOCK_P: c_uint = 9;

#[repr(C)]
pub struct MdParser {
    abi_version: c_uint,
    flags: c_uint,
    enter_block: extern "C" fn(c_uint, *mut c_void, *mut c_void) -> c_int,
    leave_block: extern "C" fn(c_uint, *mut c_void, *mut c_void) -> c_int,
    enter_span: extern "C" fn(c_uint, *mut c_void, *mut c_void) -> c_int,
    leave_span: extern "C" fn(c_uint, *mut c_void, *mut c_void) -> c_int,
    text: extern "C" fn(c_uint, *const c_char, c_uint, *mut c_void) -> c_int,
    debug_log: Option<extern "C" fn(*const c_char, *mut c_void)>,
    syntax: Option<extern "C" fn()>,
}

unsafe impl Sync for MdParser {}

extern "C" {
    fn md_parse(
        text: *const c_char,
        size: c_uint,
        parser: *const MdParser,
        userdata: *mut c_void,
    ) -> c_int;
}

struct CountCtx {
    events: u64,
}

extern "C" fn count_cb(_t: c_uint, _d: *mut c_void, ud: *mut c_void) -> c_int {
    unsafe {
        let ctx = &mut *(ud as *mut CountCtx);
        ctx.events += 1;
    }
    0
}

extern "C" fn count_text(_t: c_uint, _s: *const c_char, _n: c_uint, ud: *mut c_void) -> c_int {
    unsafe {
        let ctx = &mut *(ud as *mut CountCtx);
        ctx.events += 1;
    }
    0
}

extern "C" fn noop_cb(_t: c_uint, _d: *mut c_void, _ud: *mut c_void) -> c_int {
    0
}

extern "C" fn noop_text(_t: c_uint, _s: *const c_char, _n: c_uint, _ud: *mut c_void) -> c_int {
    0
}

struct NormCtx {
    seq: Vec<Norm>,
}

extern "C" fn norm_enter(t: c_uint, detail: *mut c_void, ud: *mut c_void) -> c_int {
    unsafe {
        let ctx = &mut *(ud as *mut NormCtx);
        match t {
            MD_BLOCK_QUOTE => ctx.seq.push(Norm::BQ),
            MD_BLOCK_UL => ctx.seq.push(Norm::UL),
            MD_BLOCK_OL => ctx.seq.push(Norm::OL),
            MD_BLOCK_H => {
                // MD_BLOCK_H_DETAIL { unsigned level; }
                let level = *(detail as *const c_uint);
                ctx.seq.push(Norm::H(level.clamp(1, 6) as u8));
            }
            MD_BLOCK_CODE => ctx.seq.push(Norm::CODE),
            MD_BLOCK_P => ctx.seq.push(Norm::P),
            _ => {}
        }
    }
    0
}

fn count_parser() -> &'static MdParser {
    static P: OnceLock<MdParser> = OnceLock::new();
    P.get_or_init(|| MdParser {
        abi_version: 0,
        flags: 0,
        enter_block: count_cb,
        leave_block: count_cb,
        enter_span: count_cb,
        leave_span: count_cb,
        text: count_text,
        debug_log: None,
        syntax: None,
    })
}

fn norm_parser() -> &'static MdParser {
    static P: OnceLock<MdParser> = OnceLock::new();
    P.get_or_init(|| MdParser {
        abi_version: 0,
        flags: 0,
        enter_block: norm_enter,
        leave_block: noop_cb,
        enter_span: noop_cb,
        leave_span: noop_cb,
        text: noop_text,
        debug_log: None,
        syntax: None,
    })
}

/// Parser-native full parse: event count only. Returns (events, native
/// result) — a negative value is md4c's own error return.
pub fn count_events(text: &str) -> Result<u64, c_int> {
    let mut ctx = CountCtx { events: 0 };
    let rc = unsafe {
        md_parse(
            text.as_ptr() as *const c_char,
            text.len() as c_uint,
            count_parser(),
            &mut ctx as *mut CountCtx as *mut c_void,
        )
    };
    if rc == 0 {
        Ok(ctx.events)
    } else {
        Err(rc)
    }
}

/// Native parse + normalization into the ORACLE-B block vocabulary.
pub fn normalize(text: &str) -> Result<Vec<Norm>, c_int> {
    let mut ctx = NormCtx {
        seq: Vec::new(),
    };
    let rc = unsafe {
        md_parse(
            text.as_ptr() as *const c_char,
            text.len() as c_uint,
            norm_parser(),
            &mut ctx as *mut NormCtx as *mut c_void,
        )
    };
    if rc == 0 {
        Ok(ctx.seq)
    } else {
        Err(rc)
    }
}
