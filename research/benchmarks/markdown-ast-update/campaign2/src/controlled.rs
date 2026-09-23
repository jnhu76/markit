//! Surface D — controlled regimes (task §20-§26).
//!
//! The controlled surface consumes [`crate::generators`] cells and turns
//! each one into a [`crate::exec::CaseSpec`], then applies the SAME
//! frozen sampling and ordering policy as the primary surfaces.

use crate::exec::CaseSpec;
use crate::generators::{Axis, ControlledCase};

/// Turn a generated controlled cell into an executable case spec.
pub fn case_spec(case: &ControlledCase) -> CaseSpec {
    CaseSpec::update(
        case.case_id,
        case.case_id_hex.clone(),
        format!("c2:{}:{}", case.axis.as_str().to_lowercase(), case.axis_label),
        case.pre_source.clone(),
        case.post_source.clone(),
        case.edit.clone(),
    )
}

/// One controlled axis' materialized cells.
pub struct AxisPlan {
    pub axis: Axis,
    pub cases: Vec<ControlledCase>,
}

/// Every controlled axis, materialized in frozen order.
pub fn all_axes() -> Result<Vec<AxisPlan>, String> {
    let mut out = Vec::new();
    for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
        let cases = crate::generators::generate_axis(axis)?;
        for case in &cases {
            crate::generators::verify_case(case)?;
        }
        out.push(AxisPlan { axis, cases });
    }
    Ok(out)
}

/// Total controlled cells across all five axes.
pub fn total_cells() -> usize {
    [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K]
        .iter()
        .map(|axis| axis.points().len())
        .sum()
}
