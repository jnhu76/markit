//! Persistent restart-certificate support (spec §6; data-model §8).
//!
//! Logical support of a certificate at an interior Owner boundary `q` is
//! the LF that establishes the blank line's start plus the complete blank
//! physical line including its LF. The I1 `RootBlankEvent` carries
//! document-absolute construction-time offsets; persistence converts
//! them to Owner-relative coordinates before READY (task #18). The
//! support never crosses the left Owner's coverage start.

/// The frozen logical support form (spec §6): the LF terminating the
/// physical line before the blank barrier (`None` only at true BOF) and
/// the complete blank physical line `[start, end)` including its LF.
/// All coordinates are relative to the certificate's Owner base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartSupport {
    pub preceding_lf: Option<usize>,
    pub blank_line: std::ops::Range<usize>,
}

impl RestartSupport {
    /// The support as one contiguous half-open range. The I1 seam only
    /// issues `preceding_lf == blank_line.start - 1` (or `None` at BOF),
    /// so the two pieces are always adjacent; construction and validation
    /// enforce this, which makes the contiguous form exact.
    pub fn support_span(&self) -> std::ops::Range<usize> {
        match self.preceding_lf {
            Some(p) => {
                debug_assert_eq!(
                    p + 1,
                    self.blank_line.start,
                    "support pieces must be adjacent"
                );
                p..self.blank_line.end
            }
            None => {
                debug_assert_eq!(
                    self.blank_line.start, 0,
                    "no preceding LF is legal only at BOF"
                );
                self.blank_line.clone()
            }
        }
    }

    /// The frozen support-touch predicate (spec §6; task #20): an edit
    /// `[start, end)` touches the support iff it intersects the support,
    /// or — for a zero-length insertion — iff `start` lies in the
    /// support. E24 (frozen example, source `"ab\n\n"`, support bytes
    /// `{2,3}`): inserting at byte 2 touches the support, so the
    /// adjacent certificate is not reusable.
    pub fn touches(&self, start: usize, end: usize) -> bool {
        let span = self.support_span();
        if start < end {
            start < span.end && end > span.start
        } else {
            span.contains(&start)
        }
    }
}

/// A persistent outgoing RestartCertificate at one Owner's right
/// coverage cut. It exists only where a real I1 `RootBlankEvent` was
/// observed by the actual parser (never from `finish()`, never from a
/// reconstructed AST state, never at the EOF boundary). A transient
/// event is not persistent state; installation into the retained Owner
/// boundary creates the certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartCertificate {
    pub support: RestartSupport,
}
