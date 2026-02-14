use super::decimal::DecimalVec;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SlStrategy {
    None,
    Skip(DecimalVec),
    Limit(DecimalVec),
}
