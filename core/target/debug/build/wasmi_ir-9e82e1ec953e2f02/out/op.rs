/// A Wasmi bytecode operator or instruction.
///
/// The [`Op`] type features a small utility API:
///
/// - [`Op::result_ref`]
/// - [`Op::result_mut`]
/// - [`Op::code`]
#[allow(non_camel_case_types)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum Op {
    I32Clz_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32Clz_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I32Ctz_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32Ctz_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I32Popcnt_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32Popcnt_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I32Sext8_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32Sext8_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I32Sext16_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32Sext16_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I32WrapI64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32WrapI64_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I64Clz_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64Clz_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I64Ctz_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64Ctz_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I64Popcnt_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64Popcnt_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I64Sext8_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64Sext8_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I64Sext16_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64Sext16_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    I64Sext32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64Sext32_Rr {
        result: Reg<i64>,
        value: Reg<i64>
    },
    F32Abs_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Abs_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Neg_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Neg_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Nabs_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Nabs_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Ceil_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Ceil_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Floor_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Floor_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Trunc_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Trunc_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Nearest_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Nearest_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32Sqrt_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Sqrt_Rr {
        result: Reg<f32>,
        value: Reg<f32>
    },
    F32ConvertI32_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32ConvertI32_Rr {
        result: Reg<f32>,
        value: Reg<i64>
    },
    F32ConvertU32_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32ConvertU32_Rr {
        result: Reg<f32>,
        value: Reg<i64>
    },
    F32ConvertI64_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32ConvertI64_Rr {
        result: Reg<f32>,
        value: Reg<i64>
    },
    F32ConvertU64_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32ConvertU64_Rr {
        result: Reg<f32>,
        value: Reg<i64>
    },
    F32DemoteF64_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32DemoteF64_Rr {
        result: Reg<f32>,
        value: Reg<f64>
    },
    F64Abs_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Abs_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Neg_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Neg_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Nabs_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Nabs_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Ceil_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Ceil_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Floor_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Floor_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Trunc_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Trunc_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Nearest_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Nearest_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64Sqrt_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Sqrt_Rr {
        result: Reg<f64>,
        value: Reg<f64>
    },
    F64ConvertI32_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64ConvertI32_Rr {
        result: Reg<f64>,
        value: Reg<i64>
    },
    F64ConvertU32_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64ConvertU32_Rr {
        result: Reg<f64>,
        value: Reg<i64>
    },
    F64ConvertI64_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64ConvertI64_Rr {
        result: Reg<f64>,
        value: Reg<i64>
    },
    F64ConvertU64_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64ConvertU64_Rr {
        result: Reg<f64>,
        value: Reg<i64>
    },
    F64PromoteF32_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64PromoteF32_Rr {
        result: Reg<f64>,
        value: Reg<f32>
    },
    I32TruncF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32TruncF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    U32TruncF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U32TruncF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    I32TruncF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32TruncF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    U32TruncF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U32TruncF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    I64TruncF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64TruncF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    U64TruncF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U64TruncF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    I64TruncF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64TruncF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    U64TruncF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U64TruncF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    I32TruncSatF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32TruncSatF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    U32TruncSatF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U32TruncSatF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    I32TruncSatF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I32TruncSatF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    U32TruncSatF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U32TruncSatF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    I64TruncSatF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64TruncSatF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    U64TruncSatF32_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U64TruncSatF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    I64TruncSatF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    I64TruncSatF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    U64TruncSatF64_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U64TruncSatF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    I32Eq_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Eq_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Eq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Eq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32And_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32And_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32And_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32And_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Or_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Or_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Or_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Or_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32NotEq_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32NotEq_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32NotEq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32NotEq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32NotAnd_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32NotAnd_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32NotAnd_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32NotAnd_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32NotOr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32NotOr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32NotOr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32NotOr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Lt_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Lt_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Lt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Lt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Lt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Lt_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Lt_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    I32Le_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Le_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Le_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Le_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Le_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Le_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Le_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    U32Lt_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U32Lt_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: u32
    },
    U32Lt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U32Lt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U32Lt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: u32
    },
    U32Lt_Rir {
        result: Reg<i64>,
        lhs: u32,
        rhs: Reg<i64>
    },
    U32Lt_Ris {
        result: Reg<i64>,
        lhs: u32,
        rhs: Slot
    },
    U32Le_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U32Le_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: u32
    },
    U32Le_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U32Le_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U32Le_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: u32
    },
    U32Le_Rir {
        result: Reg<i64>,
        lhs: u32,
        rhs: Reg<i64>
    },
    U32Le_Ris {
        result: Reg<i64>,
        lhs: u32,
        rhs: Slot
    },
    I64Eq_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Eq_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Eq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Eq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64And_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64And_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64And_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64And_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Or_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Or_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Or_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Or_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64NotEq_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64NotEq_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64NotEq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64NotEq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64NotAnd_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64NotAnd_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64NotAnd_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64NotAnd_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64NotOr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64NotOr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64NotOr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64NotOr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Lt_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Lt_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Lt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Lt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Lt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Lt_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Lt_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    I64Le_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Le_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Le_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Le_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Le_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Le_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Le_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    U64Lt_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U64Lt_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: u64
    },
    U64Lt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U64Lt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U64Lt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: u64
    },
    U64Lt_Rir {
        result: Reg<i64>,
        lhs: u64,
        rhs: Reg<i64>
    },
    U64Lt_Ris {
        result: Reg<i64>,
        lhs: u64,
        rhs: Slot
    },
    U64Le_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U64Le_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: u64
    },
    U64Le_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U64Le_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U64Le_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: u64
    },
    U64Le_Rir {
        result: Reg<i64>,
        lhs: u64,
        rhs: Reg<i64>
    },
    U64Le_Ris {
        result: Reg<i64>,
        lhs: u64,
        rhs: Slot
    },
    F32Eq_Rrs {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Eq_Rri {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Eq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F32Eq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f32
    },
    F32Lt_Rrs {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Lt_Rri {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Lt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Lt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F32Lt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f32
    },
    F32Lt_Rir {
        result: Reg<i64>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Lt_Ris {
        result: Reg<i64>,
        lhs: f32,
        rhs: Slot
    },
    F32Le_Rrs {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Le_Rri {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Le_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Le_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F32Le_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f32
    },
    F32Le_Rir {
        result: Reg<i64>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Le_Ris {
        result: Reg<i64>,
        lhs: f32,
        rhs: Slot
    },
    F32NotEq_Rrs {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32NotEq_Rri {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32NotEq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F32NotEq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f32
    },
    F32NotLt_Rrs {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32NotLt_Rri {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32NotLt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32NotLt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F32NotLt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f32
    },
    F32NotLt_Rir {
        result: Reg<i64>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32NotLt_Ris {
        result: Reg<i64>,
        lhs: f32,
        rhs: Slot
    },
    F32NotLe_Rrs {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32NotLe_Rri {
        result: Reg<i64>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32NotLe_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32NotLe_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F32NotLe_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f32
    },
    F32NotLe_Rir {
        result: Reg<i64>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32NotLe_Ris {
        result: Reg<i64>,
        lhs: f32,
        rhs: Slot
    },
    F64Eq_Rrs {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Eq_Rri {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Eq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Eq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f64
    },
    F64Lt_Rrs {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Lt_Rri {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Lt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Lt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Lt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f64
    },
    F64Lt_Rir {
        result: Reg<i64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Lt_Ris {
        result: Reg<i64>,
        lhs: f64,
        rhs: Slot
    },
    F64Le_Rrs {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Le_Rri {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Le_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Le_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Le_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f64
    },
    F64Le_Rir {
        result: Reg<i64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Le_Ris {
        result: Reg<i64>,
        lhs: f64,
        rhs: Slot
    },
    F64NotEq_Rrs {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64NotEq_Rri {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64NotEq_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F64NotEq_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f64
    },
    F64NotLt_Rrs {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64NotLt_Rri {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64NotLt_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64NotLt_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F64NotLt_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f64
    },
    F64NotLt_Rir {
        result: Reg<i64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64NotLt_Ris {
        result: Reg<i64>,
        lhs: f64,
        rhs: Slot
    },
    F64NotLe_Rrs {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64NotLe_Rri {
        result: Reg<i64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64NotLe_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64NotLe_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    F64NotLe_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: f64
    },
    F64NotLe_Rir {
        result: Reg<i64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64NotLe_Ris {
        result: Reg<i64>,
        lhs: f64,
        rhs: Slot
    },
    I32Add_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Add_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Add_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Add_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Add_Rs_rs {
        result: SlotAndReg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Add_Rs_ri {
        result: SlotAndReg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Add_Rs_ss {
        result: SlotAndReg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Add_Rs_si {
        result: SlotAndReg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Sub_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Sub_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Sub_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Sub_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Sub_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    I32Mul_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Mul_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32Mul_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Mul_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Div_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Div_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<i32>
    },
    I32Div_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Div_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Div_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<i32>
    },
    I32Div_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Div_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    U32Div_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U32Div_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<u32>
    },
    U32Div_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U32Div_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U32Div_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<u32>
    },
    U32Div_Rir {
        result: Reg<i64>,
        lhs: u32,
        rhs: Reg<i64>
    },
    U32Div_Ris {
        result: Reg<i64>,
        lhs: u32,
        rhs: Slot
    },
    I32Rem_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Rem_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<i32>
    },
    I32Rem_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Rem_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Rem_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<i32>
    },
    I32Rem_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Rem_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    U32Rem_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U32Rem_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<u32>
    },
    U32Rem_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U32Rem_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U32Rem_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<u32>
    },
    U32Rem_Rir {
        result: Reg<i64>,
        lhs: u32,
        rhs: Reg<i64>
    },
    U32Rem_Ris {
        result: Reg<i64>,
        lhs: u32,
        rhs: Slot
    },
    I32BitAnd_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32BitAnd_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32BitAnd_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32BitAnd_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32BitOr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32BitOr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32BitOr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32BitOr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32BitXor_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32BitXor_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i32
    },
    I32BitXor_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32BitXor_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i32
    },
    I32Shl_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Shl_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I32Shl_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Shl_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Shl_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I32Shl_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Shl_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    I32Shr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Shr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I32Shr_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Shr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Shr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I32Shr_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Shr_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    U32Shr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U32Shr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    U32Shr_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U32Shr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U32Shr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    U32Shr_Rir {
        result: Reg<i64>,
        lhs: u32,
        rhs: Reg<i64>
    },
    U32Shr_Ris {
        result: Reg<i64>,
        lhs: u32,
        rhs: Slot
    },
    I32Rotl_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Rotl_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I32Rotl_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Rotl_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Rotl_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I32Rotl_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Rotl_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    I32Rotr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I32Rotr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I32Rotr_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I32Rotr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I32Rotr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I32Rotr_Rir {
        result: Reg<i64>,
        lhs: i32,
        rhs: Reg<i64>
    },
    I32Rotr_Ris {
        result: Reg<i64>,
        lhs: i32,
        rhs: Slot
    },
    I64Add_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Add_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Add_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Add_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Add_Rs_rs {
        result: SlotAndReg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Add_Rs_ri {
        result: SlotAndReg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Add_Rs_ss {
        result: SlotAndReg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Add_Rs_si {
        result: SlotAndReg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Sub_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Sub_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Sub_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Sub_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Sub_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    I64Mul_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Mul_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64Mul_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Mul_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Div_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Div_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<i64>
    },
    I64Div_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Div_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Div_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<i64>
    },
    I64Div_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Div_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    U64Div_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U64Div_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<u64>
    },
    U64Div_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U64Div_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U64Div_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<u64>
    },
    U64Div_Rir {
        result: Reg<i64>,
        lhs: u64,
        rhs: Reg<i64>
    },
    U64Div_Ris {
        result: Reg<i64>,
        lhs: u64,
        rhs: Slot
    },
    I64Rem_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Rem_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<i64>
    },
    I64Rem_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Rem_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Rem_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<i64>
    },
    I64Rem_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Rem_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    U64Rem_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U64Rem_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: NonZero<u64>
    },
    U64Rem_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U64Rem_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U64Rem_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: NonZero<u64>
    },
    U64Rem_Rir {
        result: Reg<i64>,
        lhs: u64,
        rhs: Reg<i64>
    },
    U64Rem_Ris {
        result: Reg<i64>,
        lhs: u64,
        rhs: Slot
    },
    I64BitAnd_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64BitAnd_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64BitAnd_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64BitAnd_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64BitOr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64BitOr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64BitOr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64BitOr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64BitXor_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64BitXor_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: i64
    },
    I64BitXor_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64BitXor_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: i64
    },
    I64Shl_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Shl_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I64Shl_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Shl_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Shl_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I64Shl_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Shl_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    I64Shr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Shr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I64Shr_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Shr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Shr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I64Shr_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Shr_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    U64Shr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    U64Shr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    U64Shr_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    U64Shr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    U64Shr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    U64Shr_Rir {
        result: Reg<i64>,
        lhs: u64,
        rhs: Reg<i64>
    },
    U64Shr_Ris {
        result: Reg<i64>,
        lhs: u64,
        rhs: Slot
    },
    I64Rotl_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Rotl_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I64Rotl_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Rotl_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Rotl_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I64Rotl_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Rotl_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    I64Rotr_Rrs {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Slot
    },
    I64Rotr_Rri {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: ShiftAmount
    },
    I64Rotr_Rsr {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Reg<i64>
    },
    I64Rotr_Rss {
        result: Reg<i64>,
        lhs: Slot,
        rhs: Slot
    },
    I64Rotr_Rsi {
        result: Reg<i64>,
        lhs: Slot,
        rhs: ShiftAmount
    },
    I64Rotr_Rir {
        result: Reg<i64>,
        lhs: i64,
        rhs: Reg<i64>
    },
    I64Rotr_Ris {
        result: Reg<i64>,
        lhs: i64,
        rhs: Slot
    },
    F32Add_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Add_Rri {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Add_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Add_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Add_Rsi {
        result: Reg<f32>,
        lhs: Slot,
        rhs: f32
    },
    F32Add_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Add_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F32Sub_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Sub_Rri {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Sub_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Sub_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Sub_Rsi {
        result: Reg<f32>,
        lhs: Slot,
        rhs: f32
    },
    F32Sub_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Sub_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F32Mul_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Mul_Rri {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Mul_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Mul_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Mul_Rsi {
        result: Reg<f32>,
        lhs: Slot,
        rhs: f32
    },
    F32Mul_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Mul_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F32Div_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Div_Rri {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Div_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Div_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Div_Rsi {
        result: Reg<f32>,
        lhs: Slot,
        rhs: f32
    },
    F32Div_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Div_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F32Min_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Min_Rri {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Min_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Min_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Min_Rsi {
        result: Reg<f32>,
        lhs: Slot,
        rhs: f32
    },
    F32Min_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Min_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F32Max_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Max_Rri {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: f32
    },
    F32Max_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Max_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Max_Rsi {
        result: Reg<f32>,
        lhs: Slot,
        rhs: f32
    },
    F32Max_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Max_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F64Add_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Add_Rri {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Add_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Add_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Add_Rsi {
        result: Reg<f64>,
        lhs: Slot,
        rhs: f64
    },
    F64Add_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Add_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    F64Sub_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Sub_Rri {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Sub_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Sub_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Sub_Rsi {
        result: Reg<f64>,
        lhs: Slot,
        rhs: f64
    },
    F64Sub_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Sub_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    F64Mul_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Mul_Rri {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Mul_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Mul_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Mul_Rsi {
        result: Reg<f64>,
        lhs: Slot,
        rhs: f64
    },
    F64Mul_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Mul_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    F64Div_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Div_Rri {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Div_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Div_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Div_Rsi {
        result: Reg<f64>,
        lhs: Slot,
        rhs: f64
    },
    F64Div_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Div_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    F64Min_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Min_Rri {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Min_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Min_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Min_Rsi {
        result: Reg<f64>,
        lhs: Slot,
        rhs: f64
    },
    F64Min_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Min_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    F64Max_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Max_Rri {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: f64
    },
    F64Max_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Max_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Max_Rsi {
        result: Reg<f64>,
        lhs: Slot,
        rhs: f64
    },
    F64Max_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Max_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    F32Copysign_Rrs {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Slot
    },
    F32Copysign_Rsr {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Reg<f32>
    },
    F32Copysign_Rss {
        result: Reg<f32>,
        lhs: Slot,
        rhs: Slot
    },
    F32Copysign_Rir {
        result: Reg<f32>,
        lhs: f32,
        rhs: Reg<f32>
    },
    F32Copysign_Ris {
        result: Reg<f32>,
        lhs: f32,
        rhs: Slot
    },
    F64Copysign_Rrs {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Slot
    },
    F64Copysign_Rsr {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Reg<f64>
    },
    F64Copysign_Rss {
        result: Reg<f64>,
        lhs: Slot,
        rhs: Slot
    },
    F64Copysign_Rir {
        result: Reg<f64>,
        lhs: f64,
        rhs: Reg<f64>
    },
    F64Copysign_Ris {
        result: Reg<f64>,
        lhs: f64,
        rhs: Slot
    },
    I32Mul_Rrr {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Reg<i64>
    },
    I64Mul_Rrr {
        result: Reg<i64>,
        lhs: Reg<i64>,
        rhs: Reg<i64>
    },
    F32Mul_Rrr {
        result: Reg<f32>,
        lhs: Reg<f32>,
        rhs: Reg<f32>
    },
    F64Mul_Rrr {
        result: Reg<f64>,
        lhs: Reg<f64>,
        rhs: Reg<f64>
    },
    BranchI32Eq_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32Eq_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32Eq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32Eq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32NotEq_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32NotEq_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32NotEq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32NotEq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32And_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32And_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32And_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32And_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32NotAnd_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32NotAnd_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32NotAnd_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32NotAnd_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32Or_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32Or_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32Or_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32Or_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32NotOr_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32NotOr_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32NotOr_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32NotOr_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32Lt_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32Lt_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32Lt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchI32Lt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32Lt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32Lt_Ir {
        offset: BranchOffset,
        lhs: i32,
        rhs: Reg<i64>
    },
    BranchI32Lt_Is {
        offset: BranchOffset,
        lhs: i32,
        rhs: Slot
    },
    BranchI32Le_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI32Le_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i32
    },
    BranchI32Le_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchI32Le_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI32Le_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i32
    },
    BranchI32Le_Ir {
        offset: BranchOffset,
        lhs: i32,
        rhs: Reg<i64>
    },
    BranchI32Le_Is {
        offset: BranchOffset,
        lhs: i32,
        rhs: Slot
    },
    BranchU32Lt_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchU32Lt_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: u32
    },
    BranchU32Lt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchU32Lt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchU32Lt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: u32
    },
    BranchU32Lt_Ir {
        offset: BranchOffset,
        lhs: u32,
        rhs: Reg<i64>
    },
    BranchU32Lt_Is {
        offset: BranchOffset,
        lhs: u32,
        rhs: Slot
    },
    BranchU32Le_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchU32Le_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: u32
    },
    BranchU32Le_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchU32Le_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchU32Le_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: u32
    },
    BranchU32Le_Ir {
        offset: BranchOffset,
        lhs: u32,
        rhs: Reg<i64>
    },
    BranchU32Le_Is {
        offset: BranchOffset,
        lhs: u32,
        rhs: Slot
    },
    BranchI64Eq_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64Eq_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64Eq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64Eq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64NotEq_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64NotEq_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64NotEq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64NotEq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64And_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64And_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64And_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64And_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64NotAnd_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64NotAnd_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64NotAnd_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64NotAnd_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64Or_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64Or_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64Or_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64Or_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64NotOr_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64NotOr_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64NotOr_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64NotOr_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64Lt_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64Lt_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64Lt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchI64Lt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64Lt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64Lt_Ir {
        offset: BranchOffset,
        lhs: i64,
        rhs: Reg<i64>
    },
    BranchI64Lt_Is {
        offset: BranchOffset,
        lhs: i64,
        rhs: Slot
    },
    BranchI64Le_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchI64Le_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: i64
    },
    BranchI64Le_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchI64Le_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchI64Le_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: i64
    },
    BranchI64Le_Ir {
        offset: BranchOffset,
        lhs: i64,
        rhs: Reg<i64>
    },
    BranchI64Le_Is {
        offset: BranchOffset,
        lhs: i64,
        rhs: Slot
    },
    BranchU64Lt_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchU64Lt_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: u64
    },
    BranchU64Lt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchU64Lt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchU64Lt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: u64
    },
    BranchU64Lt_Ir {
        offset: BranchOffset,
        lhs: u64,
        rhs: Reg<i64>
    },
    BranchU64Lt_Is {
        offset: BranchOffset,
        lhs: u64,
        rhs: Slot
    },
    BranchU64Le_Rs {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: Slot
    },
    BranchU64Le_Ri {
        offset: BranchOffset,
        lhs: Reg<i64>,
        rhs: u64
    },
    BranchU64Le_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<i64>
    },
    BranchU64Le_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchU64Le_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: u64
    },
    BranchU64Le_Ir {
        offset: BranchOffset,
        lhs: u64,
        rhs: Reg<i64>
    },
    BranchU64Le_Is {
        offset: BranchOffset,
        lhs: u64,
        rhs: Slot
    },
    BranchF32Eq_Rs {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: Slot
    },
    BranchF32Eq_Ri {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: f32
    },
    BranchF32Eq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF32Eq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f32
    },
    BranchF32NotEq_Rs {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: Slot
    },
    BranchF32NotEq_Ri {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: f32
    },
    BranchF32NotEq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF32NotEq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f32
    },
    BranchF32Lt_Rs {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: Slot
    },
    BranchF32Lt_Ri {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: f32
    },
    BranchF32Lt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f32>
    },
    BranchF32Lt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF32Lt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f32
    },
    BranchF32Lt_Ir {
        offset: BranchOffset,
        lhs: f32,
        rhs: Reg<f32>
    },
    BranchF32Lt_Is {
        offset: BranchOffset,
        lhs: f32,
        rhs: Slot
    },
    BranchF32NotLt_Rs {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: Slot
    },
    BranchF32NotLt_Ri {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: f32
    },
    BranchF32NotLt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f32>
    },
    BranchF32NotLt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF32NotLt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f32
    },
    BranchF32NotLt_Ir {
        offset: BranchOffset,
        lhs: f32,
        rhs: Reg<f32>
    },
    BranchF32NotLt_Is {
        offset: BranchOffset,
        lhs: f32,
        rhs: Slot
    },
    BranchF32Le_Rs {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: Slot
    },
    BranchF32Le_Ri {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: f32
    },
    BranchF32Le_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f32>
    },
    BranchF32Le_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF32Le_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f32
    },
    BranchF32Le_Ir {
        offset: BranchOffset,
        lhs: f32,
        rhs: Reg<f32>
    },
    BranchF32Le_Is {
        offset: BranchOffset,
        lhs: f32,
        rhs: Slot
    },
    BranchF32NotLe_Rs {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: Slot
    },
    BranchF32NotLe_Ri {
        offset: BranchOffset,
        lhs: Reg<f32>,
        rhs: f32
    },
    BranchF32NotLe_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f32>
    },
    BranchF32NotLe_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF32NotLe_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f32
    },
    BranchF32NotLe_Ir {
        offset: BranchOffset,
        lhs: f32,
        rhs: Reg<f32>
    },
    BranchF32NotLe_Is {
        offset: BranchOffset,
        lhs: f32,
        rhs: Slot
    },
    BranchF64Eq_Rs {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: Slot
    },
    BranchF64Eq_Ri {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: f64
    },
    BranchF64Eq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF64Eq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f64
    },
    BranchF64NotEq_Rs {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: Slot
    },
    BranchF64NotEq_Ri {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: f64
    },
    BranchF64NotEq_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF64NotEq_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f64
    },
    BranchF64Lt_Rs {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: Slot
    },
    BranchF64Lt_Ri {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: f64
    },
    BranchF64Lt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f64>
    },
    BranchF64Lt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF64Lt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f64
    },
    BranchF64Lt_Ir {
        offset: BranchOffset,
        lhs: f64,
        rhs: Reg<f64>
    },
    BranchF64Lt_Is {
        offset: BranchOffset,
        lhs: f64,
        rhs: Slot
    },
    BranchF64NotLt_Rs {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: Slot
    },
    BranchF64NotLt_Ri {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: f64
    },
    BranchF64NotLt_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f64>
    },
    BranchF64NotLt_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF64NotLt_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f64
    },
    BranchF64NotLt_Ir {
        offset: BranchOffset,
        lhs: f64,
        rhs: Reg<f64>
    },
    BranchF64NotLt_Is {
        offset: BranchOffset,
        lhs: f64,
        rhs: Slot
    },
    BranchF64Le_Rs {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: Slot
    },
    BranchF64Le_Ri {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: f64
    },
    BranchF64Le_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f64>
    },
    BranchF64Le_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF64Le_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f64
    },
    BranchF64Le_Ir {
        offset: BranchOffset,
        lhs: f64,
        rhs: Reg<f64>
    },
    BranchF64Le_Is {
        offset: BranchOffset,
        lhs: f64,
        rhs: Slot
    },
    BranchF64NotLe_Rs {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: Slot
    },
    BranchF64NotLe_Ri {
        offset: BranchOffset,
        lhs: Reg<f64>,
        rhs: f64
    },
    BranchF64NotLe_Sr {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Reg<f64>
    },
    BranchF64NotLe_Ss {
        offset: BranchOffset,
        lhs: Slot,
        rhs: Slot
    },
    BranchF64NotLe_Si {
        offset: BranchOffset,
        lhs: Slot,
        rhs: f64
    },
    BranchF64NotLe_Ir {
        offset: BranchOffset,
        lhs: f64,
        rhs: Reg<f64>
    },
    BranchF64NotLe_Is {
        offset: BranchOffset,
        lhs: f64,
        rhs: Slot
    },
    U32Select_Rrri {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Reg<i64>,
        false_val: u32
    },
    U32Select_Rrsi {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: u32
    },
    U32Select_Rrir {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: u32,
        false_val: Reg<i64>
    },
    U32Select_Rris {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: u32,
        false_val: Slot
    },
    U32Select_Rrii {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: u32,
        false_val: u32
    },
    U32Select_Rsri {
        result: Reg<i64>,
        condition: Slot,
        true_val: Reg<i64>,
        false_val: u32
    },
    U32Select_Rssi {
        result: Reg<i64>,
        condition: Slot,
        true_val: Slot,
        false_val: u32
    },
    U32Select_Rsir {
        result: Reg<i64>,
        condition: Slot,
        true_val: u32,
        false_val: Reg<i64>
    },
    U32Select_Rsis {
        result: Reg<i64>,
        condition: Slot,
        true_val: u32,
        false_val: Slot
    },
    U32Select_Rsii {
        result: Reg<i64>,
        condition: Slot,
        true_val: u32,
        false_val: u32
    },
    U64Select_Rrrs {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Reg<i64>,
        false_val: Slot
    },
    U64Select_Rrri {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Reg<i64>,
        false_val: u64
    },
    U64Select_Rrsr {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: Reg<i64>
    },
    U64Select_Rrss {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: Slot
    },
    U64Select_Rrsi {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: u64
    },
    U64Select_Rrir {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: u64,
        false_val: Reg<i64>
    },
    U64Select_Rris {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: u64,
        false_val: Slot
    },
    U64Select_Rrii {
        result: Reg<i64>,
        condition: Reg<i64>,
        true_val: u64,
        false_val: u64
    },
    U64Select_Rsrs {
        result: Reg<i64>,
        condition: Slot,
        true_val: Reg<i64>,
        false_val: Slot
    },
    U64Select_Rsri {
        result: Reg<i64>,
        condition: Slot,
        true_val: Reg<i64>,
        false_val: u64
    },
    U64Select_Rssr {
        result: Reg<i64>,
        condition: Slot,
        true_val: Slot,
        false_val: Reg<i64>
    },
    U64Select_Rsss {
        result: Reg<i64>,
        condition: Slot,
        true_val: Slot,
        false_val: Slot
    },
    U64Select_Rssi {
        result: Reg<i64>,
        condition: Slot,
        true_val: Slot,
        false_val: u64
    },
    U64Select_Rsir {
        result: Reg<i64>,
        condition: Slot,
        true_val: u64,
        false_val: Reg<i64>
    },
    U64Select_Rsis {
        result: Reg<i64>,
        condition: Slot,
        true_val: u64,
        false_val: Slot
    },
    U64Select_Rsii {
        result: Reg<i64>,
        condition: Slot,
        true_val: u64,
        false_val: u64
    },
    F32Select_Rrrs {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: Reg<f32>,
        false_val: Slot
    },
    F32Select_Rrri {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: Reg<f32>,
        false_val: f32
    },
    F32Select_Rrsr {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: Reg<f32>
    },
    F32Select_Rrss {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: Slot
    },
    F32Select_Rrsi {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: f32
    },
    F32Select_Rrir {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: f32,
        false_val: Reg<f32>
    },
    F32Select_Rris {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: f32,
        false_val: Slot
    },
    F32Select_Rrii {
        result: Reg<f32>,
        condition: Reg<i64>,
        true_val: f32,
        false_val: f32
    },
    F32Select_Rsrs {
        result: Reg<f32>,
        condition: Slot,
        true_val: Reg<f32>,
        false_val: Slot
    },
    F32Select_Rsri {
        result: Reg<f32>,
        condition: Slot,
        true_val: Reg<f32>,
        false_val: f32
    },
    F32Select_Rssr {
        result: Reg<f32>,
        condition: Slot,
        true_val: Slot,
        false_val: Reg<f32>
    },
    F32Select_Rsss {
        result: Reg<f32>,
        condition: Slot,
        true_val: Slot,
        false_val: Slot
    },
    F32Select_Rssi {
        result: Reg<f32>,
        condition: Slot,
        true_val: Slot,
        false_val: f32
    },
    F32Select_Rsir {
        result: Reg<f32>,
        condition: Slot,
        true_val: f32,
        false_val: Reg<f32>
    },
    F32Select_Rsis {
        result: Reg<f32>,
        condition: Slot,
        true_val: f32,
        false_val: Slot
    },
    F32Select_Rsii {
        result: Reg<f32>,
        condition: Slot,
        true_val: f32,
        false_val: f32
    },
    F64Select_Rrrs {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: Reg<f64>,
        false_val: Slot
    },
    F64Select_Rrri {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: Reg<f64>,
        false_val: f64
    },
    F64Select_Rrsr {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: Reg<f64>
    },
    F64Select_Rrss {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: Slot
    },
    F64Select_Rrsi {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: Slot,
        false_val: f64
    },
    F64Select_Rrir {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: f64,
        false_val: Reg<f64>
    },
    F64Select_Rris {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: f64,
        false_val: Slot
    },
    F64Select_Rrii {
        result: Reg<f64>,
        condition: Reg<i64>,
        true_val: f64,
        false_val: f64
    },
    F64Select_Rsrs {
        result: Reg<f64>,
        condition: Slot,
        true_val: Reg<f64>,
        false_val: Slot
    },
    F64Select_Rsri {
        result: Reg<f64>,
        condition: Slot,
        true_val: Reg<f64>,
        false_val: f64
    },
    F64Select_Rssr {
        result: Reg<f64>,
        condition: Slot,
        true_val: Slot,
        false_val: Reg<f64>
    },
    F64Select_Rsss {
        result: Reg<f64>,
        condition: Slot,
        true_val: Slot,
        false_val: Slot
    },
    F64Select_Rssi {
        result: Reg<f64>,
        condition: Slot,
        true_val: Slot,
        false_val: f64
    },
    F64Select_Rsir {
        result: Reg<f64>,
        condition: Slot,
        true_val: f64,
        false_val: Reg<f64>
    },
    F64Select_Rsis {
        result: Reg<f64>,
        condition: Slot,
        true_val: f64,
        false_val: Slot
    },
    F64Select_Rsii {
        result: Reg<f64>,
        condition: Slot,
        true_val: f64,
        false_val: f64
    },
    U32Load_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U32LoadMem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U32LoadMem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U32Load_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U32LoadMem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U32LoadMem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U32Load_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U64Load_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U64LoadMem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadMem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64Load_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U64LoadMem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadMem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64Load_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    F32Load_Rr {
        result: Reg<f32>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    F32LoadMem0Offset16_Rr {
        result: Reg<f32>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    F32LoadMem0Offset16_Rs_r {
        result: SlotAndReg<f32>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    F32Load_Rs {
        result: Reg<f32>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    F32LoadMem0Offset16_Rs {
        result: Reg<f32>,
        ptr: Slot,
        offset: Offset16
    },
    F32LoadMem0Offset16_Rs_s {
        result: SlotAndReg<f32>,
        ptr: Slot,
        offset: Offset16
    },
    F32Load_Ri {
        result: Reg<f32>,
        ptr: Address,
        memory: Memory
    },
    F64Load_Rr {
        result: Reg<f64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    F64LoadMem0Offset16_Rr {
        result: Reg<f64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    F64LoadMem0Offset16_Rs_r {
        result: SlotAndReg<f64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    F64Load_Rs {
        result: Reg<f64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    F64LoadMem0Offset16_Rs {
        result: Reg<f64>,
        ptr: Slot,
        offset: Offset16
    },
    F64LoadMem0Offset16_Rs_s {
        result: SlotAndReg<f64>,
        ptr: Slot,
        offset: Offset16
    },
    F64Load_Ri {
        result: Reg<f64>,
        ptr: Address,
        memory: Memory
    },
    I32LoadExtend8_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    I32LoadExtend8Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I32LoadExtend8Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I32LoadExtend8_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    I32LoadExtend8Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I32LoadExtend8Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I32LoadExtend8_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    I32LoadExtend16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    I32LoadExtend16Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I32LoadExtend16Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I32LoadExtend16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    I32LoadExtend16Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I32LoadExtend16Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I32LoadExtend16_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U32LoadExtend8_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U32LoadExtend8Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U32LoadExtend8Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U32LoadExtend8_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U32LoadExtend8Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U32LoadExtend8Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U32LoadExtend8_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U32LoadExtend16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U32LoadExtend16Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U32LoadExtend16Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U32LoadExtend16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U32LoadExtend16Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U32LoadExtend16Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U32LoadExtend16_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    I64LoadExtend8_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    I64LoadExtend8Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I64LoadExtend8Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I64LoadExtend8_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    I64LoadExtend8Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I64LoadExtend8Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I64LoadExtend8_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    I64LoadExtend16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    I64LoadExtend16Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I64LoadExtend16Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I64LoadExtend16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    I64LoadExtend16Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I64LoadExtend16Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I64LoadExtend16_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    I64LoadExtend32_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    I64LoadExtend32Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I64LoadExtend32Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    I64LoadExtend32_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    I64LoadExtend32Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I64LoadExtend32Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    I64LoadExtend32_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U64LoadExtend8_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U64LoadExtend8Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadExtend8Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadExtend8_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U64LoadExtend8Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadExtend8Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadExtend8_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U64LoadExtend16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U64LoadExtend16Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadExtend16Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadExtend16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U64LoadExtend16Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadExtend16Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadExtend16_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U64LoadExtend32_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset,
        memory: Memory
    },
    U64LoadExtend32Mem0Offset16_Rr {
        result: Reg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadExtend32Mem0Offset16_Rs_r {
        result: SlotAndReg<i64>,
        ptr: Reg<i64>,
        offset: Offset16
    },
    U64LoadExtend32_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset,
        memory: Memory
    },
    U64LoadExtend32Mem0Offset16_Rs {
        result: Reg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadExtend32Mem0Offset16_Rs_s {
        result: SlotAndReg<i64>,
        ptr: Slot,
        offset: Offset16
    },
    U64LoadExtend32_Ri {
        result: Reg<i64>,
        ptr: Address,
        memory: Memory
    },
    U32Store_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    U32StoreMem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    U32Store_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: u32,
        memory: Memory
    },
    U32StoreMem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: u32
    },
    U32Store_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    U32StoreMem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    U32Store_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    U32StoreMem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    U32Store_Si {
        ptr: Slot,
        offset: Offset,
        value: u32,
        memory: Memory
    },
    U32StoreMem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: u32
    },
    U32Store_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    U32Store_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    U32Store_Ii {
        ptr: Address,
        value: u32,
        memory: Memory
    },
    U64Store_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    U64StoreMem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    U64Store_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: u64,
        memory: Memory
    },
    U64StoreMem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: u64
    },
    U64Store_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    U64StoreMem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    U64Store_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    U64StoreMem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    U64Store_Si {
        ptr: Slot,
        offset: Offset,
        value: u64,
        memory: Memory
    },
    U64StoreMem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: u64
    },
    U64Store_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    U64Store_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    U64Store_Ii {
        ptr: Address,
        value: u64,
        memory: Memory
    },
    F32Store_Rr {
        ptr: Reg<i64>,
        offset: Offset,
        value: Reg<f32>,
        memory: Memory
    },
    F32StoreMem0Offset16_Rr {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Reg<f32>
    },
    F32Store_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<f32>,
        memory: Memory
    },
    F32StoreMem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<f32>
    },
    F32Store_Ir {
        ptr: Address,
        value: Reg<f32>,
        memory: Memory
    },
    F64Store_Rr {
        ptr: Reg<i64>,
        offset: Offset,
        value: Reg<f64>,
        memory: Memory
    },
    F64StoreMem0Offset16_Rr {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Reg<f64>
    },
    F64Store_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<f64>,
        memory: Memory
    },
    F64StoreMem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<f64>
    },
    F64Store_Ir {
        ptr: Address,
        value: Reg<f64>,
        memory: Memory
    },
    I32StoreWrap8_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I32StoreWrap8Mem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    I32StoreWrap8_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: i8,
        memory: Memory
    },
    I32StoreWrap8Mem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: i8
    },
    I32StoreWrap8_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    I32StoreWrap8Mem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    I32StoreWrap8_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I32StoreWrap8Mem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    I32StoreWrap8_Si {
        ptr: Slot,
        offset: Offset,
        value: i8,
        memory: Memory
    },
    I32StoreWrap8Mem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: i8
    },
    I32StoreWrap8_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    I32StoreWrap8_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    I32StoreWrap8_Ii {
        ptr: Address,
        value: i8,
        memory: Memory
    },
    I32StoreWrap16_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I32StoreWrap16Mem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    I32StoreWrap16_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: i16,
        memory: Memory
    },
    I32StoreWrap16Mem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: i16
    },
    I32StoreWrap16_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    I32StoreWrap16Mem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    I32StoreWrap16_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I32StoreWrap16Mem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    I32StoreWrap16_Si {
        ptr: Slot,
        offset: Offset,
        value: i16,
        memory: Memory
    },
    I32StoreWrap16Mem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: i16
    },
    I32StoreWrap16_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    I32StoreWrap16_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    I32StoreWrap16_Ii {
        ptr: Address,
        value: i16,
        memory: Memory
    },
    I64StoreWrap8_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap8Mem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    I64StoreWrap8_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: i8,
        memory: Memory
    },
    I64StoreWrap8Mem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: i8
    },
    I64StoreWrap8_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    I64StoreWrap8Mem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    I64StoreWrap8_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap8Mem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    I64StoreWrap8_Si {
        ptr: Slot,
        offset: Offset,
        value: i8,
        memory: Memory
    },
    I64StoreWrap8Mem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: i8
    },
    I64StoreWrap8_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    I64StoreWrap8_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap8_Ii {
        ptr: Address,
        value: i8,
        memory: Memory
    },
    I64StoreWrap16_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap16Mem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    I64StoreWrap16_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: i16,
        memory: Memory
    },
    I64StoreWrap16Mem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: i16
    },
    I64StoreWrap16_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    I64StoreWrap16Mem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    I64StoreWrap16_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap16Mem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    I64StoreWrap16_Si {
        ptr: Slot,
        offset: Offset,
        value: i16,
        memory: Memory
    },
    I64StoreWrap16Mem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: i16
    },
    I64StoreWrap16_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    I64StoreWrap16_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap16_Ii {
        ptr: Address,
        value: i16,
        memory: Memory
    },
    I64StoreWrap32_Rs {
        ptr: Reg<i64>,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap32Mem0Offset16_Rs {
        ptr: Reg<i64>,
        offset: Offset16,
        value: Slot
    },
    I64StoreWrap32_Ri {
        ptr: Reg<i64>,
        offset: Offset,
        value: i32,
        memory: Memory
    },
    I64StoreWrap32Mem0Offset16_Ri {
        ptr: Reg<i64>,
        offset: Offset16,
        value: i32
    },
    I64StoreWrap32_Sr {
        ptr: Slot,
        offset: Offset,
        value: Reg<i64>,
        memory: Memory
    },
    I64StoreWrap32Mem0Offset16_Sr {
        ptr: Slot,
        offset: Offset16,
        value: Reg<i64>
    },
    I64StoreWrap32_Ss {
        ptr: Slot,
        offset: Offset,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap32Mem0Offset16_Ss {
        ptr: Slot,
        offset: Offset16,
        value: Slot
    },
    I64StoreWrap32_Si {
        ptr: Slot,
        offset: Offset,
        value: i32,
        memory: Memory
    },
    I64StoreWrap32Mem0Offset16_Si {
        ptr: Slot,
        offset: Offset16,
        value: i32
    },
    I64StoreWrap32_Ir {
        ptr: Address,
        value: Reg<i64>,
        memory: Memory
    },
    I64StoreWrap32_Is {
        ptr: Address,
        value: Slot,
        memory: Memory
    },
    I64StoreWrap32_Ii {
        ptr: Address,
        value: i32,
        memory: Memory
    },
    Return {

    },
    Trap {
        trap_code: TrapCode
    },
    ConsumeFuel {
        fuel: BlockFuel
    },
    Branch {
        offset: BranchOffset
    },
    BranchTable_R {
        len_targets: u32,
        index: Reg<i64>
    },
    BranchTable_S {
        len_targets: u32,
        index: Slot
    },
    BranchTableSpan_R {
        len_targets: u32,
        index: Reg<i64>,
        values: BoundedSlotSpan
    },
    BranchTableSpan_S {
        len_targets: u32,
        index: Slot,
        values: BoundedSlotSpan
    },
    U32Copy_Ri {
        result: Reg<i64>,
        value: u32
    },
    U32Copy_Si {
        result: Slot,
        value: u32
    },
    U64Copy_Rs {
        result: Reg<i64>,
        value: Slot
    },
    U64Copy_Ri {
        result: Reg<i64>,
        value: u64
    },
    U64Copy_Sr {
        result: Slot,
        value: Reg<i64>
    },
    U64Copy_Ss {
        result: Slot,
        value: Slot
    },
    U64Copy_Si {
        result: Slot,
        value: u64
    },
    F32Copy_Ri {
        result: Reg<f32>,
        value: f32
    },
    F32Copy_Rs {
        result: Reg<f32>,
        value: Slot
    },
    F32Copy_Sr {
        result: Slot,
        value: Reg<f32>
    },
    F64Copy_Rs {
        result: Reg<f64>,
        value: Slot
    },
    F64Copy_Ri {
        result: Reg<f64>,
        value: f64
    },
    F64Copy_Sr {
        result: Slot,
        value: Reg<f64>
    },
    U64Copy_S0s1 {
        result: Local<0>,
        value: Local<1>
    },
    U64Copy_S0s2 {
        result: Local<0>,
        value: Local<2>
    },
    U64Copy_S0s3 {
        result: Local<0>,
        value: Local<3>
    },
    U64Copy_S0s4 {
        result: Local<0>,
        value: Local<4>
    },
    U64Copy_S0s5 {
        result: Local<0>,
        value: Local<5>
    },
    U64Copy_S1s0 {
        result: Local<1>,
        value: Local<0>
    },
    U64Copy_S1s2 {
        result: Local<1>,
        value: Local<2>
    },
    U64Copy_S1s3 {
        result: Local<1>,
        value: Local<3>
    },
    U64Copy_S1s4 {
        result: Local<1>,
        value: Local<4>
    },
    U64Copy_S1s5 {
        result: Local<1>,
        value: Local<5>
    },
    U64Copy_S2s0 {
        result: Local<2>,
        value: Local<0>
    },
    U64Copy_S2s1 {
        result: Local<2>,
        value: Local<1>
    },
    U64Copy_S2s3 {
        result: Local<2>,
        value: Local<3>
    },
    U64Copy_S2s4 {
        result: Local<2>,
        value: Local<4>
    },
    U64Copy_S2s5 {
        result: Local<2>,
        value: Local<5>
    },
    U64Copy_S3s0 {
        result: Local<3>,
        value: Local<0>
    },
    U64Copy_S3s1 {
        result: Local<3>,
        value: Local<1>
    },
    U64Copy_S3s2 {
        result: Local<3>,
        value: Local<2>
    },
    U64Copy_S3s4 {
        result: Local<3>,
        value: Local<4>
    },
    U64Copy_S3s5 {
        result: Local<3>,
        value: Local<5>
    },
    U64Copy_S4s0 {
        result: Local<4>,
        value: Local<0>
    },
    U64Copy_S4s1 {
        result: Local<4>,
        value: Local<1>
    },
    U64Copy_S4s2 {
        result: Local<4>,
        value: Local<2>
    },
    U64Copy_S4s3 {
        result: Local<4>,
        value: Local<3>
    },
    U64Copy_S4s5 {
        result: Local<4>,
        value: Local<5>
    },
    U64Copy_S5s0 {
        result: Local<5>,
        value: Local<0>
    },
    U64Copy_S5s1 {
        result: Local<5>,
        value: Local<1>
    },
    U64Copy_S5s2 {
        result: Local<5>,
        value: Local<2>
    },
    U64Copy_S5s3 {
        result: Local<5>,
        value: Local<3>
    },
    U64Copy_S5s4 {
        result: Local<5>,
        value: Local<4>
    },
    F32ReinterpretI32_Rr {
        result: Reg<f32>,
        value: Reg<i64>
    },
    I32ReinterpretF32_Rr {
        result: Reg<i64>,
        value: Reg<f32>
    },
    F64ReinterpretI64_Rr {
        result: Reg<f64>,
        value: Reg<i64>
    },
    I64ReinterpretF64_Rr {
        result: Reg<i64>,
        value: Reg<f64>
    },
    U64Copy_S0r {
        result: Local<0>,
        value: Reg<i64>
    },
    U64Copy_S1r {
        result: Local<1>,
        value: Reg<i64>
    },
    U64Copy_S2r {
        result: Local<2>,
        value: Reg<i64>
    },
    U64Copy_S3r {
        result: Local<3>,
        value: Reg<i64>
    },
    U64Copy_S4r {
        result: Local<4>,
        value: Reg<i64>
    },
    U64Copy_S5r {
        result: Local<5>,
        value: Reg<i64>
    },
    U64Copy_S6r {
        result: Local<6>,
        value: Reg<i64>
    },
    U64Copy_S7r {
        result: Local<7>,
        value: Reg<i64>
    },
    U64Copy_S8r {
        result: Local<8>,
        value: Reg<i64>
    },
    U64Copy_S9r {
        result: Local<9>,
        value: Reg<i64>
    },
    F32Copy_S0r {
        result: Local<0>,
        value: Reg<f32>
    },
    F32Copy_S1r {
        result: Local<1>,
        value: Reg<f32>
    },
    F32Copy_S2r {
        result: Local<2>,
        value: Reg<f32>
    },
    F32Copy_S3r {
        result: Local<3>,
        value: Reg<f32>
    },
    F32Copy_S4r {
        result: Local<4>,
        value: Reg<f32>
    },
    F32Copy_S5r {
        result: Local<5>,
        value: Reg<f32>
    },
    F32Copy_S6r {
        result: Local<6>,
        value: Reg<f32>
    },
    F32Copy_S7r {
        result: Local<7>,
        value: Reg<f32>
    },
    F32Copy_S8r {
        result: Local<8>,
        value: Reg<f32>
    },
    F32Copy_S9r {
        result: Local<9>,
        value: Reg<f32>
    },
    F64Copy_S0r {
        result: Local<0>,
        value: Reg<f64>
    },
    F64Copy_S1r {
        result: Local<1>,
        value: Reg<f64>
    },
    F64Copy_S2r {
        result: Local<2>,
        value: Reg<f64>
    },
    F64Copy_S3r {
        result: Local<3>,
        value: Reg<f64>
    },
    F64Copy_S4r {
        result: Local<4>,
        value: Reg<f64>
    },
    F64Copy_S5r {
        result: Local<5>,
        value: Reg<f64>
    },
    F64Copy_S6r {
        result: Local<6>,
        value: Reg<f64>
    },
    F64Copy_S7r {
        result: Local<7>,
        value: Reg<f64>
    },
    F64Copy_S8r {
        result: Local<8>,
        value: Reg<f64>
    },
    F64Copy_S9r {
        result: Local<9>,
        value: Reg<f64>
    },
    RefFunc {
        result: Reg<i64>,
        func: Func
    },
    CallInternal {
        params: BoundedSlotSpan,
        func: InternalFunc
    },
    CallImported {
        params: BoundedSlotSpan,
        func: Func
    },
    CallIndirect_R {
        table: Table,
        func_type: FuncType,
        params: BoundedSlotSpan,
        index: Reg<i64>
    },
    CallIndirect_S {
        table: Table,
        func_type: FuncType,
        params: BoundedSlotSpan,
        index: Slot
    },
    ReturnCallIndirect_R {
        table: Table,
        func_type: FuncType,
        params: BoundedSlotSpan,
        index: Reg<i64>
    },
    ReturnCallIndirect_S {
        table: Table,
        func_type: FuncType,
        params: BoundedSlotSpan,
        index: Slot
    },
    ReturnCallInternal {
        params: BoundedSlotSpan,
        func: InternalFunc
    },
    ReturnCallImported {
        params: BoundedSlotSpan,
        func: Func
    },
    GlobalGetU64_R {
        global: Global,
        result: Reg<i64>
    },
    GlobalGetF32_R {
        global: Global,
        result: Reg<f32>
    },
    GlobalGetF64_R {
        global: Global,
        result: Reg<f64>
    },
    GlobalSetU32_I {
        global: Global,
        value: u32
    },
    GlobalSetU64_R {
        global: Global,
        value: Reg<i64>
    },
    GlobalSetU64_S {
        global: Global,
        value: Slot
    },
    GlobalSetU64_I {
        global: Global,
        value: u64
    },
    GlobalSetF32_R {
        global: Global,
        value: Reg<f32>
    },
    GlobalSetF64_R {
        global: Global,
        value: Reg<f64>
    },
    DataDrop {
        data: Data
    },
    MemorySize {
        result: Reg<i64>,
        memory: Memory
    },
    MemoryGrow {
        result: Reg<i64>,
        delta: Slot,
        memory: Memory
    },
    MemoryCopy {
        dst_memory: Memory,
        src_memory: Memory,
        dst: Slot,
        src: Slot,
        len: Slot
    },
    MemoryFill {
        memory: Memory,
        dst: Slot,
        len: Slot,
        value: Slot
    },
    MemoryInit {
        memory: Memory,
        data: Data,
        dst: Slot,
        src: Slot,
        len: Slot
    },
    TableGet_Rr {
        result: Reg<i64>,
        index: Reg<i64>,
        table: Table
    },
    TableSet_Rs {
        table: Table,
        index: Reg<i64>,
        value: Slot
    },
    TableSet_Ri {
        table: Table,
        index: Reg<i64>,
        value: u32
    },
    TableGet_Rs {
        result: Reg<i64>,
        index: Slot,
        table: Table
    },
    TableSet_Sr {
        table: Table,
        index: Slot,
        value: Reg<i64>
    },
    TableSet_Ss {
        table: Table,
        index: Slot,
        value: Slot
    },
    TableSet_Si {
        table: Table,
        index: Slot,
        value: u32
    },
    TableGet_Ri {
        result: Reg<i64>,
        index: u32,
        table: Table
    },
    TableSet_Ir {
        table: Table,
        index: u32,
        value: Reg<i64>
    },
    TableSet_Is {
        table: Table,
        index: u32,
        value: Slot
    },
    TableSet_Ii {
        table: Table,
        index: u32,
        value: u32
    },
    TableSize {
        result: Reg<i64>,
        table: Table
    },
    TableGrow {
        result: Reg<i64>,
        delta: Slot,
        value: Slot,
        table: Table
    },
    TableCopy {
        dst_table: Table,
        src_table: Table,
        dst: Slot,
        src: Slot,
        len: Slot
    },
    TableFill {
        table: Table,
        dst: Slot,
        len: Slot,
        value: Slot
    },
    TableInit {
        table: Table,
        elem: Elem,
        dst: Slot,
        src: Slot,
        len: Slot
    },
    ElemDrop {
        elem: Elem
    },
    I64Add128 {
        results: FixedSlotSpan<2>,
        lhs_lo: Slot,
        lhs_hi: Slot,
        rhs_lo: Slot,
        rhs_hi: Slot
    },
    I64Sub128 {
        results: FixedSlotSpan<2>,
        lhs_lo: Slot,
        lhs_hi: Slot,
        rhs_lo: Slot,
        rhs_hi: Slot
    },
    I64MulWide {
        results: FixedSlotSpan<2>,
        lhs: Slot,
        rhs: Slot
    },
    U64MulWide {
        results: FixedSlotSpan<2>,
        lhs: Slot,
        rhs: Slot
    }
}

/// The number of unique operator variants in [`Op`].
            pub const LEN_OPS: ::core::primitive::usize = 1129;

/// Expands `mac` using the snake-case and camel-case identifiers of all operators.
            /// 
/// # Note
/// 
/// Simd related operators are only included if the `simd` crate feature is enabled.
/// 
/// # Example
/// 
/// The expanded code format fed to the `mac` macro is as follows:
/// 
/// ```no-compile
/// i32_add_sss => I32Add_Sss,
/// i32_add_ssi => I32Add_Ssi,
/// i32_sub_sss => I32Sub_Sss,
/// i32_sub_ssi => I32Sub_Ssi,
/// i32_sub_sis => I32Sub_Sis,
/// i32_mul_sss => I32Mul_Sss,
/// i32_mul_ssi => I32Mul_Ssi,
/// etc ..
/// ```
#[macro_export]
macro_rules! for_each_op {
    ($mac:ident) => {
        $mac! {
        i32_clz_rs => I32Clz_Rs,
        i32_clz_rr => I32Clz_Rr,
        i32_ctz_rs => I32Ctz_Rs,
        i32_ctz_rr => I32Ctz_Rr,
        i32_popcnt_rs => I32Popcnt_Rs,
        i32_popcnt_rr => I32Popcnt_Rr,
        i32_sext8_rs => I32Sext8_Rs,
        i32_sext8_rr => I32Sext8_Rr,
        i32_sext16_rs => I32Sext16_Rs,
        i32_sext16_rr => I32Sext16_Rr,
        i32_wrap_i64_rs => I32WrapI64_Rs,
        i32_wrap_i64_rr => I32WrapI64_Rr,
        i64_clz_rs => I64Clz_Rs,
        i64_clz_rr => I64Clz_Rr,
        i64_ctz_rs => I64Ctz_Rs,
        i64_ctz_rr => I64Ctz_Rr,
        i64_popcnt_rs => I64Popcnt_Rs,
        i64_popcnt_rr => I64Popcnt_Rr,
        i64_sext8_rs => I64Sext8_Rs,
        i64_sext8_rr => I64Sext8_Rr,
        i64_sext16_rs => I64Sext16_Rs,
        i64_sext16_rr => I64Sext16_Rr,
        i64_sext32_rs => I64Sext32_Rs,
        i64_sext32_rr => I64Sext32_Rr,
        f32_abs_rs => F32Abs_Rs,
        f32_abs_rr => F32Abs_Rr,
        f32_neg_rs => F32Neg_Rs,
        f32_neg_rr => F32Neg_Rr,
        f32_nabs_rs => F32Nabs_Rs,
        f32_nabs_rr => F32Nabs_Rr,
        f32_ceil_rs => F32Ceil_Rs,
        f32_ceil_rr => F32Ceil_Rr,
        f32_floor_rs => F32Floor_Rs,
        f32_floor_rr => F32Floor_Rr,
        f32_trunc_rs => F32Trunc_Rs,
        f32_trunc_rr => F32Trunc_Rr,
        f32_nearest_rs => F32Nearest_Rs,
        f32_nearest_rr => F32Nearest_Rr,
        f32_sqrt_rs => F32Sqrt_Rs,
        f32_sqrt_rr => F32Sqrt_Rr,
        f32_convert_i32_rs => F32ConvertI32_Rs,
        f32_convert_i32_rr => F32ConvertI32_Rr,
        f32_convert_u32_rs => F32ConvertU32_Rs,
        f32_convert_u32_rr => F32ConvertU32_Rr,
        f32_convert_i64_rs => F32ConvertI64_Rs,
        f32_convert_i64_rr => F32ConvertI64_Rr,
        f32_convert_u64_rs => F32ConvertU64_Rs,
        f32_convert_u64_rr => F32ConvertU64_Rr,
        f32_demote_f64_rs => F32DemoteF64_Rs,
        f32_demote_f64_rr => F32DemoteF64_Rr,
        f64_abs_rs => F64Abs_Rs,
        f64_abs_rr => F64Abs_Rr,
        f64_neg_rs => F64Neg_Rs,
        f64_neg_rr => F64Neg_Rr,
        f64_nabs_rs => F64Nabs_Rs,
        f64_nabs_rr => F64Nabs_Rr,
        f64_ceil_rs => F64Ceil_Rs,
        f64_ceil_rr => F64Ceil_Rr,
        f64_floor_rs => F64Floor_Rs,
        f64_floor_rr => F64Floor_Rr,
        f64_trunc_rs => F64Trunc_Rs,
        f64_trunc_rr => F64Trunc_Rr,
        f64_nearest_rs => F64Nearest_Rs,
        f64_nearest_rr => F64Nearest_Rr,
        f64_sqrt_rs => F64Sqrt_Rs,
        f64_sqrt_rr => F64Sqrt_Rr,
        f64_convert_i32_rs => F64ConvertI32_Rs,
        f64_convert_i32_rr => F64ConvertI32_Rr,
        f64_convert_u32_rs => F64ConvertU32_Rs,
        f64_convert_u32_rr => F64ConvertU32_Rr,
        f64_convert_i64_rs => F64ConvertI64_Rs,
        f64_convert_i64_rr => F64ConvertI64_Rr,
        f64_convert_u64_rs => F64ConvertU64_Rs,
        f64_convert_u64_rr => F64ConvertU64_Rr,
        f64_promote_f32_rs => F64PromoteF32_Rs,
        f64_promote_f32_rr => F64PromoteF32_Rr,
        i32_trunc_f32_rs => I32TruncF32_Rs,
        i32_trunc_f32_rr => I32TruncF32_Rr,
        u32_trunc_f32_rs => U32TruncF32_Rs,
        u32_trunc_f32_rr => U32TruncF32_Rr,
        i32_trunc_f64_rs => I32TruncF64_Rs,
        i32_trunc_f64_rr => I32TruncF64_Rr,
        u32_trunc_f64_rs => U32TruncF64_Rs,
        u32_trunc_f64_rr => U32TruncF64_Rr,
        i64_trunc_f32_rs => I64TruncF32_Rs,
        i64_trunc_f32_rr => I64TruncF32_Rr,
        u64_trunc_f32_rs => U64TruncF32_Rs,
        u64_trunc_f32_rr => U64TruncF32_Rr,
        i64_trunc_f64_rs => I64TruncF64_Rs,
        i64_trunc_f64_rr => I64TruncF64_Rr,
        u64_trunc_f64_rs => U64TruncF64_Rs,
        u64_trunc_f64_rr => U64TruncF64_Rr,
        i32_trunc_sat_f32_rs => I32TruncSatF32_Rs,
        i32_trunc_sat_f32_rr => I32TruncSatF32_Rr,
        u32_trunc_sat_f32_rs => U32TruncSatF32_Rs,
        u32_trunc_sat_f32_rr => U32TruncSatF32_Rr,
        i32_trunc_sat_f64_rs => I32TruncSatF64_Rs,
        i32_trunc_sat_f64_rr => I32TruncSatF64_Rr,
        u32_trunc_sat_f64_rs => U32TruncSatF64_Rs,
        u32_trunc_sat_f64_rr => U32TruncSatF64_Rr,
        i64_trunc_sat_f32_rs => I64TruncSatF32_Rs,
        i64_trunc_sat_f32_rr => I64TruncSatF32_Rr,
        u64_trunc_sat_f32_rs => U64TruncSatF32_Rs,
        u64_trunc_sat_f32_rr => U64TruncSatF32_Rr,
        i64_trunc_sat_f64_rs => I64TruncSatF64_Rs,
        i64_trunc_sat_f64_rr => I64TruncSatF64_Rr,
        u64_trunc_sat_f64_rs => U64TruncSatF64_Rs,
        u64_trunc_sat_f64_rr => U64TruncSatF64_Rr,
        i32_eq_rrs => I32Eq_Rrs,
        i32_eq_rri => I32Eq_Rri,
        i32_eq_rss => I32Eq_Rss,
        i32_eq_rsi => I32Eq_Rsi,
        i32_and_rrs => I32And_Rrs,
        i32_and_rri => I32And_Rri,
        i32_and_rss => I32And_Rss,
        i32_and_rsi => I32And_Rsi,
        i32_or_rrs => I32Or_Rrs,
        i32_or_rri => I32Or_Rri,
        i32_or_rss => I32Or_Rss,
        i32_or_rsi => I32Or_Rsi,
        i32_not_eq_rrs => I32NotEq_Rrs,
        i32_not_eq_rri => I32NotEq_Rri,
        i32_not_eq_rss => I32NotEq_Rss,
        i32_not_eq_rsi => I32NotEq_Rsi,
        i32_not_and_rrs => I32NotAnd_Rrs,
        i32_not_and_rri => I32NotAnd_Rri,
        i32_not_and_rss => I32NotAnd_Rss,
        i32_not_and_rsi => I32NotAnd_Rsi,
        i32_not_or_rrs => I32NotOr_Rrs,
        i32_not_or_rri => I32NotOr_Rri,
        i32_not_or_rss => I32NotOr_Rss,
        i32_not_or_rsi => I32NotOr_Rsi,
        i32_lt_rrs => I32Lt_Rrs,
        i32_lt_rri => I32Lt_Rri,
        i32_lt_rsr => I32Lt_Rsr,
        i32_lt_rss => I32Lt_Rss,
        i32_lt_rsi => I32Lt_Rsi,
        i32_lt_rir => I32Lt_Rir,
        i32_lt_ris => I32Lt_Ris,
        i32_le_rrs => I32Le_Rrs,
        i32_le_rri => I32Le_Rri,
        i32_le_rsr => I32Le_Rsr,
        i32_le_rss => I32Le_Rss,
        i32_le_rsi => I32Le_Rsi,
        i32_le_rir => I32Le_Rir,
        i32_le_ris => I32Le_Ris,
        u32_lt_rrs => U32Lt_Rrs,
        u32_lt_rri => U32Lt_Rri,
        u32_lt_rsr => U32Lt_Rsr,
        u32_lt_rss => U32Lt_Rss,
        u32_lt_rsi => U32Lt_Rsi,
        u32_lt_rir => U32Lt_Rir,
        u32_lt_ris => U32Lt_Ris,
        u32_le_rrs => U32Le_Rrs,
        u32_le_rri => U32Le_Rri,
        u32_le_rsr => U32Le_Rsr,
        u32_le_rss => U32Le_Rss,
        u32_le_rsi => U32Le_Rsi,
        u32_le_rir => U32Le_Rir,
        u32_le_ris => U32Le_Ris,
        i64_eq_rrs => I64Eq_Rrs,
        i64_eq_rri => I64Eq_Rri,
        i64_eq_rss => I64Eq_Rss,
        i64_eq_rsi => I64Eq_Rsi,
        i64_and_rrs => I64And_Rrs,
        i64_and_rri => I64And_Rri,
        i64_and_rss => I64And_Rss,
        i64_and_rsi => I64And_Rsi,
        i64_or_rrs => I64Or_Rrs,
        i64_or_rri => I64Or_Rri,
        i64_or_rss => I64Or_Rss,
        i64_or_rsi => I64Or_Rsi,
        i64_not_eq_rrs => I64NotEq_Rrs,
        i64_not_eq_rri => I64NotEq_Rri,
        i64_not_eq_rss => I64NotEq_Rss,
        i64_not_eq_rsi => I64NotEq_Rsi,
        i64_not_and_rrs => I64NotAnd_Rrs,
        i64_not_and_rri => I64NotAnd_Rri,
        i64_not_and_rss => I64NotAnd_Rss,
        i64_not_and_rsi => I64NotAnd_Rsi,
        i64_not_or_rrs => I64NotOr_Rrs,
        i64_not_or_rri => I64NotOr_Rri,
        i64_not_or_rss => I64NotOr_Rss,
        i64_not_or_rsi => I64NotOr_Rsi,
        i64_lt_rrs => I64Lt_Rrs,
        i64_lt_rri => I64Lt_Rri,
        i64_lt_rsr => I64Lt_Rsr,
        i64_lt_rss => I64Lt_Rss,
        i64_lt_rsi => I64Lt_Rsi,
        i64_lt_rir => I64Lt_Rir,
        i64_lt_ris => I64Lt_Ris,
        i64_le_rrs => I64Le_Rrs,
        i64_le_rri => I64Le_Rri,
        i64_le_rsr => I64Le_Rsr,
        i64_le_rss => I64Le_Rss,
        i64_le_rsi => I64Le_Rsi,
        i64_le_rir => I64Le_Rir,
        i64_le_ris => I64Le_Ris,
        u64_lt_rrs => U64Lt_Rrs,
        u64_lt_rri => U64Lt_Rri,
        u64_lt_rsr => U64Lt_Rsr,
        u64_lt_rss => U64Lt_Rss,
        u64_lt_rsi => U64Lt_Rsi,
        u64_lt_rir => U64Lt_Rir,
        u64_lt_ris => U64Lt_Ris,
        u64_le_rrs => U64Le_Rrs,
        u64_le_rri => U64Le_Rri,
        u64_le_rsr => U64Le_Rsr,
        u64_le_rss => U64Le_Rss,
        u64_le_rsi => U64Le_Rsi,
        u64_le_rir => U64Le_Rir,
        u64_le_ris => U64Le_Ris,
        f32_eq_rrs => F32Eq_Rrs,
        f32_eq_rri => F32Eq_Rri,
        f32_eq_rss => F32Eq_Rss,
        f32_eq_rsi => F32Eq_Rsi,
        f32_lt_rrs => F32Lt_Rrs,
        f32_lt_rri => F32Lt_Rri,
        f32_lt_rsr => F32Lt_Rsr,
        f32_lt_rss => F32Lt_Rss,
        f32_lt_rsi => F32Lt_Rsi,
        f32_lt_rir => F32Lt_Rir,
        f32_lt_ris => F32Lt_Ris,
        f32_le_rrs => F32Le_Rrs,
        f32_le_rri => F32Le_Rri,
        f32_le_rsr => F32Le_Rsr,
        f32_le_rss => F32Le_Rss,
        f32_le_rsi => F32Le_Rsi,
        f32_le_rir => F32Le_Rir,
        f32_le_ris => F32Le_Ris,
        f32_not_eq_rrs => F32NotEq_Rrs,
        f32_not_eq_rri => F32NotEq_Rri,
        f32_not_eq_rss => F32NotEq_Rss,
        f32_not_eq_rsi => F32NotEq_Rsi,
        f32_not_lt_rrs => F32NotLt_Rrs,
        f32_not_lt_rri => F32NotLt_Rri,
        f32_not_lt_rsr => F32NotLt_Rsr,
        f32_not_lt_rss => F32NotLt_Rss,
        f32_not_lt_rsi => F32NotLt_Rsi,
        f32_not_lt_rir => F32NotLt_Rir,
        f32_not_lt_ris => F32NotLt_Ris,
        f32_not_le_rrs => F32NotLe_Rrs,
        f32_not_le_rri => F32NotLe_Rri,
        f32_not_le_rsr => F32NotLe_Rsr,
        f32_not_le_rss => F32NotLe_Rss,
        f32_not_le_rsi => F32NotLe_Rsi,
        f32_not_le_rir => F32NotLe_Rir,
        f32_not_le_ris => F32NotLe_Ris,
        f64_eq_rrs => F64Eq_Rrs,
        f64_eq_rri => F64Eq_Rri,
        f64_eq_rss => F64Eq_Rss,
        f64_eq_rsi => F64Eq_Rsi,
        f64_lt_rrs => F64Lt_Rrs,
        f64_lt_rri => F64Lt_Rri,
        f64_lt_rsr => F64Lt_Rsr,
        f64_lt_rss => F64Lt_Rss,
        f64_lt_rsi => F64Lt_Rsi,
        f64_lt_rir => F64Lt_Rir,
        f64_lt_ris => F64Lt_Ris,
        f64_le_rrs => F64Le_Rrs,
        f64_le_rri => F64Le_Rri,
        f64_le_rsr => F64Le_Rsr,
        f64_le_rss => F64Le_Rss,
        f64_le_rsi => F64Le_Rsi,
        f64_le_rir => F64Le_Rir,
        f64_le_ris => F64Le_Ris,
        f64_not_eq_rrs => F64NotEq_Rrs,
        f64_not_eq_rri => F64NotEq_Rri,
        f64_not_eq_rss => F64NotEq_Rss,
        f64_not_eq_rsi => F64NotEq_Rsi,
        f64_not_lt_rrs => F64NotLt_Rrs,
        f64_not_lt_rri => F64NotLt_Rri,
        f64_not_lt_rsr => F64NotLt_Rsr,
        f64_not_lt_rss => F64NotLt_Rss,
        f64_not_lt_rsi => F64NotLt_Rsi,
        f64_not_lt_rir => F64NotLt_Rir,
        f64_not_lt_ris => F64NotLt_Ris,
        f64_not_le_rrs => F64NotLe_Rrs,
        f64_not_le_rri => F64NotLe_Rri,
        f64_not_le_rsr => F64NotLe_Rsr,
        f64_not_le_rss => F64NotLe_Rss,
        f64_not_le_rsi => F64NotLe_Rsi,
        f64_not_le_rir => F64NotLe_Rir,
        f64_not_le_ris => F64NotLe_Ris,
        i32_add_rrs => I32Add_Rrs,
        i32_add_rri => I32Add_Rri,
        i32_add_rss => I32Add_Rss,
        i32_add_rsi => I32Add_Rsi,
        i32_add_rs_rs => I32Add_Rs_rs,
        i32_add_rs_ri => I32Add_Rs_ri,
        i32_add_rs_ss => I32Add_Rs_ss,
        i32_add_rs_si => I32Add_Rs_si,
        i32_sub_rrs => I32Sub_Rrs,
        i32_sub_rsr => I32Sub_Rsr,
        i32_sub_rss => I32Sub_Rss,
        i32_sub_rir => I32Sub_Rir,
        i32_sub_ris => I32Sub_Ris,
        i32_mul_rrs => I32Mul_Rrs,
        i32_mul_rri => I32Mul_Rri,
        i32_mul_rss => I32Mul_Rss,
        i32_mul_rsi => I32Mul_Rsi,
        i32_div_rrs => I32Div_Rrs,
        i32_div_rri => I32Div_Rri,
        i32_div_rsr => I32Div_Rsr,
        i32_div_rss => I32Div_Rss,
        i32_div_rsi => I32Div_Rsi,
        i32_div_rir => I32Div_Rir,
        i32_div_ris => I32Div_Ris,
        u32_div_rrs => U32Div_Rrs,
        u32_div_rri => U32Div_Rri,
        u32_div_rsr => U32Div_Rsr,
        u32_div_rss => U32Div_Rss,
        u32_div_rsi => U32Div_Rsi,
        u32_div_rir => U32Div_Rir,
        u32_div_ris => U32Div_Ris,
        i32_rem_rrs => I32Rem_Rrs,
        i32_rem_rri => I32Rem_Rri,
        i32_rem_rsr => I32Rem_Rsr,
        i32_rem_rss => I32Rem_Rss,
        i32_rem_rsi => I32Rem_Rsi,
        i32_rem_rir => I32Rem_Rir,
        i32_rem_ris => I32Rem_Ris,
        u32_rem_rrs => U32Rem_Rrs,
        u32_rem_rri => U32Rem_Rri,
        u32_rem_rsr => U32Rem_Rsr,
        u32_rem_rss => U32Rem_Rss,
        u32_rem_rsi => U32Rem_Rsi,
        u32_rem_rir => U32Rem_Rir,
        u32_rem_ris => U32Rem_Ris,
        i32_bitand_rrs => I32BitAnd_Rrs,
        i32_bitand_rri => I32BitAnd_Rri,
        i32_bitand_rss => I32BitAnd_Rss,
        i32_bitand_rsi => I32BitAnd_Rsi,
        i32_bitor_rrs => I32BitOr_Rrs,
        i32_bitor_rri => I32BitOr_Rri,
        i32_bitor_rss => I32BitOr_Rss,
        i32_bitor_rsi => I32BitOr_Rsi,
        i32_bitxor_rrs => I32BitXor_Rrs,
        i32_bitxor_rri => I32BitXor_Rri,
        i32_bitxor_rss => I32BitXor_Rss,
        i32_bitxor_rsi => I32BitXor_Rsi,
        i32_shl_rrs => I32Shl_Rrs,
        i32_shl_rri => I32Shl_Rri,
        i32_shl_rsr => I32Shl_Rsr,
        i32_shl_rss => I32Shl_Rss,
        i32_shl_rsi => I32Shl_Rsi,
        i32_shl_rir => I32Shl_Rir,
        i32_shl_ris => I32Shl_Ris,
        i32_shr_rrs => I32Shr_Rrs,
        i32_shr_rri => I32Shr_Rri,
        i32_shr_rsr => I32Shr_Rsr,
        i32_shr_rss => I32Shr_Rss,
        i32_shr_rsi => I32Shr_Rsi,
        i32_shr_rir => I32Shr_Rir,
        i32_shr_ris => I32Shr_Ris,
        u32_shr_rrs => U32Shr_Rrs,
        u32_shr_rri => U32Shr_Rri,
        u32_shr_rsr => U32Shr_Rsr,
        u32_shr_rss => U32Shr_Rss,
        u32_shr_rsi => U32Shr_Rsi,
        u32_shr_rir => U32Shr_Rir,
        u32_shr_ris => U32Shr_Ris,
        i32_rotl_rrs => I32Rotl_Rrs,
        i32_rotl_rri => I32Rotl_Rri,
        i32_rotl_rsr => I32Rotl_Rsr,
        i32_rotl_rss => I32Rotl_Rss,
        i32_rotl_rsi => I32Rotl_Rsi,
        i32_rotl_rir => I32Rotl_Rir,
        i32_rotl_ris => I32Rotl_Ris,
        i32_rotr_rrs => I32Rotr_Rrs,
        i32_rotr_rri => I32Rotr_Rri,
        i32_rotr_rsr => I32Rotr_Rsr,
        i32_rotr_rss => I32Rotr_Rss,
        i32_rotr_rsi => I32Rotr_Rsi,
        i32_rotr_rir => I32Rotr_Rir,
        i32_rotr_ris => I32Rotr_Ris,
        i64_add_rrs => I64Add_Rrs,
        i64_add_rri => I64Add_Rri,
        i64_add_rss => I64Add_Rss,
        i64_add_rsi => I64Add_Rsi,
        i64_add_rs_rs => I64Add_Rs_rs,
        i64_add_rs_ri => I64Add_Rs_ri,
        i64_add_rs_ss => I64Add_Rs_ss,
        i64_add_rs_si => I64Add_Rs_si,
        i64_sub_rrs => I64Sub_Rrs,
        i64_sub_rsr => I64Sub_Rsr,
        i64_sub_rss => I64Sub_Rss,
        i64_sub_rir => I64Sub_Rir,
        i64_sub_ris => I64Sub_Ris,
        i64_mul_rrs => I64Mul_Rrs,
        i64_mul_rri => I64Mul_Rri,
        i64_mul_rss => I64Mul_Rss,
        i64_mul_rsi => I64Mul_Rsi,
        i64_div_rrs => I64Div_Rrs,
        i64_div_rri => I64Div_Rri,
        i64_div_rsr => I64Div_Rsr,
        i64_div_rss => I64Div_Rss,
        i64_div_rsi => I64Div_Rsi,
        i64_div_rir => I64Div_Rir,
        i64_div_ris => I64Div_Ris,
        u64_div_rrs => U64Div_Rrs,
        u64_div_rri => U64Div_Rri,
        u64_div_rsr => U64Div_Rsr,
        u64_div_rss => U64Div_Rss,
        u64_div_rsi => U64Div_Rsi,
        u64_div_rir => U64Div_Rir,
        u64_div_ris => U64Div_Ris,
        i64_rem_rrs => I64Rem_Rrs,
        i64_rem_rri => I64Rem_Rri,
        i64_rem_rsr => I64Rem_Rsr,
        i64_rem_rss => I64Rem_Rss,
        i64_rem_rsi => I64Rem_Rsi,
        i64_rem_rir => I64Rem_Rir,
        i64_rem_ris => I64Rem_Ris,
        u64_rem_rrs => U64Rem_Rrs,
        u64_rem_rri => U64Rem_Rri,
        u64_rem_rsr => U64Rem_Rsr,
        u64_rem_rss => U64Rem_Rss,
        u64_rem_rsi => U64Rem_Rsi,
        u64_rem_rir => U64Rem_Rir,
        u64_rem_ris => U64Rem_Ris,
        i64_bitand_rrs => I64BitAnd_Rrs,
        i64_bitand_rri => I64BitAnd_Rri,
        i64_bitand_rss => I64BitAnd_Rss,
        i64_bitand_rsi => I64BitAnd_Rsi,
        i64_bitor_rrs => I64BitOr_Rrs,
        i64_bitor_rri => I64BitOr_Rri,
        i64_bitor_rss => I64BitOr_Rss,
        i64_bitor_rsi => I64BitOr_Rsi,
        i64_bitxor_rrs => I64BitXor_Rrs,
        i64_bitxor_rri => I64BitXor_Rri,
        i64_bitxor_rss => I64BitXor_Rss,
        i64_bitxor_rsi => I64BitXor_Rsi,
        i64_shl_rrs => I64Shl_Rrs,
        i64_shl_rri => I64Shl_Rri,
        i64_shl_rsr => I64Shl_Rsr,
        i64_shl_rss => I64Shl_Rss,
        i64_shl_rsi => I64Shl_Rsi,
        i64_shl_rir => I64Shl_Rir,
        i64_shl_ris => I64Shl_Ris,
        i64_shr_rrs => I64Shr_Rrs,
        i64_shr_rri => I64Shr_Rri,
        i64_shr_rsr => I64Shr_Rsr,
        i64_shr_rss => I64Shr_Rss,
        i64_shr_rsi => I64Shr_Rsi,
        i64_shr_rir => I64Shr_Rir,
        i64_shr_ris => I64Shr_Ris,
        u64_shr_rrs => U64Shr_Rrs,
        u64_shr_rri => U64Shr_Rri,
        u64_shr_rsr => U64Shr_Rsr,
        u64_shr_rss => U64Shr_Rss,
        u64_shr_rsi => U64Shr_Rsi,
        u64_shr_rir => U64Shr_Rir,
        u64_shr_ris => U64Shr_Ris,
        i64_rotl_rrs => I64Rotl_Rrs,
        i64_rotl_rri => I64Rotl_Rri,
        i64_rotl_rsr => I64Rotl_Rsr,
        i64_rotl_rss => I64Rotl_Rss,
        i64_rotl_rsi => I64Rotl_Rsi,
        i64_rotl_rir => I64Rotl_Rir,
        i64_rotl_ris => I64Rotl_Ris,
        i64_rotr_rrs => I64Rotr_Rrs,
        i64_rotr_rri => I64Rotr_Rri,
        i64_rotr_rsr => I64Rotr_Rsr,
        i64_rotr_rss => I64Rotr_Rss,
        i64_rotr_rsi => I64Rotr_Rsi,
        i64_rotr_rir => I64Rotr_Rir,
        i64_rotr_ris => I64Rotr_Ris,
        f32_add_rrs => F32Add_Rrs,
        f32_add_rri => F32Add_Rri,
        f32_add_rsr => F32Add_Rsr,
        f32_add_rss => F32Add_Rss,
        f32_add_rsi => F32Add_Rsi,
        f32_add_rir => F32Add_Rir,
        f32_add_ris => F32Add_Ris,
        f32_sub_rrs => F32Sub_Rrs,
        f32_sub_rri => F32Sub_Rri,
        f32_sub_rsr => F32Sub_Rsr,
        f32_sub_rss => F32Sub_Rss,
        f32_sub_rsi => F32Sub_Rsi,
        f32_sub_rir => F32Sub_Rir,
        f32_sub_ris => F32Sub_Ris,
        f32_mul_rrs => F32Mul_Rrs,
        f32_mul_rri => F32Mul_Rri,
        f32_mul_rsr => F32Mul_Rsr,
        f32_mul_rss => F32Mul_Rss,
        f32_mul_rsi => F32Mul_Rsi,
        f32_mul_rir => F32Mul_Rir,
        f32_mul_ris => F32Mul_Ris,
        f32_div_rrs => F32Div_Rrs,
        f32_div_rri => F32Div_Rri,
        f32_div_rsr => F32Div_Rsr,
        f32_div_rss => F32Div_Rss,
        f32_div_rsi => F32Div_Rsi,
        f32_div_rir => F32Div_Rir,
        f32_div_ris => F32Div_Ris,
        f32_min_rrs => F32Min_Rrs,
        f32_min_rri => F32Min_Rri,
        f32_min_rsr => F32Min_Rsr,
        f32_min_rss => F32Min_Rss,
        f32_min_rsi => F32Min_Rsi,
        f32_min_rir => F32Min_Rir,
        f32_min_ris => F32Min_Ris,
        f32_max_rrs => F32Max_Rrs,
        f32_max_rri => F32Max_Rri,
        f32_max_rsr => F32Max_Rsr,
        f32_max_rss => F32Max_Rss,
        f32_max_rsi => F32Max_Rsi,
        f32_max_rir => F32Max_Rir,
        f32_max_ris => F32Max_Ris,
        f64_add_rrs => F64Add_Rrs,
        f64_add_rri => F64Add_Rri,
        f64_add_rsr => F64Add_Rsr,
        f64_add_rss => F64Add_Rss,
        f64_add_rsi => F64Add_Rsi,
        f64_add_rir => F64Add_Rir,
        f64_add_ris => F64Add_Ris,
        f64_sub_rrs => F64Sub_Rrs,
        f64_sub_rri => F64Sub_Rri,
        f64_sub_rsr => F64Sub_Rsr,
        f64_sub_rss => F64Sub_Rss,
        f64_sub_rsi => F64Sub_Rsi,
        f64_sub_rir => F64Sub_Rir,
        f64_sub_ris => F64Sub_Ris,
        f64_mul_rrs => F64Mul_Rrs,
        f64_mul_rri => F64Mul_Rri,
        f64_mul_rsr => F64Mul_Rsr,
        f64_mul_rss => F64Mul_Rss,
        f64_mul_rsi => F64Mul_Rsi,
        f64_mul_rir => F64Mul_Rir,
        f64_mul_ris => F64Mul_Ris,
        f64_div_rrs => F64Div_Rrs,
        f64_div_rri => F64Div_Rri,
        f64_div_rsr => F64Div_Rsr,
        f64_div_rss => F64Div_Rss,
        f64_div_rsi => F64Div_Rsi,
        f64_div_rir => F64Div_Rir,
        f64_div_ris => F64Div_Ris,
        f64_min_rrs => F64Min_Rrs,
        f64_min_rri => F64Min_Rri,
        f64_min_rsr => F64Min_Rsr,
        f64_min_rss => F64Min_Rss,
        f64_min_rsi => F64Min_Rsi,
        f64_min_rir => F64Min_Rir,
        f64_min_ris => F64Min_Ris,
        f64_max_rrs => F64Max_Rrs,
        f64_max_rri => F64Max_Rri,
        f64_max_rsr => F64Max_Rsr,
        f64_max_rss => F64Max_Rss,
        f64_max_rsi => F64Max_Rsi,
        f64_max_rir => F64Max_Rir,
        f64_max_ris => F64Max_Ris,
        f32_copysign_rrs => F32Copysign_Rrs,
        f32_copysign_rsr => F32Copysign_Rsr,
        f32_copysign_rss => F32Copysign_Rss,
        f32_copysign_rir => F32Copysign_Rir,
        f32_copysign_ris => F32Copysign_Ris,
        f64_copysign_rrs => F64Copysign_Rrs,
        f64_copysign_rsr => F64Copysign_Rsr,
        f64_copysign_rss => F64Copysign_Rss,
        f64_copysign_rir => F64Copysign_Rir,
        f64_copysign_ris => F64Copysign_Ris,
        i32_mul_rrr => I32Mul_Rrr,
        i64_mul_rrr => I64Mul_Rrr,
        f32_mul_rrr => F32Mul_Rrr,
        f64_mul_rrr => F64Mul_Rrr,
        branch_i32_eq_rs => BranchI32Eq_Rs,
        branch_i32_eq_ri => BranchI32Eq_Ri,
        branch_i32_eq_ss => BranchI32Eq_Ss,
        branch_i32_eq_si => BranchI32Eq_Si,
        branch_i32_not_eq_rs => BranchI32NotEq_Rs,
        branch_i32_not_eq_ri => BranchI32NotEq_Ri,
        branch_i32_not_eq_ss => BranchI32NotEq_Ss,
        branch_i32_not_eq_si => BranchI32NotEq_Si,
        branch_i32_and_rs => BranchI32And_Rs,
        branch_i32_and_ri => BranchI32And_Ri,
        branch_i32_and_ss => BranchI32And_Ss,
        branch_i32_and_si => BranchI32And_Si,
        branch_i32_not_and_rs => BranchI32NotAnd_Rs,
        branch_i32_not_and_ri => BranchI32NotAnd_Ri,
        branch_i32_not_and_ss => BranchI32NotAnd_Ss,
        branch_i32_not_and_si => BranchI32NotAnd_Si,
        branch_i32_or_rs => BranchI32Or_Rs,
        branch_i32_or_ri => BranchI32Or_Ri,
        branch_i32_or_ss => BranchI32Or_Ss,
        branch_i32_or_si => BranchI32Or_Si,
        branch_i32_not_or_rs => BranchI32NotOr_Rs,
        branch_i32_not_or_ri => BranchI32NotOr_Ri,
        branch_i32_not_or_ss => BranchI32NotOr_Ss,
        branch_i32_not_or_si => BranchI32NotOr_Si,
        branch_i32_lt_rs => BranchI32Lt_Rs,
        branch_i32_lt_ri => BranchI32Lt_Ri,
        branch_i32_lt_sr => BranchI32Lt_Sr,
        branch_i32_lt_ss => BranchI32Lt_Ss,
        branch_i32_lt_si => BranchI32Lt_Si,
        branch_i32_lt_ir => BranchI32Lt_Ir,
        branch_i32_lt_is => BranchI32Lt_Is,
        branch_i32_le_rs => BranchI32Le_Rs,
        branch_i32_le_ri => BranchI32Le_Ri,
        branch_i32_le_sr => BranchI32Le_Sr,
        branch_i32_le_ss => BranchI32Le_Ss,
        branch_i32_le_si => BranchI32Le_Si,
        branch_i32_le_ir => BranchI32Le_Ir,
        branch_i32_le_is => BranchI32Le_Is,
        branch_u32_lt_rs => BranchU32Lt_Rs,
        branch_u32_lt_ri => BranchU32Lt_Ri,
        branch_u32_lt_sr => BranchU32Lt_Sr,
        branch_u32_lt_ss => BranchU32Lt_Ss,
        branch_u32_lt_si => BranchU32Lt_Si,
        branch_u32_lt_ir => BranchU32Lt_Ir,
        branch_u32_lt_is => BranchU32Lt_Is,
        branch_u32_le_rs => BranchU32Le_Rs,
        branch_u32_le_ri => BranchU32Le_Ri,
        branch_u32_le_sr => BranchU32Le_Sr,
        branch_u32_le_ss => BranchU32Le_Ss,
        branch_u32_le_si => BranchU32Le_Si,
        branch_u32_le_ir => BranchU32Le_Ir,
        branch_u32_le_is => BranchU32Le_Is,
        branch_i64_eq_rs => BranchI64Eq_Rs,
        branch_i64_eq_ri => BranchI64Eq_Ri,
        branch_i64_eq_ss => BranchI64Eq_Ss,
        branch_i64_eq_si => BranchI64Eq_Si,
        branch_i64_not_eq_rs => BranchI64NotEq_Rs,
        branch_i64_not_eq_ri => BranchI64NotEq_Ri,
        branch_i64_not_eq_ss => BranchI64NotEq_Ss,
        branch_i64_not_eq_si => BranchI64NotEq_Si,
        branch_i64_and_rs => BranchI64And_Rs,
        branch_i64_and_ri => BranchI64And_Ri,
        branch_i64_and_ss => BranchI64And_Ss,
        branch_i64_and_si => BranchI64And_Si,
        branch_i64_not_and_rs => BranchI64NotAnd_Rs,
        branch_i64_not_and_ri => BranchI64NotAnd_Ri,
        branch_i64_not_and_ss => BranchI64NotAnd_Ss,
        branch_i64_not_and_si => BranchI64NotAnd_Si,
        branch_i64_or_rs => BranchI64Or_Rs,
        branch_i64_or_ri => BranchI64Or_Ri,
        branch_i64_or_ss => BranchI64Or_Ss,
        branch_i64_or_si => BranchI64Or_Si,
        branch_i64_not_or_rs => BranchI64NotOr_Rs,
        branch_i64_not_or_ri => BranchI64NotOr_Ri,
        branch_i64_not_or_ss => BranchI64NotOr_Ss,
        branch_i64_not_or_si => BranchI64NotOr_Si,
        branch_i64_lt_rs => BranchI64Lt_Rs,
        branch_i64_lt_ri => BranchI64Lt_Ri,
        branch_i64_lt_sr => BranchI64Lt_Sr,
        branch_i64_lt_ss => BranchI64Lt_Ss,
        branch_i64_lt_si => BranchI64Lt_Si,
        branch_i64_lt_ir => BranchI64Lt_Ir,
        branch_i64_lt_is => BranchI64Lt_Is,
        branch_i64_le_rs => BranchI64Le_Rs,
        branch_i64_le_ri => BranchI64Le_Ri,
        branch_i64_le_sr => BranchI64Le_Sr,
        branch_i64_le_ss => BranchI64Le_Ss,
        branch_i64_le_si => BranchI64Le_Si,
        branch_i64_le_ir => BranchI64Le_Ir,
        branch_i64_le_is => BranchI64Le_Is,
        branch_u64_lt_rs => BranchU64Lt_Rs,
        branch_u64_lt_ri => BranchU64Lt_Ri,
        branch_u64_lt_sr => BranchU64Lt_Sr,
        branch_u64_lt_ss => BranchU64Lt_Ss,
        branch_u64_lt_si => BranchU64Lt_Si,
        branch_u64_lt_ir => BranchU64Lt_Ir,
        branch_u64_lt_is => BranchU64Lt_Is,
        branch_u64_le_rs => BranchU64Le_Rs,
        branch_u64_le_ri => BranchU64Le_Ri,
        branch_u64_le_sr => BranchU64Le_Sr,
        branch_u64_le_ss => BranchU64Le_Ss,
        branch_u64_le_si => BranchU64Le_Si,
        branch_u64_le_ir => BranchU64Le_Ir,
        branch_u64_le_is => BranchU64Le_Is,
        branch_f32_eq_rs => BranchF32Eq_Rs,
        branch_f32_eq_ri => BranchF32Eq_Ri,
        branch_f32_eq_ss => BranchF32Eq_Ss,
        branch_f32_eq_si => BranchF32Eq_Si,
        branch_f32_not_eq_rs => BranchF32NotEq_Rs,
        branch_f32_not_eq_ri => BranchF32NotEq_Ri,
        branch_f32_not_eq_ss => BranchF32NotEq_Ss,
        branch_f32_not_eq_si => BranchF32NotEq_Si,
        branch_f32_lt_rs => BranchF32Lt_Rs,
        branch_f32_lt_ri => BranchF32Lt_Ri,
        branch_f32_lt_sr => BranchF32Lt_Sr,
        branch_f32_lt_ss => BranchF32Lt_Ss,
        branch_f32_lt_si => BranchF32Lt_Si,
        branch_f32_lt_ir => BranchF32Lt_Ir,
        branch_f32_lt_is => BranchF32Lt_Is,
        branch_f32_not_lt_rs => BranchF32NotLt_Rs,
        branch_f32_not_lt_ri => BranchF32NotLt_Ri,
        branch_f32_not_lt_sr => BranchF32NotLt_Sr,
        branch_f32_not_lt_ss => BranchF32NotLt_Ss,
        branch_f32_not_lt_si => BranchF32NotLt_Si,
        branch_f32_not_lt_ir => BranchF32NotLt_Ir,
        branch_f32_not_lt_is => BranchF32NotLt_Is,
        branch_f32_le_rs => BranchF32Le_Rs,
        branch_f32_le_ri => BranchF32Le_Ri,
        branch_f32_le_sr => BranchF32Le_Sr,
        branch_f32_le_ss => BranchF32Le_Ss,
        branch_f32_le_si => BranchF32Le_Si,
        branch_f32_le_ir => BranchF32Le_Ir,
        branch_f32_le_is => BranchF32Le_Is,
        branch_f32_not_le_rs => BranchF32NotLe_Rs,
        branch_f32_not_le_ri => BranchF32NotLe_Ri,
        branch_f32_not_le_sr => BranchF32NotLe_Sr,
        branch_f32_not_le_ss => BranchF32NotLe_Ss,
        branch_f32_not_le_si => BranchF32NotLe_Si,
        branch_f32_not_le_ir => BranchF32NotLe_Ir,
        branch_f32_not_le_is => BranchF32NotLe_Is,
        branch_f64_eq_rs => BranchF64Eq_Rs,
        branch_f64_eq_ri => BranchF64Eq_Ri,
        branch_f64_eq_ss => BranchF64Eq_Ss,
        branch_f64_eq_si => BranchF64Eq_Si,
        branch_f64_not_eq_rs => BranchF64NotEq_Rs,
        branch_f64_not_eq_ri => BranchF64NotEq_Ri,
        branch_f64_not_eq_ss => BranchF64NotEq_Ss,
        branch_f64_not_eq_si => BranchF64NotEq_Si,
        branch_f64_lt_rs => BranchF64Lt_Rs,
        branch_f64_lt_ri => BranchF64Lt_Ri,
        branch_f64_lt_sr => BranchF64Lt_Sr,
        branch_f64_lt_ss => BranchF64Lt_Ss,
        branch_f64_lt_si => BranchF64Lt_Si,
        branch_f64_lt_ir => BranchF64Lt_Ir,
        branch_f64_lt_is => BranchF64Lt_Is,
        branch_f64_not_lt_rs => BranchF64NotLt_Rs,
        branch_f64_not_lt_ri => BranchF64NotLt_Ri,
        branch_f64_not_lt_sr => BranchF64NotLt_Sr,
        branch_f64_not_lt_ss => BranchF64NotLt_Ss,
        branch_f64_not_lt_si => BranchF64NotLt_Si,
        branch_f64_not_lt_ir => BranchF64NotLt_Ir,
        branch_f64_not_lt_is => BranchF64NotLt_Is,
        branch_f64_le_rs => BranchF64Le_Rs,
        branch_f64_le_ri => BranchF64Le_Ri,
        branch_f64_le_sr => BranchF64Le_Sr,
        branch_f64_le_ss => BranchF64Le_Ss,
        branch_f64_le_si => BranchF64Le_Si,
        branch_f64_le_ir => BranchF64Le_Ir,
        branch_f64_le_is => BranchF64Le_Is,
        branch_f64_not_le_rs => BranchF64NotLe_Rs,
        branch_f64_not_le_ri => BranchF64NotLe_Ri,
        branch_f64_not_le_sr => BranchF64NotLe_Sr,
        branch_f64_not_le_ss => BranchF64NotLe_Ss,
        branch_f64_not_le_si => BranchF64NotLe_Si,
        branch_f64_not_le_ir => BranchF64NotLe_Ir,
        branch_f64_not_le_is => BranchF64NotLe_Is,
        u32_select_rrri => U32Select_Rrri,
        u32_select_rrsi => U32Select_Rrsi,
        u32_select_rrir => U32Select_Rrir,
        u32_select_rris => U32Select_Rris,
        u32_select_rrii => U32Select_Rrii,
        u32_select_rsri => U32Select_Rsri,
        u32_select_rssi => U32Select_Rssi,
        u32_select_rsir => U32Select_Rsir,
        u32_select_rsis => U32Select_Rsis,
        u32_select_rsii => U32Select_Rsii,
        u64_select_rrrs => U64Select_Rrrs,
        u64_select_rrri => U64Select_Rrri,
        u64_select_rrsr => U64Select_Rrsr,
        u64_select_rrss => U64Select_Rrss,
        u64_select_rrsi => U64Select_Rrsi,
        u64_select_rrir => U64Select_Rrir,
        u64_select_rris => U64Select_Rris,
        u64_select_rrii => U64Select_Rrii,
        u64_select_rsrs => U64Select_Rsrs,
        u64_select_rsri => U64Select_Rsri,
        u64_select_rssr => U64Select_Rssr,
        u64_select_rsss => U64Select_Rsss,
        u64_select_rssi => U64Select_Rssi,
        u64_select_rsir => U64Select_Rsir,
        u64_select_rsis => U64Select_Rsis,
        u64_select_rsii => U64Select_Rsii,
        f32_select_rrrs => F32Select_Rrrs,
        f32_select_rrri => F32Select_Rrri,
        f32_select_rrsr => F32Select_Rrsr,
        f32_select_rrss => F32Select_Rrss,
        f32_select_rrsi => F32Select_Rrsi,
        f32_select_rrir => F32Select_Rrir,
        f32_select_rris => F32Select_Rris,
        f32_select_rrii => F32Select_Rrii,
        f32_select_rsrs => F32Select_Rsrs,
        f32_select_rsri => F32Select_Rsri,
        f32_select_rssr => F32Select_Rssr,
        f32_select_rsss => F32Select_Rsss,
        f32_select_rssi => F32Select_Rssi,
        f32_select_rsir => F32Select_Rsir,
        f32_select_rsis => F32Select_Rsis,
        f32_select_rsii => F32Select_Rsii,
        f64_select_rrrs => F64Select_Rrrs,
        f64_select_rrri => F64Select_Rrri,
        f64_select_rrsr => F64Select_Rrsr,
        f64_select_rrss => F64Select_Rrss,
        f64_select_rrsi => F64Select_Rrsi,
        f64_select_rrir => F64Select_Rrir,
        f64_select_rris => F64Select_Rris,
        f64_select_rrii => F64Select_Rrii,
        f64_select_rsrs => F64Select_Rsrs,
        f64_select_rsri => F64Select_Rsri,
        f64_select_rssr => F64Select_Rssr,
        f64_select_rsss => F64Select_Rsss,
        f64_select_rssi => F64Select_Rssi,
        f64_select_rsir => F64Select_Rsir,
        f64_select_rsis => F64Select_Rsis,
        f64_select_rsii => F64Select_Rsii,
        u32_load_rr => U32Load_Rr,
        u32_load_mem0_offset16_rr => U32LoadMem0Offset16_Rr,
        u32_load_mem0_offset16_rs_r => U32LoadMem0Offset16_Rs_r,
        u32_load_rs => U32Load_Rs,
        u32_load_mem0_offset16_rs => U32LoadMem0Offset16_Rs,
        u32_load_mem0_offset16_rs_s => U32LoadMem0Offset16_Rs_s,
        u32_load_ri => U32Load_Ri,
        u64_load_rr => U64Load_Rr,
        u64_load_mem0_offset16_rr => U64LoadMem0Offset16_Rr,
        u64_load_mem0_offset16_rs_r => U64LoadMem0Offset16_Rs_r,
        u64_load_rs => U64Load_Rs,
        u64_load_mem0_offset16_rs => U64LoadMem0Offset16_Rs,
        u64_load_mem0_offset16_rs_s => U64LoadMem0Offset16_Rs_s,
        u64_load_ri => U64Load_Ri,
        f32_load_rr => F32Load_Rr,
        f32_load_mem0_offset16_rr => F32LoadMem0Offset16_Rr,
        f32_load_mem0_offset16_rs_r => F32LoadMem0Offset16_Rs_r,
        f32_load_rs => F32Load_Rs,
        f32_load_mem0_offset16_rs => F32LoadMem0Offset16_Rs,
        f32_load_mem0_offset16_rs_s => F32LoadMem0Offset16_Rs_s,
        f32_load_ri => F32Load_Ri,
        f64_load_rr => F64Load_Rr,
        f64_load_mem0_offset16_rr => F64LoadMem0Offset16_Rr,
        f64_load_mem0_offset16_rs_r => F64LoadMem0Offset16_Rs_r,
        f64_load_rs => F64Load_Rs,
        f64_load_mem0_offset16_rs => F64LoadMem0Offset16_Rs,
        f64_load_mem0_offset16_rs_s => F64LoadMem0Offset16_Rs_s,
        f64_load_ri => F64Load_Ri,
        i32_load_extend8_rr => I32LoadExtend8_Rr,
        i32_load_extend8_mem0_offset16_rr => I32LoadExtend8Mem0Offset16_Rr,
        i32_load_extend8_mem0_offset16_rs_r => I32LoadExtend8Mem0Offset16_Rs_r,
        i32_load_extend8_rs => I32LoadExtend8_Rs,
        i32_load_extend8_mem0_offset16_rs => I32LoadExtend8Mem0Offset16_Rs,
        i32_load_extend8_mem0_offset16_rs_s => I32LoadExtend8Mem0Offset16_Rs_s,
        i32_load_extend8_ri => I32LoadExtend8_Ri,
        i32_load_extend16_rr => I32LoadExtend16_Rr,
        i32_load_extend16_mem0_offset16_rr => I32LoadExtend16Mem0Offset16_Rr,
        i32_load_extend16_mem0_offset16_rs_r => I32LoadExtend16Mem0Offset16_Rs_r,
        i32_load_extend16_rs => I32LoadExtend16_Rs,
        i32_load_extend16_mem0_offset16_rs => I32LoadExtend16Mem0Offset16_Rs,
        i32_load_extend16_mem0_offset16_rs_s => I32LoadExtend16Mem0Offset16_Rs_s,
        i32_load_extend16_ri => I32LoadExtend16_Ri,
        u32_load_extend8_rr => U32LoadExtend8_Rr,
        u32_load_extend8_mem0_offset16_rr => U32LoadExtend8Mem0Offset16_Rr,
        u32_load_extend8_mem0_offset16_rs_r => U32LoadExtend8Mem0Offset16_Rs_r,
        u32_load_extend8_rs => U32LoadExtend8_Rs,
        u32_load_extend8_mem0_offset16_rs => U32LoadExtend8Mem0Offset16_Rs,
        u32_load_extend8_mem0_offset16_rs_s => U32LoadExtend8Mem0Offset16_Rs_s,
        u32_load_extend8_ri => U32LoadExtend8_Ri,
        u32_load_extend16_rr => U32LoadExtend16_Rr,
        u32_load_extend16_mem0_offset16_rr => U32LoadExtend16Mem0Offset16_Rr,
        u32_load_extend16_mem0_offset16_rs_r => U32LoadExtend16Mem0Offset16_Rs_r,
        u32_load_extend16_rs => U32LoadExtend16_Rs,
        u32_load_extend16_mem0_offset16_rs => U32LoadExtend16Mem0Offset16_Rs,
        u32_load_extend16_mem0_offset16_rs_s => U32LoadExtend16Mem0Offset16_Rs_s,
        u32_load_extend16_ri => U32LoadExtend16_Ri,
        i64_load_extend8_rr => I64LoadExtend8_Rr,
        i64_load_extend8_mem0_offset16_rr => I64LoadExtend8Mem0Offset16_Rr,
        i64_load_extend8_mem0_offset16_rs_r => I64LoadExtend8Mem0Offset16_Rs_r,
        i64_load_extend8_rs => I64LoadExtend8_Rs,
        i64_load_extend8_mem0_offset16_rs => I64LoadExtend8Mem0Offset16_Rs,
        i64_load_extend8_mem0_offset16_rs_s => I64LoadExtend8Mem0Offset16_Rs_s,
        i64_load_extend8_ri => I64LoadExtend8_Ri,
        i64_load_extend16_rr => I64LoadExtend16_Rr,
        i64_load_extend16_mem0_offset16_rr => I64LoadExtend16Mem0Offset16_Rr,
        i64_load_extend16_mem0_offset16_rs_r => I64LoadExtend16Mem0Offset16_Rs_r,
        i64_load_extend16_rs => I64LoadExtend16_Rs,
        i64_load_extend16_mem0_offset16_rs => I64LoadExtend16Mem0Offset16_Rs,
        i64_load_extend16_mem0_offset16_rs_s => I64LoadExtend16Mem0Offset16_Rs_s,
        i64_load_extend16_ri => I64LoadExtend16_Ri,
        i64_load_extend32_rr => I64LoadExtend32_Rr,
        i64_load_extend32_mem0_offset16_rr => I64LoadExtend32Mem0Offset16_Rr,
        i64_load_extend32_mem0_offset16_rs_r => I64LoadExtend32Mem0Offset16_Rs_r,
        i64_load_extend32_rs => I64LoadExtend32_Rs,
        i64_load_extend32_mem0_offset16_rs => I64LoadExtend32Mem0Offset16_Rs,
        i64_load_extend32_mem0_offset16_rs_s => I64LoadExtend32Mem0Offset16_Rs_s,
        i64_load_extend32_ri => I64LoadExtend32_Ri,
        u64_load_extend8_rr => U64LoadExtend8_Rr,
        u64_load_extend8_mem0_offset16_rr => U64LoadExtend8Mem0Offset16_Rr,
        u64_load_extend8_mem0_offset16_rs_r => U64LoadExtend8Mem0Offset16_Rs_r,
        u64_load_extend8_rs => U64LoadExtend8_Rs,
        u64_load_extend8_mem0_offset16_rs => U64LoadExtend8Mem0Offset16_Rs,
        u64_load_extend8_mem0_offset16_rs_s => U64LoadExtend8Mem0Offset16_Rs_s,
        u64_load_extend8_ri => U64LoadExtend8_Ri,
        u64_load_extend16_rr => U64LoadExtend16_Rr,
        u64_load_extend16_mem0_offset16_rr => U64LoadExtend16Mem0Offset16_Rr,
        u64_load_extend16_mem0_offset16_rs_r => U64LoadExtend16Mem0Offset16_Rs_r,
        u64_load_extend16_rs => U64LoadExtend16_Rs,
        u64_load_extend16_mem0_offset16_rs => U64LoadExtend16Mem0Offset16_Rs,
        u64_load_extend16_mem0_offset16_rs_s => U64LoadExtend16Mem0Offset16_Rs_s,
        u64_load_extend16_ri => U64LoadExtend16_Ri,
        u64_load_extend32_rr => U64LoadExtend32_Rr,
        u64_load_extend32_mem0_offset16_rr => U64LoadExtend32Mem0Offset16_Rr,
        u64_load_extend32_mem0_offset16_rs_r => U64LoadExtend32Mem0Offset16_Rs_r,
        u64_load_extend32_rs => U64LoadExtend32_Rs,
        u64_load_extend32_mem0_offset16_rs => U64LoadExtend32Mem0Offset16_Rs,
        u64_load_extend32_mem0_offset16_rs_s => U64LoadExtend32Mem0Offset16_Rs_s,
        u64_load_extend32_ri => U64LoadExtend32_Ri,
        u32_store_rs => U32Store_Rs,
        u32_store_mem0_offset16_rs => U32StoreMem0Offset16_Rs,
        u32_store_ri => U32Store_Ri,
        u32_store_mem0_offset16_ri => U32StoreMem0Offset16_Ri,
        u32_store_sr => U32Store_Sr,
        u32_store_mem0_offset16_sr => U32StoreMem0Offset16_Sr,
        u32_store_ss => U32Store_Ss,
        u32_store_mem0_offset16_ss => U32StoreMem0Offset16_Ss,
        u32_store_si => U32Store_Si,
        u32_store_mem0_offset16_si => U32StoreMem0Offset16_Si,
        u32_store_ir => U32Store_Ir,
        u32_store_is => U32Store_Is,
        u32_store_ii => U32Store_Ii,
        u64_store_rs => U64Store_Rs,
        u64_store_mem0_offset16_rs => U64StoreMem0Offset16_Rs,
        u64_store_ri => U64Store_Ri,
        u64_store_mem0_offset16_ri => U64StoreMem0Offset16_Ri,
        u64_store_sr => U64Store_Sr,
        u64_store_mem0_offset16_sr => U64StoreMem0Offset16_Sr,
        u64_store_ss => U64Store_Ss,
        u64_store_mem0_offset16_ss => U64StoreMem0Offset16_Ss,
        u64_store_si => U64Store_Si,
        u64_store_mem0_offset16_si => U64StoreMem0Offset16_Si,
        u64_store_ir => U64Store_Ir,
        u64_store_is => U64Store_Is,
        u64_store_ii => U64Store_Ii,
        f32_store_rr => F32Store_Rr,
        f32_store_mem0_offset16_rr => F32StoreMem0Offset16_Rr,
        f32_store_sr => F32Store_Sr,
        f32_store_mem0_offset16_sr => F32StoreMem0Offset16_Sr,
        f32_store_ir => F32Store_Ir,
        f64_store_rr => F64Store_Rr,
        f64_store_mem0_offset16_rr => F64StoreMem0Offset16_Rr,
        f64_store_sr => F64Store_Sr,
        f64_store_mem0_offset16_sr => F64StoreMem0Offset16_Sr,
        f64_store_ir => F64Store_Ir,
        i32_store_wrap8_rs => I32StoreWrap8_Rs,
        i32_store_wrap8_mem0_offset16_rs => I32StoreWrap8Mem0Offset16_Rs,
        i32_store_wrap8_ri => I32StoreWrap8_Ri,
        i32_store_wrap8_mem0_offset16_ri => I32StoreWrap8Mem0Offset16_Ri,
        i32_store_wrap8_sr => I32StoreWrap8_Sr,
        i32_store_wrap8_mem0_offset16_sr => I32StoreWrap8Mem0Offset16_Sr,
        i32_store_wrap8_ss => I32StoreWrap8_Ss,
        i32_store_wrap8_mem0_offset16_ss => I32StoreWrap8Mem0Offset16_Ss,
        i32_store_wrap8_si => I32StoreWrap8_Si,
        i32_store_wrap8_mem0_offset16_si => I32StoreWrap8Mem0Offset16_Si,
        i32_store_wrap8_ir => I32StoreWrap8_Ir,
        i32_store_wrap8_is => I32StoreWrap8_Is,
        i32_store_wrap8_ii => I32StoreWrap8_Ii,
        i32_store_wrap16_rs => I32StoreWrap16_Rs,
        i32_store_wrap16_mem0_offset16_rs => I32StoreWrap16Mem0Offset16_Rs,
        i32_store_wrap16_ri => I32StoreWrap16_Ri,
        i32_store_wrap16_mem0_offset16_ri => I32StoreWrap16Mem0Offset16_Ri,
        i32_store_wrap16_sr => I32StoreWrap16_Sr,
        i32_store_wrap16_mem0_offset16_sr => I32StoreWrap16Mem0Offset16_Sr,
        i32_store_wrap16_ss => I32StoreWrap16_Ss,
        i32_store_wrap16_mem0_offset16_ss => I32StoreWrap16Mem0Offset16_Ss,
        i32_store_wrap16_si => I32StoreWrap16_Si,
        i32_store_wrap16_mem0_offset16_si => I32StoreWrap16Mem0Offset16_Si,
        i32_store_wrap16_ir => I32StoreWrap16_Ir,
        i32_store_wrap16_is => I32StoreWrap16_Is,
        i32_store_wrap16_ii => I32StoreWrap16_Ii,
        i64_store_wrap8_rs => I64StoreWrap8_Rs,
        i64_store_wrap8_mem0_offset16_rs => I64StoreWrap8Mem0Offset16_Rs,
        i64_store_wrap8_ri => I64StoreWrap8_Ri,
        i64_store_wrap8_mem0_offset16_ri => I64StoreWrap8Mem0Offset16_Ri,
        i64_store_wrap8_sr => I64StoreWrap8_Sr,
        i64_store_wrap8_mem0_offset16_sr => I64StoreWrap8Mem0Offset16_Sr,
        i64_store_wrap8_ss => I64StoreWrap8_Ss,
        i64_store_wrap8_mem0_offset16_ss => I64StoreWrap8Mem0Offset16_Ss,
        i64_store_wrap8_si => I64StoreWrap8_Si,
        i64_store_wrap8_mem0_offset16_si => I64StoreWrap8Mem0Offset16_Si,
        i64_store_wrap8_ir => I64StoreWrap8_Ir,
        i64_store_wrap8_is => I64StoreWrap8_Is,
        i64_store_wrap8_ii => I64StoreWrap8_Ii,
        i64_store_wrap16_rs => I64StoreWrap16_Rs,
        i64_store_wrap16_mem0_offset16_rs => I64StoreWrap16Mem0Offset16_Rs,
        i64_store_wrap16_ri => I64StoreWrap16_Ri,
        i64_store_wrap16_mem0_offset16_ri => I64StoreWrap16Mem0Offset16_Ri,
        i64_store_wrap16_sr => I64StoreWrap16_Sr,
        i64_store_wrap16_mem0_offset16_sr => I64StoreWrap16Mem0Offset16_Sr,
        i64_store_wrap16_ss => I64StoreWrap16_Ss,
        i64_store_wrap16_mem0_offset16_ss => I64StoreWrap16Mem0Offset16_Ss,
        i64_store_wrap16_si => I64StoreWrap16_Si,
        i64_store_wrap16_mem0_offset16_si => I64StoreWrap16Mem0Offset16_Si,
        i64_store_wrap16_ir => I64StoreWrap16_Ir,
        i64_store_wrap16_is => I64StoreWrap16_Is,
        i64_store_wrap16_ii => I64StoreWrap16_Ii,
        i64_store_wrap32_rs => I64StoreWrap32_Rs,
        i64_store_wrap32_mem0_offset16_rs => I64StoreWrap32Mem0Offset16_Rs,
        i64_store_wrap32_ri => I64StoreWrap32_Ri,
        i64_store_wrap32_mem0_offset16_ri => I64StoreWrap32Mem0Offset16_Ri,
        i64_store_wrap32_sr => I64StoreWrap32_Sr,
        i64_store_wrap32_mem0_offset16_sr => I64StoreWrap32Mem0Offset16_Sr,
        i64_store_wrap32_ss => I64StoreWrap32_Ss,
        i64_store_wrap32_mem0_offset16_ss => I64StoreWrap32Mem0Offset16_Ss,
        i64_store_wrap32_si => I64StoreWrap32_Si,
        i64_store_wrap32_mem0_offset16_si => I64StoreWrap32Mem0Offset16_Si,
        i64_store_wrap32_ir => I64StoreWrap32_Ir,
        i64_store_wrap32_is => I64StoreWrap32_Is,
        i64_store_wrap32_ii => I64StoreWrap32_Ii,
        r#return => Return,
        trap => Trap,
        consume_fuel => ConsumeFuel,
        branch => Branch,
        branch_table_r => BranchTable_R,
        branch_table_s => BranchTable_S,
        branch_table_span_r => BranchTableSpan_R,
        branch_table_span_s => BranchTableSpan_S,
        u32_copy_ri => U32Copy_Ri,
        u32_copy_si => U32Copy_Si,
        u64_copy_rs => U64Copy_Rs,
        u64_copy_ri => U64Copy_Ri,
        u64_copy_sr => U64Copy_Sr,
        u64_copy_ss => U64Copy_Ss,
        u64_copy_si => U64Copy_Si,
        f32_copy_ri => F32Copy_Ri,
        f32_copy_rs => F32Copy_Rs,
        f32_copy_sr => F32Copy_Sr,
        f64_copy_rs => F64Copy_Rs,
        f64_copy_ri => F64Copy_Ri,
        f64_copy_sr => F64Copy_Sr,
        u64_copy_s0s1 => U64Copy_S0s1,
        u64_copy_s0s2 => U64Copy_S0s2,
        u64_copy_s0s3 => U64Copy_S0s3,
        u64_copy_s0s4 => U64Copy_S0s4,
        u64_copy_s0s5 => U64Copy_S0s5,
        u64_copy_s1s0 => U64Copy_S1s0,
        u64_copy_s1s2 => U64Copy_S1s2,
        u64_copy_s1s3 => U64Copy_S1s3,
        u64_copy_s1s4 => U64Copy_S1s4,
        u64_copy_s1s5 => U64Copy_S1s5,
        u64_copy_s2s0 => U64Copy_S2s0,
        u64_copy_s2s1 => U64Copy_S2s1,
        u64_copy_s2s3 => U64Copy_S2s3,
        u64_copy_s2s4 => U64Copy_S2s4,
        u64_copy_s2s5 => U64Copy_S2s5,
        u64_copy_s3s0 => U64Copy_S3s0,
        u64_copy_s3s1 => U64Copy_S3s1,
        u64_copy_s3s2 => U64Copy_S3s2,
        u64_copy_s3s4 => U64Copy_S3s4,
        u64_copy_s3s5 => U64Copy_S3s5,
        u64_copy_s4s0 => U64Copy_S4s0,
        u64_copy_s4s1 => U64Copy_S4s1,
        u64_copy_s4s2 => U64Copy_S4s2,
        u64_copy_s4s3 => U64Copy_S4s3,
        u64_copy_s4s5 => U64Copy_S4s5,
        u64_copy_s5s0 => U64Copy_S5s0,
        u64_copy_s5s1 => U64Copy_S5s1,
        u64_copy_s5s2 => U64Copy_S5s2,
        u64_copy_s5s3 => U64Copy_S5s3,
        u64_copy_s5s4 => U64Copy_S5s4,
        f32_reinterpret_i32_rr => F32ReinterpretI32_Rr,
        i32_reinterpret_f32_rr => I32ReinterpretF32_Rr,
        f64_reinterpret_i64_rr => F64ReinterpretI64_Rr,
        i64_reinterpret_f64_rr => I64ReinterpretF64_Rr,
        u64_copy_s0r => U64Copy_S0r,
        u64_copy_s1r => U64Copy_S1r,
        u64_copy_s2r => U64Copy_S2r,
        u64_copy_s3r => U64Copy_S3r,
        u64_copy_s4r => U64Copy_S4r,
        u64_copy_s5r => U64Copy_S5r,
        u64_copy_s6r => U64Copy_S6r,
        u64_copy_s7r => U64Copy_S7r,
        u64_copy_s8r => U64Copy_S8r,
        u64_copy_s9r => U64Copy_S9r,
        f32_copy_s0r => F32Copy_S0r,
        f32_copy_s1r => F32Copy_S1r,
        f32_copy_s2r => F32Copy_S2r,
        f32_copy_s3r => F32Copy_S3r,
        f32_copy_s4r => F32Copy_S4r,
        f32_copy_s5r => F32Copy_S5r,
        f32_copy_s6r => F32Copy_S6r,
        f32_copy_s7r => F32Copy_S7r,
        f32_copy_s8r => F32Copy_S8r,
        f32_copy_s9r => F32Copy_S9r,
        f64_copy_s0r => F64Copy_S0r,
        f64_copy_s1r => F64Copy_S1r,
        f64_copy_s2r => F64Copy_S2r,
        f64_copy_s3r => F64Copy_S3r,
        f64_copy_s4r => F64Copy_S4r,
        f64_copy_s5r => F64Copy_S5r,
        f64_copy_s6r => F64Copy_S6r,
        f64_copy_s7r => F64Copy_S7r,
        f64_copy_s8r => F64Copy_S8r,
        f64_copy_s9r => F64Copy_S9r,
        ref_func => RefFunc,
        call_internal => CallInternal,
        call_imported => CallImported,
        call_indirect_r => CallIndirect_R,
        call_indirect_s => CallIndirect_S,
        r#return_call_indirect_r => ReturnCallIndirect_R,
        r#return_call_indirect_s => ReturnCallIndirect_S,
        return_call_internal => ReturnCallInternal,
        return_call_imported => ReturnCallImported,
        global_get_u64_r => GlobalGetU64_R,
        global_get_f32_r => GlobalGetF32_R,
        global_get_f64_r => GlobalGetF64_R,
        global_set_u32_i => GlobalSetU32_I,
        global_set_u64_r => GlobalSetU64_R,
        global_set_u64_s => GlobalSetU64_S,
        global_set_u64_i => GlobalSetU64_I,
        global_set_f32_r => GlobalSetF32_R,
        global_set_f64_r => GlobalSetF64_R,
        data_drop => DataDrop,
        memory_size => MemorySize,
        memory_grow => MemoryGrow,
        memory_copy => MemoryCopy,
        memory_fill => MemoryFill,
        memory_init => MemoryInit,
        table_get_rr => TableGet_Rr,
        table_set_rs => TableSet_Rs,
        table_set_ri => TableSet_Ri,
        table_get_rs => TableGet_Rs,
        table_set_sr => TableSet_Sr,
        table_set_ss => TableSet_Ss,
        table_set_si => TableSet_Si,
        table_get_ri => TableGet_Ri,
        table_set_ir => TableSet_Ir,
        table_set_is => TableSet_Is,
        table_set_ii => TableSet_Ii,
        table_size => TableSize,
        table_grow => TableGrow,
        table_copy => TableCopy,
        table_fill => TableFill,
        table_init => TableInit,
        elem_drop => ElemDrop,
        i64_add128 => I64Add128,
        i64_sub128 => I64Sub128,
        i64_mul_wide => I64MulWide,
        u64_mul_wide => U64MulWide,
        }
    };
}

impl Op {
    /// Returns the [`Location`] of the result of `self` if any.
    pub fn result_loc(&self) -> Option<Location> {
        let loc = match self {
            | Self::I32Clz_Rs { result, .. } => result.location(),
            | Self::I32Clz_Rr { result, .. } => result.location(),
            | Self::I32Ctz_Rs { result, .. } => result.location(),
            | Self::I32Ctz_Rr { result, .. } => result.location(),
            | Self::I32Popcnt_Rs { result, .. } => result.location(),
            | Self::I32Popcnt_Rr { result, .. } => result.location(),
            | Self::I32Sext8_Rs { result, .. } => result.location(),
            | Self::I32Sext8_Rr { result, .. } => result.location(),
            | Self::I32Sext16_Rs { result, .. } => result.location(),
            | Self::I32Sext16_Rr { result, .. } => result.location(),
            | Self::I32WrapI64_Rs { result, .. } => result.location(),
            | Self::I32WrapI64_Rr { result, .. } => result.location(),
            | Self::I64Clz_Rs { result, .. } => result.location(),
            | Self::I64Clz_Rr { result, .. } => result.location(),
            | Self::I64Ctz_Rs { result, .. } => result.location(),
            | Self::I64Ctz_Rr { result, .. } => result.location(),
            | Self::I64Popcnt_Rs { result, .. } => result.location(),
            | Self::I64Popcnt_Rr { result, .. } => result.location(),
            | Self::I64Sext8_Rs { result, .. } => result.location(),
            | Self::I64Sext8_Rr { result, .. } => result.location(),
            | Self::I64Sext16_Rs { result, .. } => result.location(),
            | Self::I64Sext16_Rr { result, .. } => result.location(),
            | Self::I64Sext32_Rs { result, .. } => result.location(),
            | Self::I64Sext32_Rr { result, .. } => result.location(),
            | Self::F32Abs_Rs { result, .. } => result.location(),
            | Self::F32Abs_Rr { result, .. } => result.location(),
            | Self::F32Neg_Rs { result, .. } => result.location(),
            | Self::F32Neg_Rr { result, .. } => result.location(),
            | Self::F32Nabs_Rs { result, .. } => result.location(),
            | Self::F32Nabs_Rr { result, .. } => result.location(),
            | Self::F32Ceil_Rs { result, .. } => result.location(),
            | Self::F32Ceil_Rr { result, .. } => result.location(),
            | Self::F32Floor_Rs { result, .. } => result.location(),
            | Self::F32Floor_Rr { result, .. } => result.location(),
            | Self::F32Trunc_Rs { result, .. } => result.location(),
            | Self::F32Trunc_Rr { result, .. } => result.location(),
            | Self::F32Nearest_Rs { result, .. } => result.location(),
            | Self::F32Nearest_Rr { result, .. } => result.location(),
            | Self::F32Sqrt_Rs { result, .. } => result.location(),
            | Self::F32Sqrt_Rr { result, .. } => result.location(),
            | Self::F32ConvertI32_Rs { result, .. } => result.location(),
            | Self::F32ConvertI32_Rr { result, .. } => result.location(),
            | Self::F32ConvertU32_Rs { result, .. } => result.location(),
            | Self::F32ConvertU32_Rr { result, .. } => result.location(),
            | Self::F32ConvertI64_Rs { result, .. } => result.location(),
            | Self::F32ConvertI64_Rr { result, .. } => result.location(),
            | Self::F32ConvertU64_Rs { result, .. } => result.location(),
            | Self::F32ConvertU64_Rr { result, .. } => result.location(),
            | Self::F32DemoteF64_Rs { result, .. } => result.location(),
            | Self::F32DemoteF64_Rr { result, .. } => result.location(),
            | Self::F64Abs_Rs { result, .. } => result.location(),
            | Self::F64Abs_Rr { result, .. } => result.location(),
            | Self::F64Neg_Rs { result, .. } => result.location(),
            | Self::F64Neg_Rr { result, .. } => result.location(),
            | Self::F64Nabs_Rs { result, .. } => result.location(),
            | Self::F64Nabs_Rr { result, .. } => result.location(),
            | Self::F64Ceil_Rs { result, .. } => result.location(),
            | Self::F64Ceil_Rr { result, .. } => result.location(),
            | Self::F64Floor_Rs { result, .. } => result.location(),
            | Self::F64Floor_Rr { result, .. } => result.location(),
            | Self::F64Trunc_Rs { result, .. } => result.location(),
            | Self::F64Trunc_Rr { result, .. } => result.location(),
            | Self::F64Nearest_Rs { result, .. } => result.location(),
            | Self::F64Nearest_Rr { result, .. } => result.location(),
            | Self::F64Sqrt_Rs { result, .. } => result.location(),
            | Self::F64Sqrt_Rr { result, .. } => result.location(),
            | Self::F64ConvertI32_Rs { result, .. } => result.location(),
            | Self::F64ConvertI32_Rr { result, .. } => result.location(),
            | Self::F64ConvertU32_Rs { result, .. } => result.location(),
            | Self::F64ConvertU32_Rr { result, .. } => result.location(),
            | Self::F64ConvertI64_Rs { result, .. } => result.location(),
            | Self::F64ConvertI64_Rr { result, .. } => result.location(),
            | Self::F64ConvertU64_Rs { result, .. } => result.location(),
            | Self::F64ConvertU64_Rr { result, .. } => result.location(),
            | Self::F64PromoteF32_Rs { result, .. } => result.location(),
            | Self::F64PromoteF32_Rr { result, .. } => result.location(),
            | Self::I32TruncF32_Rs { result, .. } => result.location(),
            | Self::I32TruncF32_Rr { result, .. } => result.location(),
            | Self::U32TruncF32_Rs { result, .. } => result.location(),
            | Self::U32TruncF32_Rr { result, .. } => result.location(),
            | Self::I32TruncF64_Rs { result, .. } => result.location(),
            | Self::I32TruncF64_Rr { result, .. } => result.location(),
            | Self::U32TruncF64_Rs { result, .. } => result.location(),
            | Self::U32TruncF64_Rr { result, .. } => result.location(),
            | Self::I64TruncF32_Rs { result, .. } => result.location(),
            | Self::I64TruncF32_Rr { result, .. } => result.location(),
            | Self::U64TruncF32_Rs { result, .. } => result.location(),
            | Self::U64TruncF32_Rr { result, .. } => result.location(),
            | Self::I64TruncF64_Rs { result, .. } => result.location(),
            | Self::I64TruncF64_Rr { result, .. } => result.location(),
            | Self::U64TruncF64_Rs { result, .. } => result.location(),
            | Self::U64TruncF64_Rr { result, .. } => result.location(),
            | Self::I32TruncSatF32_Rs { result, .. } => result.location(),
            | Self::I32TruncSatF32_Rr { result, .. } => result.location(),
            | Self::U32TruncSatF32_Rs { result, .. } => result.location(),
            | Self::U32TruncSatF32_Rr { result, .. } => result.location(),
            | Self::I32TruncSatF64_Rs { result, .. } => result.location(),
            | Self::I32TruncSatF64_Rr { result, .. } => result.location(),
            | Self::U32TruncSatF64_Rs { result, .. } => result.location(),
            | Self::U32TruncSatF64_Rr { result, .. } => result.location(),
            | Self::I64TruncSatF32_Rs { result, .. } => result.location(),
            | Self::I64TruncSatF32_Rr { result, .. } => result.location(),
            | Self::U64TruncSatF32_Rs { result, .. } => result.location(),
            | Self::U64TruncSatF32_Rr { result, .. } => result.location(),
            | Self::I64TruncSatF64_Rs { result, .. } => result.location(),
            | Self::I64TruncSatF64_Rr { result, .. } => result.location(),
            | Self::U64TruncSatF64_Rs { result, .. } => result.location(),
            | Self::U64TruncSatF64_Rr { result, .. } => result.location(),
            | Self::I32Eq_Rrs { result, .. } => result.location(),
            | Self::I32Eq_Rri { result, .. } => result.location(),
            | Self::I32Eq_Rss { result, .. } => result.location(),
            | Self::I32Eq_Rsi { result, .. } => result.location(),
            | Self::I32And_Rrs { result, .. } => result.location(),
            | Self::I32And_Rri { result, .. } => result.location(),
            | Self::I32And_Rss { result, .. } => result.location(),
            | Self::I32And_Rsi { result, .. } => result.location(),
            | Self::I32Or_Rrs { result, .. } => result.location(),
            | Self::I32Or_Rri { result, .. } => result.location(),
            | Self::I32Or_Rss { result, .. } => result.location(),
            | Self::I32Or_Rsi { result, .. } => result.location(),
            | Self::I32NotEq_Rrs { result, .. } => result.location(),
            | Self::I32NotEq_Rri { result, .. } => result.location(),
            | Self::I32NotEq_Rss { result, .. } => result.location(),
            | Self::I32NotEq_Rsi { result, .. } => result.location(),
            | Self::I32NotAnd_Rrs { result, .. } => result.location(),
            | Self::I32NotAnd_Rri { result, .. } => result.location(),
            | Self::I32NotAnd_Rss { result, .. } => result.location(),
            | Self::I32NotAnd_Rsi { result, .. } => result.location(),
            | Self::I32NotOr_Rrs { result, .. } => result.location(),
            | Self::I32NotOr_Rri { result, .. } => result.location(),
            | Self::I32NotOr_Rss { result, .. } => result.location(),
            | Self::I32NotOr_Rsi { result, .. } => result.location(),
            | Self::I32Lt_Rrs { result, .. } => result.location(),
            | Self::I32Lt_Rri { result, .. } => result.location(),
            | Self::I32Lt_Rsr { result, .. } => result.location(),
            | Self::I32Lt_Rss { result, .. } => result.location(),
            | Self::I32Lt_Rsi { result, .. } => result.location(),
            | Self::I32Lt_Rir { result, .. } => result.location(),
            | Self::I32Lt_Ris { result, .. } => result.location(),
            | Self::I32Le_Rrs { result, .. } => result.location(),
            | Self::I32Le_Rri { result, .. } => result.location(),
            | Self::I32Le_Rsr { result, .. } => result.location(),
            | Self::I32Le_Rss { result, .. } => result.location(),
            | Self::I32Le_Rsi { result, .. } => result.location(),
            | Self::I32Le_Rir { result, .. } => result.location(),
            | Self::I32Le_Ris { result, .. } => result.location(),
            | Self::U32Lt_Rrs { result, .. } => result.location(),
            | Self::U32Lt_Rri { result, .. } => result.location(),
            | Self::U32Lt_Rsr { result, .. } => result.location(),
            | Self::U32Lt_Rss { result, .. } => result.location(),
            | Self::U32Lt_Rsi { result, .. } => result.location(),
            | Self::U32Lt_Rir { result, .. } => result.location(),
            | Self::U32Lt_Ris { result, .. } => result.location(),
            | Self::U32Le_Rrs { result, .. } => result.location(),
            | Self::U32Le_Rri { result, .. } => result.location(),
            | Self::U32Le_Rsr { result, .. } => result.location(),
            | Self::U32Le_Rss { result, .. } => result.location(),
            | Self::U32Le_Rsi { result, .. } => result.location(),
            | Self::U32Le_Rir { result, .. } => result.location(),
            | Self::U32Le_Ris { result, .. } => result.location(),
            | Self::I64Eq_Rrs { result, .. } => result.location(),
            | Self::I64Eq_Rri { result, .. } => result.location(),
            | Self::I64Eq_Rss { result, .. } => result.location(),
            | Self::I64Eq_Rsi { result, .. } => result.location(),
            | Self::I64And_Rrs { result, .. } => result.location(),
            | Self::I64And_Rri { result, .. } => result.location(),
            | Self::I64And_Rss { result, .. } => result.location(),
            | Self::I64And_Rsi { result, .. } => result.location(),
            | Self::I64Or_Rrs { result, .. } => result.location(),
            | Self::I64Or_Rri { result, .. } => result.location(),
            | Self::I64Or_Rss { result, .. } => result.location(),
            | Self::I64Or_Rsi { result, .. } => result.location(),
            | Self::I64NotEq_Rrs { result, .. } => result.location(),
            | Self::I64NotEq_Rri { result, .. } => result.location(),
            | Self::I64NotEq_Rss { result, .. } => result.location(),
            | Self::I64NotEq_Rsi { result, .. } => result.location(),
            | Self::I64NotAnd_Rrs { result, .. } => result.location(),
            | Self::I64NotAnd_Rri { result, .. } => result.location(),
            | Self::I64NotAnd_Rss { result, .. } => result.location(),
            | Self::I64NotAnd_Rsi { result, .. } => result.location(),
            | Self::I64NotOr_Rrs { result, .. } => result.location(),
            | Self::I64NotOr_Rri { result, .. } => result.location(),
            | Self::I64NotOr_Rss { result, .. } => result.location(),
            | Self::I64NotOr_Rsi { result, .. } => result.location(),
            | Self::I64Lt_Rrs { result, .. } => result.location(),
            | Self::I64Lt_Rri { result, .. } => result.location(),
            | Self::I64Lt_Rsr { result, .. } => result.location(),
            | Self::I64Lt_Rss { result, .. } => result.location(),
            | Self::I64Lt_Rsi { result, .. } => result.location(),
            | Self::I64Lt_Rir { result, .. } => result.location(),
            | Self::I64Lt_Ris { result, .. } => result.location(),
            | Self::I64Le_Rrs { result, .. } => result.location(),
            | Self::I64Le_Rri { result, .. } => result.location(),
            | Self::I64Le_Rsr { result, .. } => result.location(),
            | Self::I64Le_Rss { result, .. } => result.location(),
            | Self::I64Le_Rsi { result, .. } => result.location(),
            | Self::I64Le_Rir { result, .. } => result.location(),
            | Self::I64Le_Ris { result, .. } => result.location(),
            | Self::U64Lt_Rrs { result, .. } => result.location(),
            | Self::U64Lt_Rri { result, .. } => result.location(),
            | Self::U64Lt_Rsr { result, .. } => result.location(),
            | Self::U64Lt_Rss { result, .. } => result.location(),
            | Self::U64Lt_Rsi { result, .. } => result.location(),
            | Self::U64Lt_Rir { result, .. } => result.location(),
            | Self::U64Lt_Ris { result, .. } => result.location(),
            | Self::U64Le_Rrs { result, .. } => result.location(),
            | Self::U64Le_Rri { result, .. } => result.location(),
            | Self::U64Le_Rsr { result, .. } => result.location(),
            | Self::U64Le_Rss { result, .. } => result.location(),
            | Self::U64Le_Rsi { result, .. } => result.location(),
            | Self::U64Le_Rir { result, .. } => result.location(),
            | Self::U64Le_Ris { result, .. } => result.location(),
            | Self::F32Eq_Rrs { result, .. } => result.location(),
            | Self::F32Eq_Rri { result, .. } => result.location(),
            | Self::F32Eq_Rss { result, .. } => result.location(),
            | Self::F32Eq_Rsi { result, .. } => result.location(),
            | Self::F32Lt_Rrs { result, .. } => result.location(),
            | Self::F32Lt_Rri { result, .. } => result.location(),
            | Self::F32Lt_Rsr { result, .. } => result.location(),
            | Self::F32Lt_Rss { result, .. } => result.location(),
            | Self::F32Lt_Rsi { result, .. } => result.location(),
            | Self::F32Lt_Rir { result, .. } => result.location(),
            | Self::F32Lt_Ris { result, .. } => result.location(),
            | Self::F32Le_Rrs { result, .. } => result.location(),
            | Self::F32Le_Rri { result, .. } => result.location(),
            | Self::F32Le_Rsr { result, .. } => result.location(),
            | Self::F32Le_Rss { result, .. } => result.location(),
            | Self::F32Le_Rsi { result, .. } => result.location(),
            | Self::F32Le_Rir { result, .. } => result.location(),
            | Self::F32Le_Ris { result, .. } => result.location(),
            | Self::F32NotEq_Rrs { result, .. } => result.location(),
            | Self::F32NotEq_Rri { result, .. } => result.location(),
            | Self::F32NotEq_Rss { result, .. } => result.location(),
            | Self::F32NotEq_Rsi { result, .. } => result.location(),
            | Self::F32NotLt_Rrs { result, .. } => result.location(),
            | Self::F32NotLt_Rri { result, .. } => result.location(),
            | Self::F32NotLt_Rsr { result, .. } => result.location(),
            | Self::F32NotLt_Rss { result, .. } => result.location(),
            | Self::F32NotLt_Rsi { result, .. } => result.location(),
            | Self::F32NotLt_Rir { result, .. } => result.location(),
            | Self::F32NotLt_Ris { result, .. } => result.location(),
            | Self::F32NotLe_Rrs { result, .. } => result.location(),
            | Self::F32NotLe_Rri { result, .. } => result.location(),
            | Self::F32NotLe_Rsr { result, .. } => result.location(),
            | Self::F32NotLe_Rss { result, .. } => result.location(),
            | Self::F32NotLe_Rsi { result, .. } => result.location(),
            | Self::F32NotLe_Rir { result, .. } => result.location(),
            | Self::F32NotLe_Ris { result, .. } => result.location(),
            | Self::F64Eq_Rrs { result, .. } => result.location(),
            | Self::F64Eq_Rri { result, .. } => result.location(),
            | Self::F64Eq_Rss { result, .. } => result.location(),
            | Self::F64Eq_Rsi { result, .. } => result.location(),
            | Self::F64Lt_Rrs { result, .. } => result.location(),
            | Self::F64Lt_Rri { result, .. } => result.location(),
            | Self::F64Lt_Rsr { result, .. } => result.location(),
            | Self::F64Lt_Rss { result, .. } => result.location(),
            | Self::F64Lt_Rsi { result, .. } => result.location(),
            | Self::F64Lt_Rir { result, .. } => result.location(),
            | Self::F64Lt_Ris { result, .. } => result.location(),
            | Self::F64Le_Rrs { result, .. } => result.location(),
            | Self::F64Le_Rri { result, .. } => result.location(),
            | Self::F64Le_Rsr { result, .. } => result.location(),
            | Self::F64Le_Rss { result, .. } => result.location(),
            | Self::F64Le_Rsi { result, .. } => result.location(),
            | Self::F64Le_Rir { result, .. } => result.location(),
            | Self::F64Le_Ris { result, .. } => result.location(),
            | Self::F64NotEq_Rrs { result, .. } => result.location(),
            | Self::F64NotEq_Rri { result, .. } => result.location(),
            | Self::F64NotEq_Rss { result, .. } => result.location(),
            | Self::F64NotEq_Rsi { result, .. } => result.location(),
            | Self::F64NotLt_Rrs { result, .. } => result.location(),
            | Self::F64NotLt_Rri { result, .. } => result.location(),
            | Self::F64NotLt_Rsr { result, .. } => result.location(),
            | Self::F64NotLt_Rss { result, .. } => result.location(),
            | Self::F64NotLt_Rsi { result, .. } => result.location(),
            | Self::F64NotLt_Rir { result, .. } => result.location(),
            | Self::F64NotLt_Ris { result, .. } => result.location(),
            | Self::F64NotLe_Rrs { result, .. } => result.location(),
            | Self::F64NotLe_Rri { result, .. } => result.location(),
            | Self::F64NotLe_Rsr { result, .. } => result.location(),
            | Self::F64NotLe_Rss { result, .. } => result.location(),
            | Self::F64NotLe_Rsi { result, .. } => result.location(),
            | Self::F64NotLe_Rir { result, .. } => result.location(),
            | Self::F64NotLe_Ris { result, .. } => result.location(),
            | Self::I32Add_Rrs { result, .. } => result.location(),
            | Self::I32Add_Rri { result, .. } => result.location(),
            | Self::I32Add_Rss { result, .. } => result.location(),
            | Self::I32Add_Rsi { result, .. } => result.location(),
            | Self::I32Add_Rs_rs { result, .. } => result.location(),
            | Self::I32Add_Rs_ri { result, .. } => result.location(),
            | Self::I32Add_Rs_ss { result, .. } => result.location(),
            | Self::I32Add_Rs_si { result, .. } => result.location(),
            | Self::I32Sub_Rrs { result, .. } => result.location(),
            | Self::I32Sub_Rsr { result, .. } => result.location(),
            | Self::I32Sub_Rss { result, .. } => result.location(),
            | Self::I32Sub_Rir { result, .. } => result.location(),
            | Self::I32Sub_Ris { result, .. } => result.location(),
            | Self::I32Mul_Rrs { result, .. } => result.location(),
            | Self::I32Mul_Rri { result, .. } => result.location(),
            | Self::I32Mul_Rss { result, .. } => result.location(),
            | Self::I32Mul_Rsi { result, .. } => result.location(),
            | Self::I32Div_Rrs { result, .. } => result.location(),
            | Self::I32Div_Rri { result, .. } => result.location(),
            | Self::I32Div_Rsr { result, .. } => result.location(),
            | Self::I32Div_Rss { result, .. } => result.location(),
            | Self::I32Div_Rsi { result, .. } => result.location(),
            | Self::I32Div_Rir { result, .. } => result.location(),
            | Self::I32Div_Ris { result, .. } => result.location(),
            | Self::U32Div_Rrs { result, .. } => result.location(),
            | Self::U32Div_Rri { result, .. } => result.location(),
            | Self::U32Div_Rsr { result, .. } => result.location(),
            | Self::U32Div_Rss { result, .. } => result.location(),
            | Self::U32Div_Rsi { result, .. } => result.location(),
            | Self::U32Div_Rir { result, .. } => result.location(),
            | Self::U32Div_Ris { result, .. } => result.location(),
            | Self::I32Rem_Rrs { result, .. } => result.location(),
            | Self::I32Rem_Rri { result, .. } => result.location(),
            | Self::I32Rem_Rsr { result, .. } => result.location(),
            | Self::I32Rem_Rss { result, .. } => result.location(),
            | Self::I32Rem_Rsi { result, .. } => result.location(),
            | Self::I32Rem_Rir { result, .. } => result.location(),
            | Self::I32Rem_Ris { result, .. } => result.location(),
            | Self::U32Rem_Rrs { result, .. } => result.location(),
            | Self::U32Rem_Rri { result, .. } => result.location(),
            | Self::U32Rem_Rsr { result, .. } => result.location(),
            | Self::U32Rem_Rss { result, .. } => result.location(),
            | Self::U32Rem_Rsi { result, .. } => result.location(),
            | Self::U32Rem_Rir { result, .. } => result.location(),
            | Self::U32Rem_Ris { result, .. } => result.location(),
            | Self::I32BitAnd_Rrs { result, .. } => result.location(),
            | Self::I32BitAnd_Rri { result, .. } => result.location(),
            | Self::I32BitAnd_Rss { result, .. } => result.location(),
            | Self::I32BitAnd_Rsi { result, .. } => result.location(),
            | Self::I32BitOr_Rrs { result, .. } => result.location(),
            | Self::I32BitOr_Rri { result, .. } => result.location(),
            | Self::I32BitOr_Rss { result, .. } => result.location(),
            | Self::I32BitOr_Rsi { result, .. } => result.location(),
            | Self::I32BitXor_Rrs { result, .. } => result.location(),
            | Self::I32BitXor_Rri { result, .. } => result.location(),
            | Self::I32BitXor_Rss { result, .. } => result.location(),
            | Self::I32BitXor_Rsi { result, .. } => result.location(),
            | Self::I32Shl_Rrs { result, .. } => result.location(),
            | Self::I32Shl_Rri { result, .. } => result.location(),
            | Self::I32Shl_Rsr { result, .. } => result.location(),
            | Self::I32Shl_Rss { result, .. } => result.location(),
            | Self::I32Shl_Rsi { result, .. } => result.location(),
            | Self::I32Shl_Rir { result, .. } => result.location(),
            | Self::I32Shl_Ris { result, .. } => result.location(),
            | Self::I32Shr_Rrs { result, .. } => result.location(),
            | Self::I32Shr_Rri { result, .. } => result.location(),
            | Self::I32Shr_Rsr { result, .. } => result.location(),
            | Self::I32Shr_Rss { result, .. } => result.location(),
            | Self::I32Shr_Rsi { result, .. } => result.location(),
            | Self::I32Shr_Rir { result, .. } => result.location(),
            | Self::I32Shr_Ris { result, .. } => result.location(),
            | Self::U32Shr_Rrs { result, .. } => result.location(),
            | Self::U32Shr_Rri { result, .. } => result.location(),
            | Self::U32Shr_Rsr { result, .. } => result.location(),
            | Self::U32Shr_Rss { result, .. } => result.location(),
            | Self::U32Shr_Rsi { result, .. } => result.location(),
            | Self::U32Shr_Rir { result, .. } => result.location(),
            | Self::U32Shr_Ris { result, .. } => result.location(),
            | Self::I32Rotl_Rrs { result, .. } => result.location(),
            | Self::I32Rotl_Rri { result, .. } => result.location(),
            | Self::I32Rotl_Rsr { result, .. } => result.location(),
            | Self::I32Rotl_Rss { result, .. } => result.location(),
            | Self::I32Rotl_Rsi { result, .. } => result.location(),
            | Self::I32Rotl_Rir { result, .. } => result.location(),
            | Self::I32Rotl_Ris { result, .. } => result.location(),
            | Self::I32Rotr_Rrs { result, .. } => result.location(),
            | Self::I32Rotr_Rri { result, .. } => result.location(),
            | Self::I32Rotr_Rsr { result, .. } => result.location(),
            | Self::I32Rotr_Rss { result, .. } => result.location(),
            | Self::I32Rotr_Rsi { result, .. } => result.location(),
            | Self::I32Rotr_Rir { result, .. } => result.location(),
            | Self::I32Rotr_Ris { result, .. } => result.location(),
            | Self::I64Add_Rrs { result, .. } => result.location(),
            | Self::I64Add_Rri { result, .. } => result.location(),
            | Self::I64Add_Rss { result, .. } => result.location(),
            | Self::I64Add_Rsi { result, .. } => result.location(),
            | Self::I64Add_Rs_rs { result, .. } => result.location(),
            | Self::I64Add_Rs_ri { result, .. } => result.location(),
            | Self::I64Add_Rs_ss { result, .. } => result.location(),
            | Self::I64Add_Rs_si { result, .. } => result.location(),
            | Self::I64Sub_Rrs { result, .. } => result.location(),
            | Self::I64Sub_Rsr { result, .. } => result.location(),
            | Self::I64Sub_Rss { result, .. } => result.location(),
            | Self::I64Sub_Rir { result, .. } => result.location(),
            | Self::I64Sub_Ris { result, .. } => result.location(),
            | Self::I64Mul_Rrs { result, .. } => result.location(),
            | Self::I64Mul_Rri { result, .. } => result.location(),
            | Self::I64Mul_Rss { result, .. } => result.location(),
            | Self::I64Mul_Rsi { result, .. } => result.location(),
            | Self::I64Div_Rrs { result, .. } => result.location(),
            | Self::I64Div_Rri { result, .. } => result.location(),
            | Self::I64Div_Rsr { result, .. } => result.location(),
            | Self::I64Div_Rss { result, .. } => result.location(),
            | Self::I64Div_Rsi { result, .. } => result.location(),
            | Self::I64Div_Rir { result, .. } => result.location(),
            | Self::I64Div_Ris { result, .. } => result.location(),
            | Self::U64Div_Rrs { result, .. } => result.location(),
            | Self::U64Div_Rri { result, .. } => result.location(),
            | Self::U64Div_Rsr { result, .. } => result.location(),
            | Self::U64Div_Rss { result, .. } => result.location(),
            | Self::U64Div_Rsi { result, .. } => result.location(),
            | Self::U64Div_Rir { result, .. } => result.location(),
            | Self::U64Div_Ris { result, .. } => result.location(),
            | Self::I64Rem_Rrs { result, .. } => result.location(),
            | Self::I64Rem_Rri { result, .. } => result.location(),
            | Self::I64Rem_Rsr { result, .. } => result.location(),
            | Self::I64Rem_Rss { result, .. } => result.location(),
            | Self::I64Rem_Rsi { result, .. } => result.location(),
            | Self::I64Rem_Rir { result, .. } => result.location(),
            | Self::I64Rem_Ris { result, .. } => result.location(),
            | Self::U64Rem_Rrs { result, .. } => result.location(),
            | Self::U64Rem_Rri { result, .. } => result.location(),
            | Self::U64Rem_Rsr { result, .. } => result.location(),
            | Self::U64Rem_Rss { result, .. } => result.location(),
            | Self::U64Rem_Rsi { result, .. } => result.location(),
            | Self::U64Rem_Rir { result, .. } => result.location(),
            | Self::U64Rem_Ris { result, .. } => result.location(),
            | Self::I64BitAnd_Rrs { result, .. } => result.location(),
            | Self::I64BitAnd_Rri { result, .. } => result.location(),
            | Self::I64BitAnd_Rss { result, .. } => result.location(),
            | Self::I64BitAnd_Rsi { result, .. } => result.location(),
            | Self::I64BitOr_Rrs { result, .. } => result.location(),
            | Self::I64BitOr_Rri { result, .. } => result.location(),
            | Self::I64BitOr_Rss { result, .. } => result.location(),
            | Self::I64BitOr_Rsi { result, .. } => result.location(),
            | Self::I64BitXor_Rrs { result, .. } => result.location(),
            | Self::I64BitXor_Rri { result, .. } => result.location(),
            | Self::I64BitXor_Rss { result, .. } => result.location(),
            | Self::I64BitXor_Rsi { result, .. } => result.location(),
            | Self::I64Shl_Rrs { result, .. } => result.location(),
            | Self::I64Shl_Rri { result, .. } => result.location(),
            | Self::I64Shl_Rsr { result, .. } => result.location(),
            | Self::I64Shl_Rss { result, .. } => result.location(),
            | Self::I64Shl_Rsi { result, .. } => result.location(),
            | Self::I64Shl_Rir { result, .. } => result.location(),
            | Self::I64Shl_Ris { result, .. } => result.location(),
            | Self::I64Shr_Rrs { result, .. } => result.location(),
            | Self::I64Shr_Rri { result, .. } => result.location(),
            | Self::I64Shr_Rsr { result, .. } => result.location(),
            | Self::I64Shr_Rss { result, .. } => result.location(),
            | Self::I64Shr_Rsi { result, .. } => result.location(),
            | Self::I64Shr_Rir { result, .. } => result.location(),
            | Self::I64Shr_Ris { result, .. } => result.location(),
            | Self::U64Shr_Rrs { result, .. } => result.location(),
            | Self::U64Shr_Rri { result, .. } => result.location(),
            | Self::U64Shr_Rsr { result, .. } => result.location(),
            | Self::U64Shr_Rss { result, .. } => result.location(),
            | Self::U64Shr_Rsi { result, .. } => result.location(),
            | Self::U64Shr_Rir { result, .. } => result.location(),
            | Self::U64Shr_Ris { result, .. } => result.location(),
            | Self::I64Rotl_Rrs { result, .. } => result.location(),
            | Self::I64Rotl_Rri { result, .. } => result.location(),
            | Self::I64Rotl_Rsr { result, .. } => result.location(),
            | Self::I64Rotl_Rss { result, .. } => result.location(),
            | Self::I64Rotl_Rsi { result, .. } => result.location(),
            | Self::I64Rotl_Rir { result, .. } => result.location(),
            | Self::I64Rotl_Ris { result, .. } => result.location(),
            | Self::I64Rotr_Rrs { result, .. } => result.location(),
            | Self::I64Rotr_Rri { result, .. } => result.location(),
            | Self::I64Rotr_Rsr { result, .. } => result.location(),
            | Self::I64Rotr_Rss { result, .. } => result.location(),
            | Self::I64Rotr_Rsi { result, .. } => result.location(),
            | Self::I64Rotr_Rir { result, .. } => result.location(),
            | Self::I64Rotr_Ris { result, .. } => result.location(),
            | Self::F32Add_Rrs { result, .. } => result.location(),
            | Self::F32Add_Rri { result, .. } => result.location(),
            | Self::F32Add_Rsr { result, .. } => result.location(),
            | Self::F32Add_Rss { result, .. } => result.location(),
            | Self::F32Add_Rsi { result, .. } => result.location(),
            | Self::F32Add_Rir { result, .. } => result.location(),
            | Self::F32Add_Ris { result, .. } => result.location(),
            | Self::F32Sub_Rrs { result, .. } => result.location(),
            | Self::F32Sub_Rri { result, .. } => result.location(),
            | Self::F32Sub_Rsr { result, .. } => result.location(),
            | Self::F32Sub_Rss { result, .. } => result.location(),
            | Self::F32Sub_Rsi { result, .. } => result.location(),
            | Self::F32Sub_Rir { result, .. } => result.location(),
            | Self::F32Sub_Ris { result, .. } => result.location(),
            | Self::F32Mul_Rrs { result, .. } => result.location(),
            | Self::F32Mul_Rri { result, .. } => result.location(),
            | Self::F32Mul_Rsr { result, .. } => result.location(),
            | Self::F32Mul_Rss { result, .. } => result.location(),
            | Self::F32Mul_Rsi { result, .. } => result.location(),
            | Self::F32Mul_Rir { result, .. } => result.location(),
            | Self::F32Mul_Ris { result, .. } => result.location(),
            | Self::F32Div_Rrs { result, .. } => result.location(),
            | Self::F32Div_Rri { result, .. } => result.location(),
            | Self::F32Div_Rsr { result, .. } => result.location(),
            | Self::F32Div_Rss { result, .. } => result.location(),
            | Self::F32Div_Rsi { result, .. } => result.location(),
            | Self::F32Div_Rir { result, .. } => result.location(),
            | Self::F32Div_Ris { result, .. } => result.location(),
            | Self::F32Min_Rrs { result, .. } => result.location(),
            | Self::F32Min_Rri { result, .. } => result.location(),
            | Self::F32Min_Rsr { result, .. } => result.location(),
            | Self::F32Min_Rss { result, .. } => result.location(),
            | Self::F32Min_Rsi { result, .. } => result.location(),
            | Self::F32Min_Rir { result, .. } => result.location(),
            | Self::F32Min_Ris { result, .. } => result.location(),
            | Self::F32Max_Rrs { result, .. } => result.location(),
            | Self::F32Max_Rri { result, .. } => result.location(),
            | Self::F32Max_Rsr { result, .. } => result.location(),
            | Self::F32Max_Rss { result, .. } => result.location(),
            | Self::F32Max_Rsi { result, .. } => result.location(),
            | Self::F32Max_Rir { result, .. } => result.location(),
            | Self::F32Max_Ris { result, .. } => result.location(),
            | Self::F64Add_Rrs { result, .. } => result.location(),
            | Self::F64Add_Rri { result, .. } => result.location(),
            | Self::F64Add_Rsr { result, .. } => result.location(),
            | Self::F64Add_Rss { result, .. } => result.location(),
            | Self::F64Add_Rsi { result, .. } => result.location(),
            | Self::F64Add_Rir { result, .. } => result.location(),
            | Self::F64Add_Ris { result, .. } => result.location(),
            | Self::F64Sub_Rrs { result, .. } => result.location(),
            | Self::F64Sub_Rri { result, .. } => result.location(),
            | Self::F64Sub_Rsr { result, .. } => result.location(),
            | Self::F64Sub_Rss { result, .. } => result.location(),
            | Self::F64Sub_Rsi { result, .. } => result.location(),
            | Self::F64Sub_Rir { result, .. } => result.location(),
            | Self::F64Sub_Ris { result, .. } => result.location(),
            | Self::F64Mul_Rrs { result, .. } => result.location(),
            | Self::F64Mul_Rri { result, .. } => result.location(),
            | Self::F64Mul_Rsr { result, .. } => result.location(),
            | Self::F64Mul_Rss { result, .. } => result.location(),
            | Self::F64Mul_Rsi { result, .. } => result.location(),
            | Self::F64Mul_Rir { result, .. } => result.location(),
            | Self::F64Mul_Ris { result, .. } => result.location(),
            | Self::F64Div_Rrs { result, .. } => result.location(),
            | Self::F64Div_Rri { result, .. } => result.location(),
            | Self::F64Div_Rsr { result, .. } => result.location(),
            | Self::F64Div_Rss { result, .. } => result.location(),
            | Self::F64Div_Rsi { result, .. } => result.location(),
            | Self::F64Div_Rir { result, .. } => result.location(),
            | Self::F64Div_Ris { result, .. } => result.location(),
            | Self::F64Min_Rrs { result, .. } => result.location(),
            | Self::F64Min_Rri { result, .. } => result.location(),
            | Self::F64Min_Rsr { result, .. } => result.location(),
            | Self::F64Min_Rss { result, .. } => result.location(),
            | Self::F64Min_Rsi { result, .. } => result.location(),
            | Self::F64Min_Rir { result, .. } => result.location(),
            | Self::F64Min_Ris { result, .. } => result.location(),
            | Self::F64Max_Rrs { result, .. } => result.location(),
            | Self::F64Max_Rri { result, .. } => result.location(),
            | Self::F64Max_Rsr { result, .. } => result.location(),
            | Self::F64Max_Rss { result, .. } => result.location(),
            | Self::F64Max_Rsi { result, .. } => result.location(),
            | Self::F64Max_Rir { result, .. } => result.location(),
            | Self::F64Max_Ris { result, .. } => result.location(),
            | Self::F32Copysign_Rrs { result, .. } => result.location(),
            | Self::F32Copysign_Rsr { result, .. } => result.location(),
            | Self::F32Copysign_Rss { result, .. } => result.location(),
            | Self::F32Copysign_Rir { result, .. } => result.location(),
            | Self::F32Copysign_Ris { result, .. } => result.location(),
            | Self::F64Copysign_Rrs { result, .. } => result.location(),
            | Self::F64Copysign_Rsr { result, .. } => result.location(),
            | Self::F64Copysign_Rss { result, .. } => result.location(),
            | Self::F64Copysign_Rir { result, .. } => result.location(),
            | Self::F64Copysign_Ris { result, .. } => result.location(),
            | Self::I32Mul_Rrr { result, .. } => result.location(),
            | Self::I64Mul_Rrr { result, .. } => result.location(),
            | Self::F32Mul_Rrr { result, .. } => result.location(),
            | Self::F64Mul_Rrr { result, .. } => result.location(),
            | Self::U32Select_Rrri { result, .. } => result.location(),
            | Self::U32Select_Rrsi { result, .. } => result.location(),
            | Self::U32Select_Rrir { result, .. } => result.location(),
            | Self::U32Select_Rris { result, .. } => result.location(),
            | Self::U32Select_Rrii { result, .. } => result.location(),
            | Self::U32Select_Rsri { result, .. } => result.location(),
            | Self::U32Select_Rssi { result, .. } => result.location(),
            | Self::U32Select_Rsir { result, .. } => result.location(),
            | Self::U32Select_Rsis { result, .. } => result.location(),
            | Self::U32Select_Rsii { result, .. } => result.location(),
            | Self::U64Select_Rrrs { result, .. } => result.location(),
            | Self::U64Select_Rrri { result, .. } => result.location(),
            | Self::U64Select_Rrsr { result, .. } => result.location(),
            | Self::U64Select_Rrss { result, .. } => result.location(),
            | Self::U64Select_Rrsi { result, .. } => result.location(),
            | Self::U64Select_Rrir { result, .. } => result.location(),
            | Self::U64Select_Rris { result, .. } => result.location(),
            | Self::U64Select_Rrii { result, .. } => result.location(),
            | Self::U64Select_Rsrs { result, .. } => result.location(),
            | Self::U64Select_Rsri { result, .. } => result.location(),
            | Self::U64Select_Rssr { result, .. } => result.location(),
            | Self::U64Select_Rsss { result, .. } => result.location(),
            | Self::U64Select_Rssi { result, .. } => result.location(),
            | Self::U64Select_Rsir { result, .. } => result.location(),
            | Self::U64Select_Rsis { result, .. } => result.location(),
            | Self::U64Select_Rsii { result, .. } => result.location(),
            | Self::F32Select_Rrrs { result, .. } => result.location(),
            | Self::F32Select_Rrri { result, .. } => result.location(),
            | Self::F32Select_Rrsr { result, .. } => result.location(),
            | Self::F32Select_Rrss { result, .. } => result.location(),
            | Self::F32Select_Rrsi { result, .. } => result.location(),
            | Self::F32Select_Rrir { result, .. } => result.location(),
            | Self::F32Select_Rris { result, .. } => result.location(),
            | Self::F32Select_Rrii { result, .. } => result.location(),
            | Self::F32Select_Rsrs { result, .. } => result.location(),
            | Self::F32Select_Rsri { result, .. } => result.location(),
            | Self::F32Select_Rssr { result, .. } => result.location(),
            | Self::F32Select_Rsss { result, .. } => result.location(),
            | Self::F32Select_Rssi { result, .. } => result.location(),
            | Self::F32Select_Rsir { result, .. } => result.location(),
            | Self::F32Select_Rsis { result, .. } => result.location(),
            | Self::F32Select_Rsii { result, .. } => result.location(),
            | Self::F64Select_Rrrs { result, .. } => result.location(),
            | Self::F64Select_Rrri { result, .. } => result.location(),
            | Self::F64Select_Rrsr { result, .. } => result.location(),
            | Self::F64Select_Rrss { result, .. } => result.location(),
            | Self::F64Select_Rrsi { result, .. } => result.location(),
            | Self::F64Select_Rrir { result, .. } => result.location(),
            | Self::F64Select_Rris { result, .. } => result.location(),
            | Self::F64Select_Rrii { result, .. } => result.location(),
            | Self::F64Select_Rsrs { result, .. } => result.location(),
            | Self::F64Select_Rsri { result, .. } => result.location(),
            | Self::F64Select_Rssr { result, .. } => result.location(),
            | Self::F64Select_Rsss { result, .. } => result.location(),
            | Self::F64Select_Rssi { result, .. } => result.location(),
            | Self::F64Select_Rsir { result, .. } => result.location(),
            | Self::F64Select_Rsis { result, .. } => result.location(),
            | Self::F64Select_Rsii { result, .. } => result.location(),
            | Self::U32Load_Rr { result, .. } => result.location(),
            | Self::U32LoadMem0Offset16_Rr { result, .. } => result.location(),
            | Self::U32LoadMem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U32Load_Rs { result, .. } => result.location(),
            | Self::U32LoadMem0Offset16_Rs { result, .. } => result.location(),
            | Self::U32LoadMem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U32Load_Ri { result, .. } => result.location(),
            | Self::U64Load_Rr { result, .. } => result.location(),
            | Self::U64LoadMem0Offset16_Rr { result, .. } => result.location(),
            | Self::U64LoadMem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U64Load_Rs { result, .. } => result.location(),
            | Self::U64LoadMem0Offset16_Rs { result, .. } => result.location(),
            | Self::U64LoadMem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U64Load_Ri { result, .. } => result.location(),
            | Self::F32Load_Rr { result, .. } => result.location(),
            | Self::F32LoadMem0Offset16_Rr { result, .. } => result.location(),
            | Self::F32LoadMem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::F32Load_Rs { result, .. } => result.location(),
            | Self::F32LoadMem0Offset16_Rs { result, .. } => result.location(),
            | Self::F32LoadMem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::F32Load_Ri { result, .. } => result.location(),
            | Self::F64Load_Rr { result, .. } => result.location(),
            | Self::F64LoadMem0Offset16_Rr { result, .. } => result.location(),
            | Self::F64LoadMem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::F64Load_Rs { result, .. } => result.location(),
            | Self::F64LoadMem0Offset16_Rs { result, .. } => result.location(),
            | Self::F64LoadMem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::F64Load_Ri { result, .. } => result.location(),
            | Self::I32LoadExtend8_Rr { result, .. } => result.location(),
            | Self::I32LoadExtend8Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::I32LoadExtend8Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::I32LoadExtend8_Rs { result, .. } => result.location(),
            | Self::I32LoadExtend8Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::I32LoadExtend8Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::I32LoadExtend8_Ri { result, .. } => result.location(),
            | Self::I32LoadExtend16_Rr { result, .. } => result.location(),
            | Self::I32LoadExtend16Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::I32LoadExtend16Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::I32LoadExtend16_Rs { result, .. } => result.location(),
            | Self::I32LoadExtend16Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::I32LoadExtend16Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::I32LoadExtend16_Ri { result, .. } => result.location(),
            | Self::U32LoadExtend8_Rr { result, .. } => result.location(),
            | Self::U32LoadExtend8Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::U32LoadExtend8Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U32LoadExtend8_Rs { result, .. } => result.location(),
            | Self::U32LoadExtend8Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::U32LoadExtend8Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U32LoadExtend8_Ri { result, .. } => result.location(),
            | Self::U32LoadExtend16_Rr { result, .. } => result.location(),
            | Self::U32LoadExtend16Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::U32LoadExtend16Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U32LoadExtend16_Rs { result, .. } => result.location(),
            | Self::U32LoadExtend16Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::U32LoadExtend16Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U32LoadExtend16_Ri { result, .. } => result.location(),
            | Self::I64LoadExtend8_Rr { result, .. } => result.location(),
            | Self::I64LoadExtend8Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::I64LoadExtend8Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::I64LoadExtend8_Rs { result, .. } => result.location(),
            | Self::I64LoadExtend8Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::I64LoadExtend8Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::I64LoadExtend8_Ri { result, .. } => result.location(),
            | Self::I64LoadExtend16_Rr { result, .. } => result.location(),
            | Self::I64LoadExtend16Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::I64LoadExtend16Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::I64LoadExtend16_Rs { result, .. } => result.location(),
            | Self::I64LoadExtend16Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::I64LoadExtend16Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::I64LoadExtend16_Ri { result, .. } => result.location(),
            | Self::I64LoadExtend32_Rr { result, .. } => result.location(),
            | Self::I64LoadExtend32Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::I64LoadExtend32Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::I64LoadExtend32_Rs { result, .. } => result.location(),
            | Self::I64LoadExtend32Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::I64LoadExtend32Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::I64LoadExtend32_Ri { result, .. } => result.location(),
            | Self::U64LoadExtend8_Rr { result, .. } => result.location(),
            | Self::U64LoadExtend8Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::U64LoadExtend8Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U64LoadExtend8_Rs { result, .. } => result.location(),
            | Self::U64LoadExtend8Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::U64LoadExtend8Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U64LoadExtend8_Ri { result, .. } => result.location(),
            | Self::U64LoadExtend16_Rr { result, .. } => result.location(),
            | Self::U64LoadExtend16Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::U64LoadExtend16Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U64LoadExtend16_Rs { result, .. } => result.location(),
            | Self::U64LoadExtend16Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::U64LoadExtend16Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U64LoadExtend16_Ri { result, .. } => result.location(),
            | Self::U64LoadExtend32_Rr { result, .. } => result.location(),
            | Self::U64LoadExtend32Mem0Offset16_Rr { result, .. } => result.location(),
            | Self::U64LoadExtend32Mem0Offset16_Rs_r { result, .. } => result.location(),
            | Self::U64LoadExtend32_Rs { result, .. } => result.location(),
            | Self::U64LoadExtend32Mem0Offset16_Rs { result, .. } => result.location(),
            | Self::U64LoadExtend32Mem0Offset16_Rs_s { result, .. } => result.location(),
            | Self::U64LoadExtend32_Ri { result, .. } => result.location(),
            | Self::U32Copy_Ri { result, .. } => result.location(),
            | Self::U32Copy_Si { result, .. } => result.location(),
            | Self::U64Copy_Rs { result, .. } => result.location(),
            | Self::U64Copy_Ri { result, .. } => result.location(),
            | Self::U64Copy_Sr { result, .. } => result.location(),
            | Self::U64Copy_Ss { result, .. } => result.location(),
            | Self::U64Copy_Si { result, .. } => result.location(),
            | Self::F32Copy_Ri { result, .. } => result.location(),
            | Self::F32Copy_Rs { result, .. } => result.location(),
            | Self::F32Copy_Sr { result, .. } => result.location(),
            | Self::F64Copy_Rs { result, .. } => result.location(),
            | Self::F64Copy_Ri { result, .. } => result.location(),
            | Self::F64Copy_Sr { result, .. } => result.location(),
            | Self::F32ReinterpretI32_Rr { result, .. } => result.location(),
            | Self::I32ReinterpretF32_Rr { result, .. } => result.location(),
            | Self::F64ReinterpretI64_Rr { result, .. } => result.location(),
            | Self::I64ReinterpretF64_Rr { result, .. } => result.location(),
            | Self::RefFunc { result, .. } => result.location(),
            | Self::GlobalGetU64_R { result, .. } => result.location(),
            | Self::GlobalGetF32_R { result, .. } => result.location(),
            | Self::GlobalGetF64_R { result, .. } => result.location(),
            | Self::MemorySize { result, .. } => result.location(),
            | Self::MemoryGrow { result, .. } => result.location(),
            | Self::TableGet_Rr { result, .. } => result.location(),
            | Self::TableGet_Rs { result, .. } => result.location(),
            | Self::TableGet_Ri { result, .. } => result.location(),
            | Self::TableSize { result, .. } => result.location(),
            | Self::TableGrow { result, .. } => result.location(),
            _ => return None,
        };
        Some(loc)
    }
    /// Returns a shared reference to the result [`Slot`] of `self` if any.
    pub fn result_ref(&self) -> Option<&Slot> {
        let res = match self {
            | Self::U32Copy_Si { result, .. }
            | Self::U64Copy_Sr { result, .. }
            | Self::U64Copy_Ss { result, .. }
            | Self::U64Copy_Si { result, .. }
            | Self::F32Copy_Sr { result, .. }
            | Self::F64Copy_Sr { result, .. } => result,
            | Self::I32Add_Rs_rs { result, .. } => &result.slot,
            | Self::I32Add_Rs_ri { result, .. } => &result.slot,
            | Self::I32Add_Rs_ss { result, .. } => &result.slot,
            | Self::I32Add_Rs_si { result, .. } => &result.slot,
            | Self::I64Add_Rs_rs { result, .. } => &result.slot,
            | Self::I64Add_Rs_ri { result, .. } => &result.slot,
            | Self::I64Add_Rs_ss { result, .. } => &result.slot,
            | Self::I64Add_Rs_si { result, .. } => &result.slot,
            | Self::U32LoadMem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U32LoadMem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::U64LoadMem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U64LoadMem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::F32LoadMem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::F32LoadMem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::F64LoadMem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::F64LoadMem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::I32LoadExtend8Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::I32LoadExtend8Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::I32LoadExtend16Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::I32LoadExtend16Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::U32LoadExtend8Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U32LoadExtend8Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::U32LoadExtend16Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U32LoadExtend16Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::I64LoadExtend8Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::I64LoadExtend8Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::I64LoadExtend16Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::I64LoadExtend16Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::I64LoadExtend32Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::I64LoadExtend32Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::U64LoadExtend8Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U64LoadExtend8Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::U64LoadExtend16Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U64LoadExtend16Mem0Offset16_Rs_s { result, .. } => &result.slot,
            | Self::U64LoadExtend32Mem0Offset16_Rs_r { result, .. } => &result.slot,
            | Self::U64LoadExtend32Mem0Offset16_Rs_s { result, .. } => &result.slot,
            _ => return None,
        };
        Some(res)
    }

    /// Returns an exclusive reference to the result [`Slot`] of `self` if any.
    pub fn result_mut(&mut self) -> Option<&mut Slot> {
        let res = match self {
            | Self::U32Copy_Si { result, .. }
            | Self::U64Copy_Sr { result, .. }
            | Self::U64Copy_Ss { result, .. }
            | Self::U64Copy_Si { result, .. }
            | Self::F32Copy_Sr { result, .. }
            | Self::F64Copy_Sr { result, .. } => result,
            | Self::I32Add_Rs_rs { result, .. } => &mut result.slot,
            | Self::I32Add_Rs_ri { result, .. } => &mut result.slot,
            | Self::I32Add_Rs_ss { result, .. } => &mut result.slot,
            | Self::I32Add_Rs_si { result, .. } => &mut result.slot,
            | Self::I64Add_Rs_rs { result, .. } => &mut result.slot,
            | Self::I64Add_Rs_ri { result, .. } => &mut result.slot,
            | Self::I64Add_Rs_ss { result, .. } => &mut result.slot,
            | Self::I64Add_Rs_si { result, .. } => &mut result.slot,
            | Self::U32LoadMem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U32LoadMem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::U64LoadMem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U64LoadMem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::F32LoadMem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::F32LoadMem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::F64LoadMem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::F64LoadMem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::I32LoadExtend8Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::I32LoadExtend8Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::I32LoadExtend16Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::I32LoadExtend16Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::U32LoadExtend8Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U32LoadExtend8Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::U32LoadExtend16Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U32LoadExtend16Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::I64LoadExtend8Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::I64LoadExtend8Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::I64LoadExtend16Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::I64LoadExtend16Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::I64LoadExtend32Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::I64LoadExtend32Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::U64LoadExtend8Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U64LoadExtend8Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::U64LoadExtend16Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U64LoadExtend16Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            | Self::U64LoadExtend32Mem0Offset16_Rs_r { result, .. } => &mut result.slot,
            | Self::U64LoadExtend32Mem0Offset16_Rs_s { result, .. } => &mut result.slot,
            _ => return None,
        };
        Some(res)
    }
}

impl Op {
    pub fn i32_clz_rs(value: Slot) -> Self {
        Self::I32Clz_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_clz_rr() -> Self {
        Self::I32Clz_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i32_ctz_rs(value: Slot) -> Self {
        Self::I32Ctz_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_ctz_rr() -> Self {
        Self::I32Ctz_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i32_popcnt_rs(value: Slot) -> Self {
        Self::I32Popcnt_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_popcnt_rr() -> Self {
        Self::I32Popcnt_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i32_sext8_rs(value: Slot) -> Self {
        Self::I32Sext8_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_sext8_rr() -> Self {
        Self::I32Sext8_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i32_sext16_rs(value: Slot) -> Self {
        Self::I32Sext16_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_sext16_rr() -> Self {
        Self::I32Sext16_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i32_wrap_i64_rs(value: Slot) -> Self {
        Self::I32WrapI64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_wrap_i64_rr() -> Self {
        Self::I32WrapI64_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_clz_rs(value: Slot) -> Self {
        Self::I64Clz_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_clz_rr() -> Self {
        Self::I64Clz_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_ctz_rs(value: Slot) -> Self {
        Self::I64Ctz_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_ctz_rr() -> Self {
        Self::I64Ctz_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_popcnt_rs(value: Slot) -> Self {
        Self::I64Popcnt_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_popcnt_rr() -> Self {
        Self::I64Popcnt_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_sext8_rs(value: Slot) -> Self {
        Self::I64Sext8_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_sext8_rr() -> Self {
        Self::I64Sext8_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_sext16_rs(value: Slot) -> Self {
        Self::I64Sext16_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_sext16_rr() -> Self {
        Self::I64Sext16_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_sext32_rs(value: Slot) -> Self {
        Self::I64Sext32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_sext32_rr() -> Self {
        Self::I64Sext32_Rr { result: <Reg<i64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f32_abs_rs(value: Slot) -> Self {
        Self::F32Abs_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_abs_rr() -> Self {
        Self::F32Abs_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_neg_rs(value: Slot) -> Self {
        Self::F32Neg_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_neg_rr() -> Self {
        Self::F32Neg_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_nabs_rs(value: Slot) -> Self {
        Self::F32Nabs_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_nabs_rr() -> Self {
        Self::F32Nabs_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_ceil_rs(value: Slot) -> Self {
        Self::F32Ceil_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_ceil_rr() -> Self {
        Self::F32Ceil_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_floor_rs(value: Slot) -> Self {
        Self::F32Floor_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_floor_rr() -> Self {
        Self::F32Floor_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_trunc_rs(value: Slot) -> Self {
        Self::F32Trunc_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_trunc_rr() -> Self {
        Self::F32Trunc_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_nearest_rs(value: Slot) -> Self {
        Self::F32Nearest_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_nearest_rr() -> Self {
        Self::F32Nearest_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_sqrt_rs(value: Slot) -> Self {
        Self::F32Sqrt_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_sqrt_rr() -> Self {
        Self::F32Sqrt_Rr { result: <Reg<f32>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_convert_i32_rs(value: Slot) -> Self {
        Self::F32ConvertI32_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_convert_i32_rr() -> Self {
        Self::F32ConvertI32_Rr { result: <Reg<f32>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f32_convert_u32_rs(value: Slot) -> Self {
        Self::F32ConvertU32_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_convert_u32_rr() -> Self {
        Self::F32ConvertU32_Rr { result: <Reg<f32>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f32_convert_i64_rs(value: Slot) -> Self {
        Self::F32ConvertI64_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_convert_i64_rr() -> Self {
        Self::F32ConvertI64_Rr { result: <Reg<f32>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f32_convert_u64_rs(value: Slot) -> Self {
        Self::F32ConvertU64_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_convert_u64_rr() -> Self {
        Self::F32ConvertU64_Rr { result: <Reg<f32>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f32_demote_f64_rs(value: Slot) -> Self {
        Self::F32DemoteF64_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_demote_f64_rr() -> Self {
        Self::F32DemoteF64_Rr { result: <Reg<f32>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_abs_rs(value: Slot) -> Self {
        Self::F64Abs_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_abs_rr() -> Self {
        Self::F64Abs_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_neg_rs(value: Slot) -> Self {
        Self::F64Neg_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_neg_rr() -> Self {
        Self::F64Neg_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_nabs_rs(value: Slot) -> Self {
        Self::F64Nabs_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_nabs_rr() -> Self {
        Self::F64Nabs_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_ceil_rs(value: Slot) -> Self {
        Self::F64Ceil_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_ceil_rr() -> Self {
        Self::F64Ceil_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_floor_rs(value: Slot) -> Self {
        Self::F64Floor_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_floor_rr() -> Self {
        Self::F64Floor_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_trunc_rs(value: Slot) -> Self {
        Self::F64Trunc_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_trunc_rr() -> Self {
        Self::F64Trunc_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_nearest_rs(value: Slot) -> Self {
        Self::F64Nearest_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_nearest_rr() -> Self {
        Self::F64Nearest_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_sqrt_rs(value: Slot) -> Self {
        Self::F64Sqrt_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_sqrt_rr() -> Self {
        Self::F64Sqrt_Rr { result: <Reg<f64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_convert_i32_rs(value: Slot) -> Self {
        Self::F64ConvertI32_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_convert_i32_rr() -> Self {
        Self::F64ConvertI32_Rr { result: <Reg<f64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f64_convert_u32_rs(value: Slot) -> Self {
        Self::F64ConvertU32_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_convert_u32_rr() -> Self {
        Self::F64ConvertU32_Rr { result: <Reg<f64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f64_convert_i64_rs(value: Slot) -> Self {
        Self::F64ConvertI64_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_convert_i64_rr() -> Self {
        Self::F64ConvertI64_Rr { result: <Reg<f64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f64_convert_u64_rs(value: Slot) -> Self {
        Self::F64ConvertU64_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_convert_u64_rr() -> Self {
        Self::F64ConvertU64_Rr { result: <Reg<f64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f64_promote_f32_rs(value: Slot) -> Self {
        Self::F64PromoteF32_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_promote_f32_rr() -> Self {
        Self::F64PromoteF32_Rr { result: <Reg<f64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn i32_trunc_f32_rs(value: Slot) -> Self {
        Self::I32TruncF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_trunc_f32_rr() -> Self {
        Self::I32TruncF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn u32_trunc_f32_rs(value: Slot) -> Self {
        Self::U32TruncF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u32_trunc_f32_rr() -> Self {
        Self::U32TruncF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn i32_trunc_f64_rs(value: Slot) -> Self {
        Self::I32TruncF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_trunc_f64_rr() -> Self {
        Self::I32TruncF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn u32_trunc_f64_rs(value: Slot) -> Self {
        Self::U32TruncF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u32_trunc_f64_rr() -> Self {
        Self::U32TruncF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn i64_trunc_f32_rs(value: Slot) -> Self {
        Self::I64TruncF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_trunc_f32_rr() -> Self {
        Self::I64TruncF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn u64_trunc_f32_rs(value: Slot) -> Self {
        Self::U64TruncF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u64_trunc_f32_rr() -> Self {
        Self::U64TruncF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn i64_trunc_f64_rs(value: Slot) -> Self {
        Self::I64TruncF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_trunc_f64_rr() -> Self {
        Self::I64TruncF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn u64_trunc_f64_rs(value: Slot) -> Self {
        Self::U64TruncF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u64_trunc_f64_rr() -> Self {
        Self::U64TruncF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn i32_trunc_sat_f32_rs(value: Slot) -> Self {
        Self::I32TruncSatF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_trunc_sat_f32_rr() -> Self {
        Self::I32TruncSatF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn u32_trunc_sat_f32_rs(value: Slot) -> Self {
        Self::U32TruncSatF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u32_trunc_sat_f32_rr() -> Self {
        Self::U32TruncSatF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn i32_trunc_sat_f64_rs(value: Slot) -> Self {
        Self::I32TruncSatF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i32_trunc_sat_f64_rr() -> Self {
        Self::I32TruncSatF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn u32_trunc_sat_f64_rs(value: Slot) -> Self {
        Self::U32TruncSatF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u32_trunc_sat_f64_rr() -> Self {
        Self::U32TruncSatF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn i64_trunc_sat_f32_rs(value: Slot) -> Self {
        Self::I64TruncSatF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_trunc_sat_f32_rr() -> Self {
        Self::I64TruncSatF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn u64_trunc_sat_f32_rs(value: Slot) -> Self {
        Self::U64TruncSatF32_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u64_trunc_sat_f32_rr() -> Self {
        Self::U64TruncSatF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn i64_trunc_sat_f64_rs(value: Slot) -> Self {
        Self::I64TruncSatF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn i64_trunc_sat_f64_rr() -> Self {
        Self::I64TruncSatF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn u64_trunc_sat_f64_rs(value: Slot) -> Self {
        Self::U64TruncSatF64_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u64_trunc_sat_f64_rr() -> Self {
        Self::U64TruncSatF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn i32_eq_rrs(rhs: Slot) -> Self {
        Self::I32Eq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_eq_rri(rhs: i32) -> Self {
        Self::I32Eq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Eq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_eq_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32Eq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_and_rrs(rhs: Slot) -> Self {
        Self::I32And_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_and_rri(rhs: i32) -> Self {
        Self::I32And_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_and_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32And_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_and_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32And_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_or_rrs(rhs: Slot) -> Self {
        Self::I32Or_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_or_rri(rhs: i32) -> Self {
        Self::I32Or_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_or_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Or_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_or_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32Or_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_not_eq_rrs(rhs: Slot) -> Self {
        Self::I32NotEq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_not_eq_rri(rhs: i32) -> Self {
        Self::I32NotEq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_not_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32NotEq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_not_eq_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32NotEq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_not_and_rrs(rhs: Slot) -> Self {
        Self::I32NotAnd_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_not_and_rri(rhs: i32) -> Self {
        Self::I32NotAnd_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_not_and_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32NotAnd_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_not_and_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32NotAnd_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_not_or_rrs(rhs: Slot) -> Self {
        Self::I32NotOr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_not_or_rri(rhs: i32) -> Self {
        Self::I32NotOr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_not_or_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32NotOr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_not_or_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32NotOr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_lt_rrs(rhs: Slot) -> Self {
        Self::I32Lt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_lt_rri(rhs: i32) -> Self {
        Self::I32Lt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_lt_rsr(lhs: Slot) -> Self {
        Self::I32Lt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Lt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_lt_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32Lt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_lt_rir(lhs: i32) -> Self {
        Self::I32Lt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_lt_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Lt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_le_rrs(rhs: Slot) -> Self {
        Self::I32Le_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_le_rri(rhs: i32) -> Self {
        Self::I32Le_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_le_rsr(lhs: Slot) -> Self {
        Self::I32Le_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Le_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_le_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32Le_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_le_rir(lhs: i32) -> Self {
        Self::I32Le_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_le_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Le_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_lt_rrs(rhs: Slot) -> Self {
        Self::U32Lt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_lt_rri(rhs: u32) -> Self {
        Self::U32Lt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_lt_rsr(lhs: Slot) -> Self {
        Self::U32Lt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U32Lt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_lt_rsi(lhs: Slot, rhs: u32) -> Self {
        Self::U32Lt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_lt_rir(lhs: u32) -> Self {
        Self::U32Lt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_lt_ris(lhs: u32, rhs: Slot) -> Self {
        Self::U32Lt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_le_rrs(rhs: Slot) -> Self {
        Self::U32Le_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_le_rri(rhs: u32) -> Self {
        Self::U32Le_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_le_rsr(lhs: Slot) -> Self {
        Self::U32Le_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U32Le_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_le_rsi(lhs: Slot, rhs: u32) -> Self {
        Self::U32Le_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_le_rir(lhs: u32) -> Self {
        Self::U32Le_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_le_ris(lhs: u32, rhs: Slot) -> Self {
        Self::U32Le_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_eq_rrs(rhs: Slot) -> Self {
        Self::I64Eq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_eq_rri(rhs: i64) -> Self {
        Self::I64Eq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Eq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_eq_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64Eq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_and_rrs(rhs: Slot) -> Self {
        Self::I64And_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_and_rri(rhs: i64) -> Self {
        Self::I64And_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_and_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64And_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_and_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64And_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_or_rrs(rhs: Slot) -> Self {
        Self::I64Or_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_or_rri(rhs: i64) -> Self {
        Self::I64Or_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_or_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Or_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_or_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64Or_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_not_eq_rrs(rhs: Slot) -> Self {
        Self::I64NotEq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_not_eq_rri(rhs: i64) -> Self {
        Self::I64NotEq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_not_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64NotEq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_not_eq_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64NotEq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_not_and_rrs(rhs: Slot) -> Self {
        Self::I64NotAnd_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_not_and_rri(rhs: i64) -> Self {
        Self::I64NotAnd_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_not_and_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64NotAnd_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_not_and_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64NotAnd_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_not_or_rrs(rhs: Slot) -> Self {
        Self::I64NotOr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_not_or_rri(rhs: i64) -> Self {
        Self::I64NotOr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_not_or_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64NotOr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_not_or_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64NotOr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_lt_rrs(rhs: Slot) -> Self {
        Self::I64Lt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_lt_rri(rhs: i64) -> Self {
        Self::I64Lt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_lt_rsr(lhs: Slot) -> Self {
        Self::I64Lt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Lt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_lt_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64Lt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_lt_rir(lhs: i64) -> Self {
        Self::I64Lt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_lt_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Lt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_le_rrs(rhs: Slot) -> Self {
        Self::I64Le_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_le_rri(rhs: i64) -> Self {
        Self::I64Le_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_le_rsr(lhs: Slot) -> Self {
        Self::I64Le_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Le_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_le_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64Le_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_le_rir(lhs: i64) -> Self {
        Self::I64Le_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_le_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Le_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_lt_rrs(rhs: Slot) -> Self {
        Self::U64Lt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_lt_rri(rhs: u64) -> Self {
        Self::U64Lt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_lt_rsr(lhs: Slot) -> Self {
        Self::U64Lt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U64Lt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_lt_rsi(lhs: Slot, rhs: u64) -> Self {
        Self::U64Lt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_lt_rir(lhs: u64) -> Self {
        Self::U64Lt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_lt_ris(lhs: u64, rhs: Slot) -> Self {
        Self::U64Lt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_le_rrs(rhs: Slot) -> Self {
        Self::U64Le_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_le_rri(rhs: u64) -> Self {
        Self::U64Le_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_le_rsr(lhs: Slot) -> Self {
        Self::U64Le_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U64Le_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_le_rsi(lhs: Slot, rhs: u64) -> Self {
        Self::U64Le_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_le_rir(lhs: u64) -> Self {
        Self::U64Le_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_le_ris(lhs: u64, rhs: Slot) -> Self {
        Self::U64Le_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_eq_rrs(rhs: Slot) -> Self {
        Self::F32Eq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_eq_rri(rhs: f32) -> Self {
        Self::F32Eq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Eq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_eq_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Eq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_lt_rrs(rhs: Slot) -> Self {
        Self::F32Lt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_lt_rri(rhs: f32) -> Self {
        Self::F32Lt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_lt_rsr(lhs: Slot) -> Self {
        Self::F32Lt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Lt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_lt_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Lt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_lt_rir(lhs: f32) -> Self {
        Self::F32Lt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_lt_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Lt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_le_rrs(rhs: Slot) -> Self {
        Self::F32Le_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_le_rri(rhs: f32) -> Self {
        Self::F32Le_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_le_rsr(lhs: Slot) -> Self {
        Self::F32Le_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Le_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_le_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Le_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_le_rir(lhs: f32) -> Self {
        Self::F32Le_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_le_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Le_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_eq_rrs(rhs: Slot) -> Self {
        Self::F32NotEq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_not_eq_rri(rhs: f32) -> Self {
        Self::F32NotEq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_not_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32NotEq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_eq_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32NotEq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_lt_rrs(rhs: Slot) -> Self {
        Self::F32NotLt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_not_lt_rri(rhs: f32) -> Self {
        Self::F32NotLt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_not_lt_rsr(lhs: Slot) -> Self {
        Self::F32NotLt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_not_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32NotLt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_lt_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32NotLt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_lt_rir(lhs: f32) -> Self {
        Self::F32NotLt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_not_lt_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32NotLt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_le_rrs(rhs: Slot) -> Self {
        Self::F32NotLe_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_not_le_rri(rhs: f32) -> Self {
        Self::F32NotLe_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_not_le_rsr(lhs: Slot) -> Self {
        Self::F32NotLe_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_not_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32NotLe_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_le_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32NotLe_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_not_le_rir(lhs: f32) -> Self {
        Self::F32NotLe_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_not_le_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32NotLe_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_eq_rrs(rhs: Slot) -> Self {
        Self::F64Eq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_eq_rri(rhs: f64) -> Self {
        Self::F64Eq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Eq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_eq_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Eq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_lt_rrs(rhs: Slot) -> Self {
        Self::F64Lt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_lt_rri(rhs: f64) -> Self {
        Self::F64Lt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_lt_rsr(lhs: Slot) -> Self {
        Self::F64Lt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Lt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_lt_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Lt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_lt_rir(lhs: f64) -> Self {
        Self::F64Lt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_lt_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Lt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_le_rrs(rhs: Slot) -> Self {
        Self::F64Le_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_le_rri(rhs: f64) -> Self {
        Self::F64Le_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_le_rsr(lhs: Slot) -> Self {
        Self::F64Le_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Le_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_le_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Le_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_le_rir(lhs: f64) -> Self {
        Self::F64Le_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_le_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Le_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_eq_rrs(rhs: Slot) -> Self {
        Self::F64NotEq_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_not_eq_rri(rhs: f64) -> Self {
        Self::F64NotEq_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_not_eq_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64NotEq_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_eq_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64NotEq_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_lt_rrs(rhs: Slot) -> Self {
        Self::F64NotLt_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_not_lt_rri(rhs: f64) -> Self {
        Self::F64NotLt_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_not_lt_rsr(lhs: Slot) -> Self {
        Self::F64NotLt_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_not_lt_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64NotLt_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_lt_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64NotLt_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_lt_rir(lhs: f64) -> Self {
        Self::F64NotLt_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_not_lt_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64NotLt_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_le_rrs(rhs: Slot) -> Self {
        Self::F64NotLe_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_not_le_rri(rhs: f64) -> Self {
        Self::F64NotLe_Rri { result: <Reg<i64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_not_le_rsr(lhs: Slot) -> Self {
        Self::F64NotLe_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_not_le_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64NotLe_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_le_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64NotLe_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f64_not_le_rir(lhs: f64) -> Self {
        Self::F64NotLe_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_not_le_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64NotLe_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_add_rrs(rhs: Slot) -> Self {
        Self::I32Add_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_add_rri(rhs: i32) -> Self {
        Self::I32Add_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_add_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Add_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_add_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32Add_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_add_rs_rs(result: SlotAndReg<i64>, rhs: Slot) -> Self {
        Self::I32Add_Rs_rs { result, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_add_rs_ri(result: SlotAndReg<i64>, rhs: i32) -> Self {
        Self::I32Add_Rs_ri { result, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_add_rs_ss(result: SlotAndReg<i64>, lhs: Slot, rhs: Slot) -> Self {
        Self::I32Add_Rs_ss { result, lhs, rhs }
    }
    pub fn i32_add_rs_si(result: SlotAndReg<i64>, lhs: Slot, rhs: i32) -> Self {
        Self::I32Add_Rs_si { result, lhs, rhs }
    }
    pub fn i32_sub_rrs(rhs: Slot) -> Self {
        Self::I32Sub_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_sub_rsr(lhs: Slot) -> Self {
        Self::I32Sub_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_sub_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Sub_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_sub_rir(lhs: i32) -> Self {
        Self::I32Sub_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_sub_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Sub_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_mul_rrs(rhs: Slot) -> Self {
        Self::I32Mul_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_mul_rri(rhs: i32) -> Self {
        Self::I32Mul_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_mul_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Mul_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_mul_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32Mul_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_div_rrs(rhs: Slot) -> Self {
        Self::I32Div_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_div_rri(rhs: NonZero<i32>) -> Self {
        Self::I32Div_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_div_rsr(lhs: Slot) -> Self {
        Self::I32Div_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_div_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Div_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_div_rsi(lhs: Slot, rhs: NonZero<i32>) -> Self {
        Self::I32Div_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_div_rir(lhs: i32) -> Self {
        Self::I32Div_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_div_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Div_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_div_rrs(rhs: Slot) -> Self {
        Self::U32Div_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_div_rri(rhs: NonZero<u32>) -> Self {
        Self::U32Div_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_div_rsr(lhs: Slot) -> Self {
        Self::U32Div_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_div_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U32Div_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_div_rsi(lhs: Slot, rhs: NonZero<u32>) -> Self {
        Self::U32Div_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_div_rir(lhs: u32) -> Self {
        Self::U32Div_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_div_ris(lhs: u32, rhs: Slot) -> Self {
        Self::U32Div_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rem_rrs(rhs: Slot) -> Self {
        Self::I32Rem_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_rem_rri(rhs: NonZero<i32>) -> Self {
        Self::I32Rem_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_rem_rsr(lhs: Slot) -> Self {
        Self::I32Rem_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_rem_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Rem_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rem_rsi(lhs: Slot, rhs: NonZero<i32>) -> Self {
        Self::I32Rem_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rem_rir(lhs: i32) -> Self {
        Self::I32Rem_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_rem_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Rem_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_rem_rrs(rhs: Slot) -> Self {
        Self::U32Rem_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_rem_rri(rhs: NonZero<u32>) -> Self {
        Self::U32Rem_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_rem_rsr(lhs: Slot) -> Self {
        Self::U32Rem_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_rem_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U32Rem_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_rem_rsi(lhs: Slot, rhs: NonZero<u32>) -> Self {
        Self::U32Rem_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_rem_rir(lhs: u32) -> Self {
        Self::U32Rem_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_rem_ris(lhs: u32, rhs: Slot) -> Self {
        Self::U32Rem_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_bitand_rrs(rhs: Slot) -> Self {
        Self::I32BitAnd_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_bitand_rri(rhs: i32) -> Self {
        Self::I32BitAnd_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_bitand_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32BitAnd_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_bitand_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32BitAnd_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_bitor_rrs(rhs: Slot) -> Self {
        Self::I32BitOr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_bitor_rri(rhs: i32) -> Self {
        Self::I32BitOr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_bitor_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32BitOr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_bitor_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32BitOr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_bitxor_rrs(rhs: Slot) -> Self {
        Self::I32BitXor_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_bitxor_rri(rhs: i32) -> Self {
        Self::I32BitXor_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_bitxor_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32BitXor_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_bitxor_rsi(lhs: Slot, rhs: i32) -> Self {
        Self::I32BitXor_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_shl_rrs(rhs: Slot) -> Self {
        Self::I32Shl_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_shl_rri(rhs: ShiftAmount) -> Self {
        Self::I32Shl_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_shl_rsr(lhs: Slot) -> Self {
        Self::I32Shl_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_shl_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Shl_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_shl_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I32Shl_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_shl_rir(lhs: i32) -> Self {
        Self::I32Shl_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_shl_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Shl_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_shr_rrs(rhs: Slot) -> Self {
        Self::I32Shr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_shr_rri(rhs: ShiftAmount) -> Self {
        Self::I32Shr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_shr_rsr(lhs: Slot) -> Self {
        Self::I32Shr_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_shr_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Shr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_shr_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I32Shr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_shr_rir(lhs: i32) -> Self {
        Self::I32Shr_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_shr_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Shr_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_shr_rrs(rhs: Slot) -> Self {
        Self::U32Shr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_shr_rri(rhs: ShiftAmount) -> Self {
        Self::U32Shr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u32_shr_rsr(lhs: Slot) -> Self {
        Self::U32Shr_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_shr_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U32Shr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_shr_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::U32Shr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u32_shr_rir(lhs: u32) -> Self {
        Self::U32Shr_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u32_shr_ris(lhs: u32, rhs: Slot) -> Self {
        Self::U32Shr_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rotl_rrs(rhs: Slot) -> Self {
        Self::I32Rotl_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_rotl_rri(rhs: ShiftAmount) -> Self {
        Self::I32Rotl_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_rotl_rsr(lhs: Slot) -> Self {
        Self::I32Rotl_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_rotl_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Rotl_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rotl_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I32Rotl_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rotl_rir(lhs: i32) -> Self {
        Self::I32Rotl_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_rotl_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Rotl_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rotr_rrs(rhs: Slot) -> Self {
        Self::I32Rotr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_rotr_rri(rhs: ShiftAmount) -> Self {
        Self::I32Rotr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i32_rotr_rsr(lhs: Slot) -> Self {
        Self::I32Rotr_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_rotr_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I32Rotr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rotr_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I32Rotr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i32_rotr_rir(lhs: i32) -> Self {
        Self::I32Rotr_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i32_rotr_ris(lhs: i32, rhs: Slot) -> Self {
        Self::I32Rotr_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_add_rrs(rhs: Slot) -> Self {
        Self::I64Add_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_add_rri(rhs: i64) -> Self {
        Self::I64Add_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_add_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Add_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_add_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64Add_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_add_rs_rs(result: SlotAndReg<i64>, rhs: Slot) -> Self {
        Self::I64Add_Rs_rs { result, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_add_rs_ri(result: SlotAndReg<i64>, rhs: i64) -> Self {
        Self::I64Add_Rs_ri { result, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_add_rs_ss(result: SlotAndReg<i64>, lhs: Slot, rhs: Slot) -> Self {
        Self::I64Add_Rs_ss { result, lhs, rhs }
    }
    pub fn i64_add_rs_si(result: SlotAndReg<i64>, lhs: Slot, rhs: i64) -> Self {
        Self::I64Add_Rs_si { result, lhs, rhs }
    }
    pub fn i64_sub_rrs(rhs: Slot) -> Self {
        Self::I64Sub_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_sub_rsr(lhs: Slot) -> Self {
        Self::I64Sub_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_sub_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Sub_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_sub_rir(lhs: i64) -> Self {
        Self::I64Sub_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_sub_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Sub_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_mul_rrs(rhs: Slot) -> Self {
        Self::I64Mul_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_mul_rri(rhs: i64) -> Self {
        Self::I64Mul_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_mul_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Mul_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_mul_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64Mul_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_div_rrs(rhs: Slot) -> Self {
        Self::I64Div_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_div_rri(rhs: NonZero<i64>) -> Self {
        Self::I64Div_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_div_rsr(lhs: Slot) -> Self {
        Self::I64Div_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_div_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Div_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_div_rsi(lhs: Slot, rhs: NonZero<i64>) -> Self {
        Self::I64Div_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_div_rir(lhs: i64) -> Self {
        Self::I64Div_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_div_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Div_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_div_rrs(rhs: Slot) -> Self {
        Self::U64Div_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_div_rri(rhs: NonZero<u64>) -> Self {
        Self::U64Div_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_div_rsr(lhs: Slot) -> Self {
        Self::U64Div_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_div_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U64Div_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_div_rsi(lhs: Slot, rhs: NonZero<u64>) -> Self {
        Self::U64Div_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_div_rir(lhs: u64) -> Self {
        Self::U64Div_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_div_ris(lhs: u64, rhs: Slot) -> Self {
        Self::U64Div_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rem_rrs(rhs: Slot) -> Self {
        Self::I64Rem_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_rem_rri(rhs: NonZero<i64>) -> Self {
        Self::I64Rem_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_rem_rsr(lhs: Slot) -> Self {
        Self::I64Rem_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_rem_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Rem_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rem_rsi(lhs: Slot, rhs: NonZero<i64>) -> Self {
        Self::I64Rem_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rem_rir(lhs: i64) -> Self {
        Self::I64Rem_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_rem_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Rem_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_rem_rrs(rhs: Slot) -> Self {
        Self::U64Rem_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_rem_rri(rhs: NonZero<u64>) -> Self {
        Self::U64Rem_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_rem_rsr(lhs: Slot) -> Self {
        Self::U64Rem_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_rem_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U64Rem_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_rem_rsi(lhs: Slot, rhs: NonZero<u64>) -> Self {
        Self::U64Rem_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_rem_rir(lhs: u64) -> Self {
        Self::U64Rem_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_rem_ris(lhs: u64, rhs: Slot) -> Self {
        Self::U64Rem_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_bitand_rrs(rhs: Slot) -> Self {
        Self::I64BitAnd_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_bitand_rri(rhs: i64) -> Self {
        Self::I64BitAnd_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_bitand_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64BitAnd_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_bitand_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64BitAnd_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_bitor_rrs(rhs: Slot) -> Self {
        Self::I64BitOr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_bitor_rri(rhs: i64) -> Self {
        Self::I64BitOr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_bitor_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64BitOr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_bitor_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64BitOr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_bitxor_rrs(rhs: Slot) -> Self {
        Self::I64BitXor_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_bitxor_rri(rhs: i64) -> Self {
        Self::I64BitXor_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_bitxor_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64BitXor_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_bitxor_rsi(lhs: Slot, rhs: i64) -> Self {
        Self::I64BitXor_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_shl_rrs(rhs: Slot) -> Self {
        Self::I64Shl_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_shl_rri(rhs: ShiftAmount) -> Self {
        Self::I64Shl_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_shl_rsr(lhs: Slot) -> Self {
        Self::I64Shl_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_shl_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Shl_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_shl_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I64Shl_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_shl_rir(lhs: i64) -> Self {
        Self::I64Shl_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_shl_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Shl_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_shr_rrs(rhs: Slot) -> Self {
        Self::I64Shr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_shr_rri(rhs: ShiftAmount) -> Self {
        Self::I64Shr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_shr_rsr(lhs: Slot) -> Self {
        Self::I64Shr_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_shr_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Shr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_shr_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I64Shr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_shr_rir(lhs: i64) -> Self {
        Self::I64Shr_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_shr_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Shr_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_shr_rrs(rhs: Slot) -> Self {
        Self::U64Shr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_shr_rri(rhs: ShiftAmount) -> Self {
        Self::U64Shr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn u64_shr_rsr(lhs: Slot) -> Self {
        Self::U64Shr_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_shr_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::U64Shr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_shr_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::U64Shr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn u64_shr_rir(lhs: u64) -> Self {
        Self::U64Shr_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn u64_shr_ris(lhs: u64, rhs: Slot) -> Self {
        Self::U64Shr_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rotl_rrs(rhs: Slot) -> Self {
        Self::I64Rotl_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_rotl_rri(rhs: ShiftAmount) -> Self {
        Self::I64Rotl_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_rotl_rsr(lhs: Slot) -> Self {
        Self::I64Rotl_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_rotl_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Rotl_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rotl_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I64Rotl_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rotl_rir(lhs: i64) -> Self {
        Self::I64Rotl_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_rotl_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Rotl_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rotr_rrs(rhs: Slot) -> Self {
        Self::I64Rotr_Rrs { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_rotr_rri(rhs: ShiftAmount) -> Self {
        Self::I64Rotr_Rri { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn i64_rotr_rsr(lhs: Slot) -> Self {
        Self::I64Rotr_Rsr { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_rotr_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::I64Rotr_Rss { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rotr_rsi(lhs: Slot, rhs: ShiftAmount) -> Self {
        Self::I64Rotr_Rsi { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn i64_rotr_rir(lhs: i64) -> Self {
        Self::I64Rotr_Rir { result: <Reg<i64>>::default(), lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn i64_rotr_ris(lhs: i64, rhs: Slot) -> Self {
        Self::I64Rotr_Ris { result: <Reg<i64>>::default(), lhs, rhs }
    }
    pub fn f32_add_rrs(rhs: Slot) -> Self {
        Self::F32Add_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_add_rri(rhs: f32) -> Self {
        Self::F32Add_Rri { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_add_rsr(lhs: Slot) -> Self {
        Self::F32Add_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_add_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Add_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_add_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Add_Rsi { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_add_rir(lhs: f32) -> Self {
        Self::F32Add_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_add_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Add_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_sub_rrs(rhs: Slot) -> Self {
        Self::F32Sub_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_sub_rri(rhs: f32) -> Self {
        Self::F32Sub_Rri { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_sub_rsr(lhs: Slot) -> Self {
        Self::F32Sub_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_sub_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Sub_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_sub_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Sub_Rsi { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_sub_rir(lhs: f32) -> Self {
        Self::F32Sub_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_sub_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Sub_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_mul_rrs(rhs: Slot) -> Self {
        Self::F32Mul_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_mul_rri(rhs: f32) -> Self {
        Self::F32Mul_Rri { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_mul_rsr(lhs: Slot) -> Self {
        Self::F32Mul_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_mul_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Mul_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_mul_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Mul_Rsi { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_mul_rir(lhs: f32) -> Self {
        Self::F32Mul_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_mul_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Mul_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_div_rrs(rhs: Slot) -> Self {
        Self::F32Div_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_div_rri(rhs: f32) -> Self {
        Self::F32Div_Rri { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_div_rsr(lhs: Slot) -> Self {
        Self::F32Div_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_div_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Div_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_div_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Div_Rsi { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_div_rir(lhs: f32) -> Self {
        Self::F32Div_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_div_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Div_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_min_rrs(rhs: Slot) -> Self {
        Self::F32Min_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_min_rri(rhs: f32) -> Self {
        Self::F32Min_Rri { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_min_rsr(lhs: Slot) -> Self {
        Self::F32Min_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_min_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Min_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_min_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Min_Rsi { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_min_rir(lhs: f32) -> Self {
        Self::F32Min_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_min_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Min_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_max_rrs(rhs: Slot) -> Self {
        Self::F32Max_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_max_rri(rhs: f32) -> Self {
        Self::F32Max_Rri { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_max_rsr(lhs: Slot) -> Self {
        Self::F32Max_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_max_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Max_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_max_rsi(lhs: Slot, rhs: f32) -> Self {
        Self::F32Max_Rsi { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_max_rir(lhs: f32) -> Self {
        Self::F32Max_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_max_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Max_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f64_add_rrs(rhs: Slot) -> Self {
        Self::F64Add_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_add_rri(rhs: f64) -> Self {
        Self::F64Add_Rri { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_add_rsr(lhs: Slot) -> Self {
        Self::F64Add_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_add_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Add_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_add_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Add_Rsi { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_add_rir(lhs: f64) -> Self {
        Self::F64Add_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_add_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Add_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_sub_rrs(rhs: Slot) -> Self {
        Self::F64Sub_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_sub_rri(rhs: f64) -> Self {
        Self::F64Sub_Rri { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_sub_rsr(lhs: Slot) -> Self {
        Self::F64Sub_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_sub_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Sub_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_sub_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Sub_Rsi { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_sub_rir(lhs: f64) -> Self {
        Self::F64Sub_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_sub_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Sub_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_mul_rrs(rhs: Slot) -> Self {
        Self::F64Mul_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_mul_rri(rhs: f64) -> Self {
        Self::F64Mul_Rri { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_mul_rsr(lhs: Slot) -> Self {
        Self::F64Mul_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_mul_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Mul_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_mul_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Mul_Rsi { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_mul_rir(lhs: f64) -> Self {
        Self::F64Mul_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_mul_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Mul_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_div_rrs(rhs: Slot) -> Self {
        Self::F64Div_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_div_rri(rhs: f64) -> Self {
        Self::F64Div_Rri { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_div_rsr(lhs: Slot) -> Self {
        Self::F64Div_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_div_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Div_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_div_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Div_Rsi { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_div_rir(lhs: f64) -> Self {
        Self::F64Div_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_div_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Div_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_min_rrs(rhs: Slot) -> Self {
        Self::F64Min_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_min_rri(rhs: f64) -> Self {
        Self::F64Min_Rri { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_min_rsr(lhs: Slot) -> Self {
        Self::F64Min_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_min_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Min_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_min_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Min_Rsi { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_min_rir(lhs: f64) -> Self {
        Self::F64Min_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_min_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Min_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_max_rrs(rhs: Slot) -> Self {
        Self::F64Max_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_max_rri(rhs: f64) -> Self {
        Self::F64Max_Rri { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_max_rsr(lhs: Slot) -> Self {
        Self::F64Max_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_max_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Max_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_max_rsi(lhs: Slot, rhs: f64) -> Self {
        Self::F64Max_Rsi { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_max_rir(lhs: f64) -> Self {
        Self::F64Max_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_max_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Max_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f32_copysign_rrs(rhs: Slot) -> Self {
        Self::F32Copysign_Rrs { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn f32_copysign_rsr(lhs: Slot) -> Self {
        Self::F32Copysign_Rsr { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_copysign_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F32Copysign_Rss { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f32_copysign_rir(lhs: f32) -> Self {
        Self::F32Copysign_Rir { result: <Reg<f32>>::default(), lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn f32_copysign_ris(lhs: f32, rhs: Slot) -> Self {
        Self::F32Copysign_Ris { result: <Reg<f32>>::default(), lhs, rhs }
    }
    pub fn f64_copysign_rrs(rhs: Slot) -> Self {
        Self::F64Copysign_Rrs { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn f64_copysign_rsr(lhs: Slot) -> Self {
        Self::F64Copysign_Rsr { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_copysign_rss(lhs: Slot, rhs: Slot) -> Self {
        Self::F64Copysign_Rss { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn f64_copysign_rir(lhs: f64) -> Self {
        Self::F64Copysign_Rir { result: <Reg<f64>>::default(), lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn f64_copysign_ris(lhs: f64, rhs: Slot) -> Self {
        Self::F64Copysign_Ris { result: <Reg<f64>>::default(), lhs, rhs }
    }
    pub fn i32_mul_rrr() -> Self {
        Self::I32Mul_Rrr { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs: <Reg<i64>>::default() }
    }
    pub fn i64_mul_rrr() -> Self {
        Self::I64Mul_Rrr { result: <Reg<i64>>::default(), lhs: <Reg<i64>>::default(), rhs: <Reg<i64>>::default() }
    }
    pub fn f32_mul_rrr() -> Self {
        Self::F32Mul_Rrr { result: <Reg<f32>>::default(), lhs: <Reg<f32>>::default(), rhs: <Reg<f32>>::default() }
    }
    pub fn f64_mul_rrr() -> Self {
        Self::F64Mul_Rrr { result: <Reg<f64>>::default(), lhs: <Reg<f64>>::default(), rhs: <Reg<f64>>::default() }
    }
    pub fn branch_i32_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32Eq_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_eq_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32Eq_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32Eq_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_eq_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32Eq_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_not_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32NotEq_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_not_eq_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32NotEq_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_not_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32NotEq_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_not_eq_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32NotEq_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_and_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32And_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_and_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32And_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_and_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32And_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_and_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32And_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_not_and_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32NotAnd_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_not_and_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32NotAnd_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_not_and_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32NotAnd_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_not_and_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32NotAnd_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_or_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32Or_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_or_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32Or_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_or_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32Or_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_or_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32Or_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_not_or_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32NotOr_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_not_or_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32NotOr_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_not_or_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32NotOr_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_not_or_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32NotOr_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32Lt_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_lt_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32Lt_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchI32Lt_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i32_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32Lt_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_lt_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32Lt_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_lt_ir(offset: BranchOffset, lhs: i32) -> Self {
        Self::BranchI32Lt_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i32_lt_is(offset: BranchOffset, lhs: i32, rhs: Slot) -> Self {
        Self::BranchI32Lt_Is { offset, lhs, rhs }
    }
    pub fn branch_i32_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI32Le_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_le_ri(offset: BranchOffset, rhs: i32) -> Self {
        Self::BranchI32Le_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i32_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchI32Le_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i32_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI32Le_Ss { offset, lhs, rhs }
    }
    pub fn branch_i32_le_si(offset: BranchOffset, lhs: Slot, rhs: i32) -> Self {
        Self::BranchI32Le_Si { offset, lhs, rhs }
    }
    pub fn branch_i32_le_ir(offset: BranchOffset, lhs: i32) -> Self {
        Self::BranchI32Le_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i32_le_is(offset: BranchOffset, lhs: i32, rhs: Slot) -> Self {
        Self::BranchI32Le_Is { offset, lhs, rhs }
    }
    pub fn branch_u32_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchU32Lt_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u32_lt_ri(offset: BranchOffset, rhs: u32) -> Self {
        Self::BranchU32Lt_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u32_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchU32Lt_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u32_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchU32Lt_Ss { offset, lhs, rhs }
    }
    pub fn branch_u32_lt_si(offset: BranchOffset, lhs: Slot, rhs: u32) -> Self {
        Self::BranchU32Lt_Si { offset, lhs, rhs }
    }
    pub fn branch_u32_lt_ir(offset: BranchOffset, lhs: u32) -> Self {
        Self::BranchU32Lt_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u32_lt_is(offset: BranchOffset, lhs: u32, rhs: Slot) -> Self {
        Self::BranchU32Lt_Is { offset, lhs, rhs }
    }
    pub fn branch_u32_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchU32Le_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u32_le_ri(offset: BranchOffset, rhs: u32) -> Self {
        Self::BranchU32Le_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u32_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchU32Le_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u32_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchU32Le_Ss { offset, lhs, rhs }
    }
    pub fn branch_u32_le_si(offset: BranchOffset, lhs: Slot, rhs: u32) -> Self {
        Self::BranchU32Le_Si { offset, lhs, rhs }
    }
    pub fn branch_u32_le_ir(offset: BranchOffset, lhs: u32) -> Self {
        Self::BranchU32Le_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u32_le_is(offset: BranchOffset, lhs: u32, rhs: Slot) -> Self {
        Self::BranchU32Le_Is { offset, lhs, rhs }
    }
    pub fn branch_i64_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64Eq_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_eq_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64Eq_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64Eq_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_eq_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64Eq_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_not_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64NotEq_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_not_eq_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64NotEq_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_not_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64NotEq_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_not_eq_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64NotEq_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_and_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64And_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_and_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64And_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_and_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64And_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_and_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64And_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_not_and_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64NotAnd_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_not_and_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64NotAnd_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_not_and_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64NotAnd_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_not_and_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64NotAnd_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_or_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64Or_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_or_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64Or_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_or_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64Or_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_or_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64Or_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_not_or_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64NotOr_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_not_or_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64NotOr_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_not_or_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64NotOr_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_not_or_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64NotOr_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64Lt_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_lt_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64Lt_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchI64Lt_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i64_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64Lt_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_lt_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64Lt_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_lt_ir(offset: BranchOffset, lhs: i64) -> Self {
        Self::BranchI64Lt_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i64_lt_is(offset: BranchOffset, lhs: i64, rhs: Slot) -> Self {
        Self::BranchI64Lt_Is { offset, lhs, rhs }
    }
    pub fn branch_i64_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchI64Le_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_le_ri(offset: BranchOffset, rhs: i64) -> Self {
        Self::BranchI64Le_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_i64_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchI64Le_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i64_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchI64Le_Ss { offset, lhs, rhs }
    }
    pub fn branch_i64_le_si(offset: BranchOffset, lhs: Slot, rhs: i64) -> Self {
        Self::BranchI64Le_Si { offset, lhs, rhs }
    }
    pub fn branch_i64_le_ir(offset: BranchOffset, lhs: i64) -> Self {
        Self::BranchI64Le_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_i64_le_is(offset: BranchOffset, lhs: i64, rhs: Slot) -> Self {
        Self::BranchI64Le_Is { offset, lhs, rhs }
    }
    pub fn branch_u64_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchU64Lt_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u64_lt_ri(offset: BranchOffset, rhs: u64) -> Self {
        Self::BranchU64Lt_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u64_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchU64Lt_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u64_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchU64Lt_Ss { offset, lhs, rhs }
    }
    pub fn branch_u64_lt_si(offset: BranchOffset, lhs: Slot, rhs: u64) -> Self {
        Self::BranchU64Lt_Si { offset, lhs, rhs }
    }
    pub fn branch_u64_lt_ir(offset: BranchOffset, lhs: u64) -> Self {
        Self::BranchU64Lt_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u64_lt_is(offset: BranchOffset, lhs: u64, rhs: Slot) -> Self {
        Self::BranchU64Lt_Is { offset, lhs, rhs }
    }
    pub fn branch_u64_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchU64Le_Rs { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u64_le_ri(offset: BranchOffset, rhs: u64) -> Self {
        Self::BranchU64Le_Ri { offset, lhs: <Reg<i64>>::default(), rhs }
    }
    pub fn branch_u64_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchU64Le_Sr { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u64_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchU64Le_Ss { offset, lhs, rhs }
    }
    pub fn branch_u64_le_si(offset: BranchOffset, lhs: Slot, rhs: u64) -> Self {
        Self::BranchU64Le_Si { offset, lhs, rhs }
    }
    pub fn branch_u64_le_ir(offset: BranchOffset, lhs: u64) -> Self {
        Self::BranchU64Le_Ir { offset, lhs, rhs: <Reg<i64>>::default() }
    }
    pub fn branch_u64_le_is(offset: BranchOffset, lhs: u64, rhs: Slot) -> Self {
        Self::BranchU64Le_Is { offset, lhs, rhs }
    }
    pub fn branch_f32_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF32Eq_Rs { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_eq_ri(offset: BranchOffset, rhs: f32) -> Self {
        Self::BranchF32Eq_Ri { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF32Eq_Ss { offset, lhs, rhs }
    }
    pub fn branch_f32_eq_si(offset: BranchOffset, lhs: Slot, rhs: f32) -> Self {
        Self::BranchF32Eq_Si { offset, lhs, rhs }
    }
    pub fn branch_f32_not_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF32NotEq_Rs { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_not_eq_ri(offset: BranchOffset, rhs: f32) -> Self {
        Self::BranchF32NotEq_Ri { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_not_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF32NotEq_Ss { offset, lhs, rhs }
    }
    pub fn branch_f32_not_eq_si(offset: BranchOffset, lhs: Slot, rhs: f32) -> Self {
        Self::BranchF32NotEq_Si { offset, lhs, rhs }
    }
    pub fn branch_f32_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF32Lt_Rs { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_lt_ri(offset: BranchOffset, rhs: f32) -> Self {
        Self::BranchF32Lt_Ri { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF32Lt_Sr { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF32Lt_Ss { offset, lhs, rhs }
    }
    pub fn branch_f32_lt_si(offset: BranchOffset, lhs: Slot, rhs: f32) -> Self {
        Self::BranchF32Lt_Si { offset, lhs, rhs }
    }
    pub fn branch_f32_lt_ir(offset: BranchOffset, lhs: f32) -> Self {
        Self::BranchF32Lt_Ir { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_lt_is(offset: BranchOffset, lhs: f32, rhs: Slot) -> Self {
        Self::BranchF32Lt_Is { offset, lhs, rhs }
    }
    pub fn branch_f32_not_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF32NotLt_Rs { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_not_lt_ri(offset: BranchOffset, rhs: f32) -> Self {
        Self::BranchF32NotLt_Ri { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_not_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF32NotLt_Sr { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_not_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF32NotLt_Ss { offset, lhs, rhs }
    }
    pub fn branch_f32_not_lt_si(offset: BranchOffset, lhs: Slot, rhs: f32) -> Self {
        Self::BranchF32NotLt_Si { offset, lhs, rhs }
    }
    pub fn branch_f32_not_lt_ir(offset: BranchOffset, lhs: f32) -> Self {
        Self::BranchF32NotLt_Ir { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_not_lt_is(offset: BranchOffset, lhs: f32, rhs: Slot) -> Self {
        Self::BranchF32NotLt_Is { offset, lhs, rhs }
    }
    pub fn branch_f32_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF32Le_Rs { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_le_ri(offset: BranchOffset, rhs: f32) -> Self {
        Self::BranchF32Le_Ri { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF32Le_Sr { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF32Le_Ss { offset, lhs, rhs }
    }
    pub fn branch_f32_le_si(offset: BranchOffset, lhs: Slot, rhs: f32) -> Self {
        Self::BranchF32Le_Si { offset, lhs, rhs }
    }
    pub fn branch_f32_le_ir(offset: BranchOffset, lhs: f32) -> Self {
        Self::BranchF32Le_Ir { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_le_is(offset: BranchOffset, lhs: f32, rhs: Slot) -> Self {
        Self::BranchF32Le_Is { offset, lhs, rhs }
    }
    pub fn branch_f32_not_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF32NotLe_Rs { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_not_le_ri(offset: BranchOffset, rhs: f32) -> Self {
        Self::BranchF32NotLe_Ri { offset, lhs: <Reg<f32>>::default(), rhs }
    }
    pub fn branch_f32_not_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF32NotLe_Sr { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_not_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF32NotLe_Ss { offset, lhs, rhs }
    }
    pub fn branch_f32_not_le_si(offset: BranchOffset, lhs: Slot, rhs: f32) -> Self {
        Self::BranchF32NotLe_Si { offset, lhs, rhs }
    }
    pub fn branch_f32_not_le_ir(offset: BranchOffset, lhs: f32) -> Self {
        Self::BranchF32NotLe_Ir { offset, lhs, rhs: <Reg<f32>>::default() }
    }
    pub fn branch_f32_not_le_is(offset: BranchOffset, lhs: f32, rhs: Slot) -> Self {
        Self::BranchF32NotLe_Is { offset, lhs, rhs }
    }
    pub fn branch_f64_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF64Eq_Rs { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_eq_ri(offset: BranchOffset, rhs: f64) -> Self {
        Self::BranchF64Eq_Ri { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF64Eq_Ss { offset, lhs, rhs }
    }
    pub fn branch_f64_eq_si(offset: BranchOffset, lhs: Slot, rhs: f64) -> Self {
        Self::BranchF64Eq_Si { offset, lhs, rhs }
    }
    pub fn branch_f64_not_eq_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF64NotEq_Rs { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_not_eq_ri(offset: BranchOffset, rhs: f64) -> Self {
        Self::BranchF64NotEq_Ri { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_not_eq_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF64NotEq_Ss { offset, lhs, rhs }
    }
    pub fn branch_f64_not_eq_si(offset: BranchOffset, lhs: Slot, rhs: f64) -> Self {
        Self::BranchF64NotEq_Si { offset, lhs, rhs }
    }
    pub fn branch_f64_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF64Lt_Rs { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_lt_ri(offset: BranchOffset, rhs: f64) -> Self {
        Self::BranchF64Lt_Ri { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF64Lt_Sr { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF64Lt_Ss { offset, lhs, rhs }
    }
    pub fn branch_f64_lt_si(offset: BranchOffset, lhs: Slot, rhs: f64) -> Self {
        Self::BranchF64Lt_Si { offset, lhs, rhs }
    }
    pub fn branch_f64_lt_ir(offset: BranchOffset, lhs: f64) -> Self {
        Self::BranchF64Lt_Ir { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_lt_is(offset: BranchOffset, lhs: f64, rhs: Slot) -> Self {
        Self::BranchF64Lt_Is { offset, lhs, rhs }
    }
    pub fn branch_f64_not_lt_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF64NotLt_Rs { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_not_lt_ri(offset: BranchOffset, rhs: f64) -> Self {
        Self::BranchF64NotLt_Ri { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_not_lt_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF64NotLt_Sr { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_not_lt_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF64NotLt_Ss { offset, lhs, rhs }
    }
    pub fn branch_f64_not_lt_si(offset: BranchOffset, lhs: Slot, rhs: f64) -> Self {
        Self::BranchF64NotLt_Si { offset, lhs, rhs }
    }
    pub fn branch_f64_not_lt_ir(offset: BranchOffset, lhs: f64) -> Self {
        Self::BranchF64NotLt_Ir { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_not_lt_is(offset: BranchOffset, lhs: f64, rhs: Slot) -> Self {
        Self::BranchF64NotLt_Is { offset, lhs, rhs }
    }
    pub fn branch_f64_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF64Le_Rs { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_le_ri(offset: BranchOffset, rhs: f64) -> Self {
        Self::BranchF64Le_Ri { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF64Le_Sr { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF64Le_Ss { offset, lhs, rhs }
    }
    pub fn branch_f64_le_si(offset: BranchOffset, lhs: Slot, rhs: f64) -> Self {
        Self::BranchF64Le_Si { offset, lhs, rhs }
    }
    pub fn branch_f64_le_ir(offset: BranchOffset, lhs: f64) -> Self {
        Self::BranchF64Le_Ir { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_le_is(offset: BranchOffset, lhs: f64, rhs: Slot) -> Self {
        Self::BranchF64Le_Is { offset, lhs, rhs }
    }
    pub fn branch_f64_not_le_rs(offset: BranchOffset, rhs: Slot) -> Self {
        Self::BranchF64NotLe_Rs { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_not_le_ri(offset: BranchOffset, rhs: f64) -> Self {
        Self::BranchF64NotLe_Ri { offset, lhs: <Reg<f64>>::default(), rhs }
    }
    pub fn branch_f64_not_le_sr(offset: BranchOffset, lhs: Slot) -> Self {
        Self::BranchF64NotLe_Sr { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_not_le_ss(offset: BranchOffset, lhs: Slot, rhs: Slot) -> Self {
        Self::BranchF64NotLe_Ss { offset, lhs, rhs }
    }
    pub fn branch_f64_not_le_si(offset: BranchOffset, lhs: Slot, rhs: f64) -> Self {
        Self::BranchF64NotLe_Si { offset, lhs, rhs }
    }
    pub fn branch_f64_not_le_ir(offset: BranchOffset, lhs: f64) -> Self {
        Self::BranchF64NotLe_Ir { offset, lhs, rhs: <Reg<f64>>::default() }
    }
    pub fn branch_f64_not_le_is(offset: BranchOffset, lhs: f64, rhs: Slot) -> Self {
        Self::BranchF64NotLe_Is { offset, lhs, rhs }
    }
    pub fn u32_select_rrri(false_val: u32) -> Self {
        Self::U32Select_Rrri { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<i64>>::default(), false_val }
    }
    pub fn u32_select_rrsi(true_val: Slot, false_val: u32) -> Self {
        Self::U32Select_Rrsi { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u32_select_rrir(true_val: u32) -> Self {
        Self::U32Select_Rrir { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<i64>>::default() }
    }
    pub fn u32_select_rris(true_val: u32, false_val: Slot) -> Self {
        Self::U32Select_Rris { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u32_select_rrii(true_val: u32, false_val: u32) -> Self {
        Self::U32Select_Rrii { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u32_select_rsri(condition: Slot, false_val: u32) -> Self {
        Self::U32Select_Rsri { result: <Reg<i64>>::default(), condition, true_val: <Reg<i64>>::default(), false_val }
    }
    pub fn u32_select_rssi(condition: Slot, true_val: Slot, false_val: u32) -> Self {
        Self::U32Select_Rssi { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn u32_select_rsir(condition: Slot, true_val: u32) -> Self {
        Self::U32Select_Rsir { result: <Reg<i64>>::default(), condition, true_val, false_val: <Reg<i64>>::default() }
    }
    pub fn u32_select_rsis(condition: Slot, true_val: u32, false_val: Slot) -> Self {
        Self::U32Select_Rsis { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn u32_select_rsii(condition: Slot, true_val: u32, false_val: u32) -> Self {
        Self::U32Select_Rsii { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn u64_select_rrrs(false_val: Slot) -> Self {
        Self::U64Select_Rrrs { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<i64>>::default(), false_val }
    }
    pub fn u64_select_rrri(false_val: u64) -> Self {
        Self::U64Select_Rrri { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<i64>>::default(), false_val }
    }
    pub fn u64_select_rrsr(true_val: Slot) -> Self {
        Self::U64Select_Rrsr { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<i64>>::default() }
    }
    pub fn u64_select_rrss(true_val: Slot, false_val: Slot) -> Self {
        Self::U64Select_Rrss { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u64_select_rrsi(true_val: Slot, false_val: u64) -> Self {
        Self::U64Select_Rrsi { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u64_select_rrir(true_val: u64) -> Self {
        Self::U64Select_Rrir { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<i64>>::default() }
    }
    pub fn u64_select_rris(true_val: u64, false_val: Slot) -> Self {
        Self::U64Select_Rris { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u64_select_rrii(true_val: u64, false_val: u64) -> Self {
        Self::U64Select_Rrii { result: <Reg<i64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn u64_select_rsrs(condition: Slot, false_val: Slot) -> Self {
        Self::U64Select_Rsrs { result: <Reg<i64>>::default(), condition, true_val: <Reg<i64>>::default(), false_val }
    }
    pub fn u64_select_rsri(condition: Slot, false_val: u64) -> Self {
        Self::U64Select_Rsri { result: <Reg<i64>>::default(), condition, true_val: <Reg<i64>>::default(), false_val }
    }
    pub fn u64_select_rssr(condition: Slot, true_val: Slot) -> Self {
        Self::U64Select_Rssr { result: <Reg<i64>>::default(), condition, true_val, false_val: <Reg<i64>>::default() }
    }
    pub fn u64_select_rsss(condition: Slot, true_val: Slot, false_val: Slot) -> Self {
        Self::U64Select_Rsss { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn u64_select_rssi(condition: Slot, true_val: Slot, false_val: u64) -> Self {
        Self::U64Select_Rssi { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn u64_select_rsir(condition: Slot, true_val: u64) -> Self {
        Self::U64Select_Rsir { result: <Reg<i64>>::default(), condition, true_val, false_val: <Reg<i64>>::default() }
    }
    pub fn u64_select_rsis(condition: Slot, true_val: u64, false_val: Slot) -> Self {
        Self::U64Select_Rsis { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn u64_select_rsii(condition: Slot, true_val: u64, false_val: u64) -> Self {
        Self::U64Select_Rsii { result: <Reg<i64>>::default(), condition, true_val, false_val }
    }
    pub fn f32_select_rrrs(false_val: Slot) -> Self {
        Self::F32Select_Rrrs { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<f32>>::default(), false_val }
    }
    pub fn f32_select_rrri(false_val: f32) -> Self {
        Self::F32Select_Rrri { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<f32>>::default(), false_val }
    }
    pub fn f32_select_rrsr(true_val: Slot) -> Self {
        Self::F32Select_Rrsr { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<f32>>::default() }
    }
    pub fn f32_select_rrss(true_val: Slot, false_val: Slot) -> Self {
        Self::F32Select_Rrss { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f32_select_rrsi(true_val: Slot, false_val: f32) -> Self {
        Self::F32Select_Rrsi { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f32_select_rrir(true_val: f32) -> Self {
        Self::F32Select_Rrir { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<f32>>::default() }
    }
    pub fn f32_select_rris(true_val: f32, false_val: Slot) -> Self {
        Self::F32Select_Rris { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f32_select_rrii(true_val: f32, false_val: f32) -> Self {
        Self::F32Select_Rrii { result: <Reg<f32>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f32_select_rsrs(condition: Slot, false_val: Slot) -> Self {
        Self::F32Select_Rsrs { result: <Reg<f32>>::default(), condition, true_val: <Reg<f32>>::default(), false_val }
    }
    pub fn f32_select_rsri(condition: Slot, false_val: f32) -> Self {
        Self::F32Select_Rsri { result: <Reg<f32>>::default(), condition, true_val: <Reg<f32>>::default(), false_val }
    }
    pub fn f32_select_rssr(condition: Slot, true_val: Slot) -> Self {
        Self::F32Select_Rssr { result: <Reg<f32>>::default(), condition, true_val, false_val: <Reg<f32>>::default() }
    }
    pub fn f32_select_rsss(condition: Slot, true_val: Slot, false_val: Slot) -> Self {
        Self::F32Select_Rsss { result: <Reg<f32>>::default(), condition, true_val, false_val }
    }
    pub fn f32_select_rssi(condition: Slot, true_val: Slot, false_val: f32) -> Self {
        Self::F32Select_Rssi { result: <Reg<f32>>::default(), condition, true_val, false_val }
    }
    pub fn f32_select_rsir(condition: Slot, true_val: f32) -> Self {
        Self::F32Select_Rsir { result: <Reg<f32>>::default(), condition, true_val, false_val: <Reg<f32>>::default() }
    }
    pub fn f32_select_rsis(condition: Slot, true_val: f32, false_val: Slot) -> Self {
        Self::F32Select_Rsis { result: <Reg<f32>>::default(), condition, true_val, false_val }
    }
    pub fn f32_select_rsii(condition: Slot, true_val: f32, false_val: f32) -> Self {
        Self::F32Select_Rsii { result: <Reg<f32>>::default(), condition, true_val, false_val }
    }
    pub fn f64_select_rrrs(false_val: Slot) -> Self {
        Self::F64Select_Rrrs { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<f64>>::default(), false_val }
    }
    pub fn f64_select_rrri(false_val: f64) -> Self {
        Self::F64Select_Rrri { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val: <Reg<f64>>::default(), false_val }
    }
    pub fn f64_select_rrsr(true_val: Slot) -> Self {
        Self::F64Select_Rrsr { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<f64>>::default() }
    }
    pub fn f64_select_rrss(true_val: Slot, false_val: Slot) -> Self {
        Self::F64Select_Rrss { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f64_select_rrsi(true_val: Slot, false_val: f64) -> Self {
        Self::F64Select_Rrsi { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f64_select_rrir(true_val: f64) -> Self {
        Self::F64Select_Rrir { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val: <Reg<f64>>::default() }
    }
    pub fn f64_select_rris(true_val: f64, false_val: Slot) -> Self {
        Self::F64Select_Rris { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f64_select_rrii(true_val: f64, false_val: f64) -> Self {
        Self::F64Select_Rrii { result: <Reg<f64>>::default(), condition: <Reg<i64>>::default(), true_val, false_val }
    }
    pub fn f64_select_rsrs(condition: Slot, false_val: Slot) -> Self {
        Self::F64Select_Rsrs { result: <Reg<f64>>::default(), condition, true_val: <Reg<f64>>::default(), false_val }
    }
    pub fn f64_select_rsri(condition: Slot, false_val: f64) -> Self {
        Self::F64Select_Rsri { result: <Reg<f64>>::default(), condition, true_val: <Reg<f64>>::default(), false_val }
    }
    pub fn f64_select_rssr(condition: Slot, true_val: Slot) -> Self {
        Self::F64Select_Rssr { result: <Reg<f64>>::default(), condition, true_val, false_val: <Reg<f64>>::default() }
    }
    pub fn f64_select_rsss(condition: Slot, true_val: Slot, false_val: Slot) -> Self {
        Self::F64Select_Rsss { result: <Reg<f64>>::default(), condition, true_val, false_val }
    }
    pub fn f64_select_rssi(condition: Slot, true_val: Slot, false_val: f64) -> Self {
        Self::F64Select_Rssi { result: <Reg<f64>>::default(), condition, true_val, false_val }
    }
    pub fn f64_select_rsir(condition: Slot, true_val: f64) -> Self {
        Self::F64Select_Rsir { result: <Reg<f64>>::default(), condition, true_val, false_val: <Reg<f64>>::default() }
    }
    pub fn f64_select_rsis(condition: Slot, true_val: f64, false_val: Slot) -> Self {
        Self::F64Select_Rsis { result: <Reg<f64>>::default(), condition, true_val, false_val }
    }
    pub fn f64_select_rsii(condition: Slot, true_val: f64, false_val: f64) -> Self {
        Self::F64Select_Rsii { result: <Reg<f64>>::default(), condition, true_val, false_val }
    }
    pub fn u32_load_rr(offset: Offset, memory: Memory) -> Self {
        Self::U32Load_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u32_load_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U32LoadMem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u32_load_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U32LoadMem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u32_load_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U32Load_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u32_load_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U32LoadMem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u32_load_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U32LoadMem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u32_load_ri(ptr: Address, memory: Memory) -> Self {
        Self::U32Load_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u64_load_rr(offset: Offset, memory: Memory) -> Self {
        Self::U64Load_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u64_load_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U64LoadMem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U64LoadMem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U64Load_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u64_load_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadMem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u64_load_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadMem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u64_load_ri(ptr: Address, memory: Memory) -> Self {
        Self::U64Load_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn f32_load_rr(offset: Offset, memory: Memory) -> Self {
        Self::F32Load_Rr { result: <Reg<f32>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn f32_load_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::F32LoadMem0Offset16_Rr { result: <Reg<f32>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn f32_load_mem0_offset16_rs_r(result: SlotAndReg<f32>, offset: Offset16) -> Self {
        Self::F32LoadMem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn f32_load_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::F32Load_Rs { result: <Reg<f32>>::default(), ptr, offset, memory }
    }
    pub fn f32_load_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::F32LoadMem0Offset16_Rs { result: <Reg<f32>>::default(), ptr, offset }
    }
    pub fn f32_load_mem0_offset16_rs_s(result: SlotAndReg<f32>, ptr: Slot, offset: Offset16) -> Self {
        Self::F32LoadMem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn f32_load_ri(ptr: Address, memory: Memory) -> Self {
        Self::F32Load_Ri { result: <Reg<f32>>::default(), ptr, memory }
    }
    pub fn f64_load_rr(offset: Offset, memory: Memory) -> Self {
        Self::F64Load_Rr { result: <Reg<f64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn f64_load_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::F64LoadMem0Offset16_Rr { result: <Reg<f64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn f64_load_mem0_offset16_rs_r(result: SlotAndReg<f64>, offset: Offset16) -> Self {
        Self::F64LoadMem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn f64_load_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::F64Load_Rs { result: <Reg<f64>>::default(), ptr, offset, memory }
    }
    pub fn f64_load_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::F64LoadMem0Offset16_Rs { result: <Reg<f64>>::default(), ptr, offset }
    }
    pub fn f64_load_mem0_offset16_rs_s(result: SlotAndReg<f64>, ptr: Slot, offset: Offset16) -> Self {
        Self::F64LoadMem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn f64_load_ri(ptr: Address, memory: Memory) -> Self {
        Self::F64Load_Ri { result: <Reg<f64>>::default(), ptr, memory }
    }
    pub fn i32_load_extend8_rr(offset: Offset, memory: Memory) -> Self {
        Self::I32LoadExtend8_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn i32_load_extend8_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::I32LoadExtend8Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i32_load_extend8_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::I32LoadExtend8Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i32_load_extend8_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I32LoadExtend8_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn i32_load_extend8_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::I32LoadExtend8Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn i32_load_extend8_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::I32LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn i32_load_extend8_ri(ptr: Address, memory: Memory) -> Self {
        Self::I32LoadExtend8_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn i32_load_extend16_rr(offset: Offset, memory: Memory) -> Self {
        Self::I32LoadExtend16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn i32_load_extend16_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::I32LoadExtend16Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i32_load_extend16_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::I32LoadExtend16Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i32_load_extend16_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I32LoadExtend16_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn i32_load_extend16_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::I32LoadExtend16Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn i32_load_extend16_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::I32LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn i32_load_extend16_ri(ptr: Address, memory: Memory) -> Self {
        Self::I32LoadExtend16_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u32_load_extend8_rr(offset: Offset, memory: Memory) -> Self {
        Self::U32LoadExtend8_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u32_load_extend8_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U32LoadExtend8Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u32_load_extend8_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U32LoadExtend8Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u32_load_extend8_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U32LoadExtend8_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u32_load_extend8_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U32LoadExtend8Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u32_load_extend8_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U32LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u32_load_extend8_ri(ptr: Address, memory: Memory) -> Self {
        Self::U32LoadExtend8_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u32_load_extend16_rr(offset: Offset, memory: Memory) -> Self {
        Self::U32LoadExtend16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u32_load_extend16_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U32LoadExtend16Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u32_load_extend16_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U32LoadExtend16Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u32_load_extend16_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U32LoadExtend16_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u32_load_extend16_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U32LoadExtend16Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u32_load_extend16_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U32LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u32_load_extend16_ri(ptr: Address, memory: Memory) -> Self {
        Self::U32LoadExtend16_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn i64_load_extend8_rr(offset: Offset, memory: Memory) -> Self {
        Self::I64LoadExtend8_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn i64_load_extend8_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::I64LoadExtend8Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i64_load_extend8_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::I64LoadExtend8Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i64_load_extend8_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I64LoadExtend8_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn i64_load_extend8_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::I64LoadExtend8Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn i64_load_extend8_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::I64LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn i64_load_extend8_ri(ptr: Address, memory: Memory) -> Self {
        Self::I64LoadExtend8_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn i64_load_extend16_rr(offset: Offset, memory: Memory) -> Self {
        Self::I64LoadExtend16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn i64_load_extend16_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::I64LoadExtend16Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i64_load_extend16_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::I64LoadExtend16Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i64_load_extend16_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I64LoadExtend16_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn i64_load_extend16_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::I64LoadExtend16Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn i64_load_extend16_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::I64LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn i64_load_extend16_ri(ptr: Address, memory: Memory) -> Self {
        Self::I64LoadExtend16_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn i64_load_extend32_rr(offset: Offset, memory: Memory) -> Self {
        Self::I64LoadExtend32_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn i64_load_extend32_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::I64LoadExtend32Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i64_load_extend32_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::I64LoadExtend32Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn i64_load_extend32_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I64LoadExtend32_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn i64_load_extend32_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::I64LoadExtend32Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn i64_load_extend32_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::I64LoadExtend32Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn i64_load_extend32_ri(ptr: Address, memory: Memory) -> Self {
        Self::I64LoadExtend32_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u64_load_extend8_rr(offset: Offset, memory: Memory) -> Self {
        Self::U64LoadExtend8_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u64_load_extend8_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U64LoadExtend8Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_extend8_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U64LoadExtend8Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_extend8_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U64LoadExtend8_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u64_load_extend8_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadExtend8Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u64_load_extend8_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u64_load_extend8_ri(ptr: Address, memory: Memory) -> Self {
        Self::U64LoadExtend8_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u64_load_extend16_rr(offset: Offset, memory: Memory) -> Self {
        Self::U64LoadExtend16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u64_load_extend16_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U64LoadExtend16Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_extend16_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U64LoadExtend16Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_extend16_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U64LoadExtend16_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u64_load_extend16_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadExtend16Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u64_load_extend16_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u64_load_extend16_ri(ptr: Address, memory: Memory) -> Self {
        Self::U64LoadExtend16_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u64_load_extend32_rr(offset: Offset, memory: Memory) -> Self {
        Self::U64LoadExtend32_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset, memory }
    }
    pub fn u64_load_extend32_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::U64LoadExtend32Mem0Offset16_Rr { result: <Reg<i64>>::default(), ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_extend32_mem0_offset16_rs_r(result: SlotAndReg<i64>, offset: Offset16) -> Self {
        Self::U64LoadExtend32Mem0Offset16_Rs_r { result, ptr: <Reg<i64>>::default(), offset }
    }
    pub fn u64_load_extend32_rs(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U64LoadExtend32_Rs { result: <Reg<i64>>::default(), ptr, offset, memory }
    }
    pub fn u64_load_extend32_mem0_offset16_rs(ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadExtend32Mem0Offset16_Rs { result: <Reg<i64>>::default(), ptr, offset }
    }
    pub fn u64_load_extend32_mem0_offset16_rs_s(result: SlotAndReg<i64>, ptr: Slot, offset: Offset16) -> Self {
        Self::U64LoadExtend32Mem0Offset16_Rs_s { result, ptr, offset }
    }
    pub fn u64_load_extend32_ri(ptr: Address, memory: Memory) -> Self {
        Self::U64LoadExtend32_Ri { result: <Reg<i64>>::default(), ptr, memory }
    }
    pub fn u32_store_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::U32Store_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn u32_store_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::U32StoreMem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn u32_store_ri(offset: Offset, value: u32, memory: Memory) -> Self {
        Self::U32Store_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn u32_store_mem0_offset16_ri(offset: Offset16, value: u32) -> Self {
        Self::U32StoreMem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn u32_store_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U32Store_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn u32_store_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::U32StoreMem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn u32_store_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::U32Store_Ss { ptr, offset, value, memory }
    }
    pub fn u32_store_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::U32StoreMem0Offset16_Ss { ptr, offset, value }
    }
    pub fn u32_store_si(ptr: Slot, offset: Offset, value: u32, memory: Memory) -> Self {
        Self::U32Store_Si { ptr, offset, value, memory }
    }
    pub fn u32_store_mem0_offset16_si(ptr: Slot, offset: Offset16, value: u32) -> Self {
        Self::U32StoreMem0Offset16_Si { ptr, offset, value }
    }
    pub fn u32_store_ir(ptr: Address, memory: Memory) -> Self {
        Self::U32Store_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn u32_store_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::U32Store_Is { ptr, value, memory }
    }
    pub fn u32_store_ii(ptr: Address, value: u32, memory: Memory) -> Self {
        Self::U32Store_Ii { ptr, value, memory }
    }
    pub fn u64_store_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::U64Store_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn u64_store_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::U64StoreMem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn u64_store_ri(offset: Offset, value: u64, memory: Memory) -> Self {
        Self::U64Store_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn u64_store_mem0_offset16_ri(offset: Offset16, value: u64) -> Self {
        Self::U64StoreMem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn u64_store_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::U64Store_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn u64_store_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::U64StoreMem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn u64_store_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::U64Store_Ss { ptr, offset, value, memory }
    }
    pub fn u64_store_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::U64StoreMem0Offset16_Ss { ptr, offset, value }
    }
    pub fn u64_store_si(ptr: Slot, offset: Offset, value: u64, memory: Memory) -> Self {
        Self::U64Store_Si { ptr, offset, value, memory }
    }
    pub fn u64_store_mem0_offset16_si(ptr: Slot, offset: Offset16, value: u64) -> Self {
        Self::U64StoreMem0Offset16_Si { ptr, offset, value }
    }
    pub fn u64_store_ir(ptr: Address, memory: Memory) -> Self {
        Self::U64Store_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn u64_store_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::U64Store_Is { ptr, value, memory }
    }
    pub fn u64_store_ii(ptr: Address, value: u64, memory: Memory) -> Self {
        Self::U64Store_Ii { ptr, value, memory }
    }
    pub fn f32_store_rr(offset: Offset, memory: Memory) -> Self {
        Self::F32Store_Rr { ptr: <Reg<i64>>::default(), offset, value: <Reg<f32>>::default(), memory }
    }
    pub fn f32_store_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::F32StoreMem0Offset16_Rr { ptr: <Reg<i64>>::default(), offset, value: <Reg<f32>>::default() }
    }
    pub fn f32_store_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::F32Store_Sr { ptr, offset, value: <Reg<f32>>::default(), memory }
    }
    pub fn f32_store_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::F32StoreMem0Offset16_Sr { ptr, offset, value: <Reg<f32>>::default() }
    }
    pub fn f32_store_ir(ptr: Address, memory: Memory) -> Self {
        Self::F32Store_Ir { ptr, value: <Reg<f32>>::default(), memory }
    }
    pub fn f64_store_rr(offset: Offset, memory: Memory) -> Self {
        Self::F64Store_Rr { ptr: <Reg<i64>>::default(), offset, value: <Reg<f64>>::default(), memory }
    }
    pub fn f64_store_mem0_offset16_rr(offset: Offset16) -> Self {
        Self::F64StoreMem0Offset16_Rr { ptr: <Reg<i64>>::default(), offset, value: <Reg<f64>>::default() }
    }
    pub fn f64_store_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::F64Store_Sr { ptr, offset, value: <Reg<f64>>::default(), memory }
    }
    pub fn f64_store_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::F64StoreMem0Offset16_Sr { ptr, offset, value: <Reg<f64>>::default() }
    }
    pub fn f64_store_ir(ptr: Address, memory: Memory) -> Self {
        Self::F64Store_Ir { ptr, value: <Reg<f64>>::default(), memory }
    }
    pub fn i32_store_wrap8_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I32StoreWrap8_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i32_store_wrap8_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::I32StoreWrap8Mem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i32_store_wrap8_ri(offset: Offset, value: i8, memory: Memory) -> Self {
        Self::I32StoreWrap8_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i32_store_wrap8_mem0_offset16_ri(offset: Offset16, value: i8) -> Self {
        Self::I32StoreWrap8Mem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i32_store_wrap8_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I32StoreWrap8_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn i32_store_wrap8_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::I32StoreWrap8Mem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn i32_store_wrap8_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I32StoreWrap8_Ss { ptr, offset, value, memory }
    }
    pub fn i32_store_wrap8_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::I32StoreWrap8Mem0Offset16_Ss { ptr, offset, value }
    }
    pub fn i32_store_wrap8_si(ptr: Slot, offset: Offset, value: i8, memory: Memory) -> Self {
        Self::I32StoreWrap8_Si { ptr, offset, value, memory }
    }
    pub fn i32_store_wrap8_mem0_offset16_si(ptr: Slot, offset: Offset16, value: i8) -> Self {
        Self::I32StoreWrap8Mem0Offset16_Si { ptr, offset, value }
    }
    pub fn i32_store_wrap8_ir(ptr: Address, memory: Memory) -> Self {
        Self::I32StoreWrap8_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn i32_store_wrap8_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::I32StoreWrap8_Is { ptr, value, memory }
    }
    pub fn i32_store_wrap8_ii(ptr: Address, value: i8, memory: Memory) -> Self {
        Self::I32StoreWrap8_Ii { ptr, value, memory }
    }
    pub fn i32_store_wrap16_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I32StoreWrap16_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i32_store_wrap16_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::I32StoreWrap16Mem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i32_store_wrap16_ri(offset: Offset, value: i16, memory: Memory) -> Self {
        Self::I32StoreWrap16_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i32_store_wrap16_mem0_offset16_ri(offset: Offset16, value: i16) -> Self {
        Self::I32StoreWrap16Mem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i32_store_wrap16_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I32StoreWrap16_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn i32_store_wrap16_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::I32StoreWrap16Mem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn i32_store_wrap16_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I32StoreWrap16_Ss { ptr, offset, value, memory }
    }
    pub fn i32_store_wrap16_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::I32StoreWrap16Mem0Offset16_Ss { ptr, offset, value }
    }
    pub fn i32_store_wrap16_si(ptr: Slot, offset: Offset, value: i16, memory: Memory) -> Self {
        Self::I32StoreWrap16_Si { ptr, offset, value, memory }
    }
    pub fn i32_store_wrap16_mem0_offset16_si(ptr: Slot, offset: Offset16, value: i16) -> Self {
        Self::I32StoreWrap16Mem0Offset16_Si { ptr, offset, value }
    }
    pub fn i32_store_wrap16_ir(ptr: Address, memory: Memory) -> Self {
        Self::I32StoreWrap16_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn i32_store_wrap16_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::I32StoreWrap16_Is { ptr, value, memory }
    }
    pub fn i32_store_wrap16_ii(ptr: Address, value: i16, memory: Memory) -> Self {
        Self::I32StoreWrap16_Ii { ptr, value, memory }
    }
    pub fn i64_store_wrap8_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap8_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i64_store_wrap8_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::I64StoreWrap8Mem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i64_store_wrap8_ri(offset: Offset, value: i8, memory: Memory) -> Self {
        Self::I64StoreWrap8_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i64_store_wrap8_mem0_offset16_ri(offset: Offset16, value: i8) -> Self {
        Self::I64StoreWrap8Mem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i64_store_wrap8_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I64StoreWrap8_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn i64_store_wrap8_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::I64StoreWrap8Mem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn i64_store_wrap8_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap8_Ss { ptr, offset, value, memory }
    }
    pub fn i64_store_wrap8_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::I64StoreWrap8Mem0Offset16_Ss { ptr, offset, value }
    }
    pub fn i64_store_wrap8_si(ptr: Slot, offset: Offset, value: i8, memory: Memory) -> Self {
        Self::I64StoreWrap8_Si { ptr, offset, value, memory }
    }
    pub fn i64_store_wrap8_mem0_offset16_si(ptr: Slot, offset: Offset16, value: i8) -> Self {
        Self::I64StoreWrap8Mem0Offset16_Si { ptr, offset, value }
    }
    pub fn i64_store_wrap8_ir(ptr: Address, memory: Memory) -> Self {
        Self::I64StoreWrap8_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn i64_store_wrap8_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap8_Is { ptr, value, memory }
    }
    pub fn i64_store_wrap8_ii(ptr: Address, value: i8, memory: Memory) -> Self {
        Self::I64StoreWrap8_Ii { ptr, value, memory }
    }
    pub fn i64_store_wrap16_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap16_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i64_store_wrap16_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::I64StoreWrap16Mem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i64_store_wrap16_ri(offset: Offset, value: i16, memory: Memory) -> Self {
        Self::I64StoreWrap16_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i64_store_wrap16_mem0_offset16_ri(offset: Offset16, value: i16) -> Self {
        Self::I64StoreWrap16Mem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i64_store_wrap16_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I64StoreWrap16_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn i64_store_wrap16_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::I64StoreWrap16Mem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn i64_store_wrap16_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap16_Ss { ptr, offset, value, memory }
    }
    pub fn i64_store_wrap16_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::I64StoreWrap16Mem0Offset16_Ss { ptr, offset, value }
    }
    pub fn i64_store_wrap16_si(ptr: Slot, offset: Offset, value: i16, memory: Memory) -> Self {
        Self::I64StoreWrap16_Si { ptr, offset, value, memory }
    }
    pub fn i64_store_wrap16_mem0_offset16_si(ptr: Slot, offset: Offset16, value: i16) -> Self {
        Self::I64StoreWrap16Mem0Offset16_Si { ptr, offset, value }
    }
    pub fn i64_store_wrap16_ir(ptr: Address, memory: Memory) -> Self {
        Self::I64StoreWrap16_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn i64_store_wrap16_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap16_Is { ptr, value, memory }
    }
    pub fn i64_store_wrap16_ii(ptr: Address, value: i16, memory: Memory) -> Self {
        Self::I64StoreWrap16_Ii { ptr, value, memory }
    }
    pub fn i64_store_wrap32_rs(offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap32_Rs { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i64_store_wrap32_mem0_offset16_rs(offset: Offset16, value: Slot) -> Self {
        Self::I64StoreWrap32Mem0Offset16_Rs { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i64_store_wrap32_ri(offset: Offset, value: i32, memory: Memory) -> Self {
        Self::I64StoreWrap32_Ri { ptr: <Reg<i64>>::default(), offset, value, memory }
    }
    pub fn i64_store_wrap32_mem0_offset16_ri(offset: Offset16, value: i32) -> Self {
        Self::I64StoreWrap32Mem0Offset16_Ri { ptr: <Reg<i64>>::default(), offset, value }
    }
    pub fn i64_store_wrap32_sr(ptr: Slot, offset: Offset, memory: Memory) -> Self {
        Self::I64StoreWrap32_Sr { ptr, offset, value: <Reg<i64>>::default(), memory }
    }
    pub fn i64_store_wrap32_mem0_offset16_sr(ptr: Slot, offset: Offset16) -> Self {
        Self::I64StoreWrap32Mem0Offset16_Sr { ptr, offset, value: <Reg<i64>>::default() }
    }
    pub fn i64_store_wrap32_ss(ptr: Slot, offset: Offset, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap32_Ss { ptr, offset, value, memory }
    }
    pub fn i64_store_wrap32_mem0_offset16_ss(ptr: Slot, offset: Offset16, value: Slot) -> Self {
        Self::I64StoreWrap32Mem0Offset16_Ss { ptr, offset, value }
    }
    pub fn i64_store_wrap32_si(ptr: Slot, offset: Offset, value: i32, memory: Memory) -> Self {
        Self::I64StoreWrap32_Si { ptr, offset, value, memory }
    }
    pub fn i64_store_wrap32_mem0_offset16_si(ptr: Slot, offset: Offset16, value: i32) -> Self {
        Self::I64StoreWrap32Mem0Offset16_Si { ptr, offset, value }
    }
    pub fn i64_store_wrap32_ir(ptr: Address, memory: Memory) -> Self {
        Self::I64StoreWrap32_Ir { ptr, value: <Reg<i64>>::default(), memory }
    }
    pub fn i64_store_wrap32_is(ptr: Address, value: Slot, memory: Memory) -> Self {
        Self::I64StoreWrap32_Is { ptr, value, memory }
    }
    pub fn i64_store_wrap32_ii(ptr: Address, value: i32, memory: Memory) -> Self {
        Self::I64StoreWrap32_Ii { ptr, value, memory }
    }
    pub fn r#return() -> Self {
        Self::Return {  }
    }
    pub fn trap(trap_code: TrapCode) -> Self {
        Self::Trap { trap_code }
    }
    pub fn consume_fuel(fuel: BlockFuel) -> Self {
        Self::ConsumeFuel { fuel }
    }
    pub fn branch(offset: BranchOffset) -> Self {
        Self::Branch { offset }
    }
    pub fn branch_table_r(len_targets: u32) -> Self {
        Self::BranchTable_R { len_targets, index: <Reg<i64>>::default() }
    }
    pub fn branch_table_s(len_targets: u32, index: Slot) -> Self {
        Self::BranchTable_S { len_targets, index }
    }
    pub fn branch_table_span_r(len_targets: u32, values: BoundedSlotSpan) -> Self {
        Self::BranchTableSpan_R { len_targets, index: <Reg<i64>>::default(), values }
    }
    pub fn branch_table_span_s(len_targets: u32, index: Slot, values: BoundedSlotSpan) -> Self {
        Self::BranchTableSpan_S { len_targets, index, values }
    }
    pub fn u32_copy_ri(value: u32) -> Self {
        Self::U32Copy_Ri { result: <Reg<i64>>::default(), value }
    }
    pub fn u32_copy_si(result: Slot, value: u32) -> Self {
        Self::U32Copy_Si { result, value }
    }
    pub fn u64_copy_rs(value: Slot) -> Self {
        Self::U64Copy_Rs { result: <Reg<i64>>::default(), value }
    }
    pub fn u64_copy_ri(value: u64) -> Self {
        Self::U64Copy_Ri { result: <Reg<i64>>::default(), value }
    }
    pub fn u64_copy_sr(result: Slot) -> Self {
        Self::U64Copy_Sr { result, value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_ss(result: Slot, value: Slot) -> Self {
        Self::U64Copy_Ss { result, value }
    }
    pub fn u64_copy_si(result: Slot, value: u64) -> Self {
        Self::U64Copy_Si { result, value }
    }
    pub fn f32_copy_ri(value: f32) -> Self {
        Self::F32Copy_Ri { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_copy_rs(value: Slot) -> Self {
        Self::F32Copy_Rs { result: <Reg<f32>>::default(), value }
    }
    pub fn f32_copy_sr(result: Slot) -> Self {
        Self::F32Copy_Sr { result, value: <Reg<f32>>::default() }
    }
    pub fn f64_copy_rs(value: Slot) -> Self {
        Self::F64Copy_Rs { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_copy_ri(value: f64) -> Self {
        Self::F64Copy_Ri { result: <Reg<f64>>::default(), value }
    }
    pub fn f64_copy_sr(result: Slot) -> Self {
        Self::F64Copy_Sr { result, value: <Reg<f64>>::default() }
    }
    pub fn u64_copy_s0s1() -> Self {
        Self::U64Copy_S0s1 { result: <Local<0>>::default(), value: <Local<1>>::default() }
    }
    pub fn u64_copy_s0s2() -> Self {
        Self::U64Copy_S0s2 { result: <Local<0>>::default(), value: <Local<2>>::default() }
    }
    pub fn u64_copy_s0s3() -> Self {
        Self::U64Copy_S0s3 { result: <Local<0>>::default(), value: <Local<3>>::default() }
    }
    pub fn u64_copy_s0s4() -> Self {
        Self::U64Copy_S0s4 { result: <Local<0>>::default(), value: <Local<4>>::default() }
    }
    pub fn u64_copy_s0s5() -> Self {
        Self::U64Copy_S0s5 { result: <Local<0>>::default(), value: <Local<5>>::default() }
    }
    pub fn u64_copy_s1s0() -> Self {
        Self::U64Copy_S1s0 { result: <Local<1>>::default(), value: <Local<0>>::default() }
    }
    pub fn u64_copy_s1s2() -> Self {
        Self::U64Copy_S1s2 { result: <Local<1>>::default(), value: <Local<2>>::default() }
    }
    pub fn u64_copy_s1s3() -> Self {
        Self::U64Copy_S1s3 { result: <Local<1>>::default(), value: <Local<3>>::default() }
    }
    pub fn u64_copy_s1s4() -> Self {
        Self::U64Copy_S1s4 { result: <Local<1>>::default(), value: <Local<4>>::default() }
    }
    pub fn u64_copy_s1s5() -> Self {
        Self::U64Copy_S1s5 { result: <Local<1>>::default(), value: <Local<5>>::default() }
    }
    pub fn u64_copy_s2s0() -> Self {
        Self::U64Copy_S2s0 { result: <Local<2>>::default(), value: <Local<0>>::default() }
    }
    pub fn u64_copy_s2s1() -> Self {
        Self::U64Copy_S2s1 { result: <Local<2>>::default(), value: <Local<1>>::default() }
    }
    pub fn u64_copy_s2s3() -> Self {
        Self::U64Copy_S2s3 { result: <Local<2>>::default(), value: <Local<3>>::default() }
    }
    pub fn u64_copy_s2s4() -> Self {
        Self::U64Copy_S2s4 { result: <Local<2>>::default(), value: <Local<4>>::default() }
    }
    pub fn u64_copy_s2s5() -> Self {
        Self::U64Copy_S2s5 { result: <Local<2>>::default(), value: <Local<5>>::default() }
    }
    pub fn u64_copy_s3s0() -> Self {
        Self::U64Copy_S3s0 { result: <Local<3>>::default(), value: <Local<0>>::default() }
    }
    pub fn u64_copy_s3s1() -> Self {
        Self::U64Copy_S3s1 { result: <Local<3>>::default(), value: <Local<1>>::default() }
    }
    pub fn u64_copy_s3s2() -> Self {
        Self::U64Copy_S3s2 { result: <Local<3>>::default(), value: <Local<2>>::default() }
    }
    pub fn u64_copy_s3s4() -> Self {
        Self::U64Copy_S3s4 { result: <Local<3>>::default(), value: <Local<4>>::default() }
    }
    pub fn u64_copy_s3s5() -> Self {
        Self::U64Copy_S3s5 { result: <Local<3>>::default(), value: <Local<5>>::default() }
    }
    pub fn u64_copy_s4s0() -> Self {
        Self::U64Copy_S4s0 { result: <Local<4>>::default(), value: <Local<0>>::default() }
    }
    pub fn u64_copy_s4s1() -> Self {
        Self::U64Copy_S4s1 { result: <Local<4>>::default(), value: <Local<1>>::default() }
    }
    pub fn u64_copy_s4s2() -> Self {
        Self::U64Copy_S4s2 { result: <Local<4>>::default(), value: <Local<2>>::default() }
    }
    pub fn u64_copy_s4s3() -> Self {
        Self::U64Copy_S4s3 { result: <Local<4>>::default(), value: <Local<3>>::default() }
    }
    pub fn u64_copy_s4s5() -> Self {
        Self::U64Copy_S4s5 { result: <Local<4>>::default(), value: <Local<5>>::default() }
    }
    pub fn u64_copy_s5s0() -> Self {
        Self::U64Copy_S5s0 { result: <Local<5>>::default(), value: <Local<0>>::default() }
    }
    pub fn u64_copy_s5s1() -> Self {
        Self::U64Copy_S5s1 { result: <Local<5>>::default(), value: <Local<1>>::default() }
    }
    pub fn u64_copy_s5s2() -> Self {
        Self::U64Copy_S5s2 { result: <Local<5>>::default(), value: <Local<2>>::default() }
    }
    pub fn u64_copy_s5s3() -> Self {
        Self::U64Copy_S5s3 { result: <Local<5>>::default(), value: <Local<3>>::default() }
    }
    pub fn u64_copy_s5s4() -> Self {
        Self::U64Copy_S5s4 { result: <Local<5>>::default(), value: <Local<4>>::default() }
    }
    pub fn f32_reinterpret_i32_rr() -> Self {
        Self::F32ReinterpretI32_Rr { result: <Reg<f32>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i32_reinterpret_f32_rr() -> Self {
        Self::I32ReinterpretF32_Rr { result: <Reg<i64>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f64_reinterpret_i64_rr() -> Self {
        Self::F64ReinterpretI64_Rr { result: <Reg<f64>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn i64_reinterpret_f64_rr() -> Self {
        Self::I64ReinterpretF64_Rr { result: <Reg<i64>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn u64_copy_s0r() -> Self {
        Self::U64Copy_S0r { result: <Local<0>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s1r() -> Self {
        Self::U64Copy_S1r { result: <Local<1>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s2r() -> Self {
        Self::U64Copy_S2r { result: <Local<2>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s3r() -> Self {
        Self::U64Copy_S3r { result: <Local<3>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s4r() -> Self {
        Self::U64Copy_S4r { result: <Local<4>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s5r() -> Self {
        Self::U64Copy_S5r { result: <Local<5>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s6r() -> Self {
        Self::U64Copy_S6r { result: <Local<6>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s7r() -> Self {
        Self::U64Copy_S7r { result: <Local<7>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s8r() -> Self {
        Self::U64Copy_S8r { result: <Local<8>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn u64_copy_s9r() -> Self {
        Self::U64Copy_S9r { result: <Local<9>>::default(), value: <Reg<i64>>::default() }
    }
    pub fn f32_copy_s0r() -> Self {
        Self::F32Copy_S0r { result: <Local<0>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s1r() -> Self {
        Self::F32Copy_S1r { result: <Local<1>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s2r() -> Self {
        Self::F32Copy_S2r { result: <Local<2>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s3r() -> Self {
        Self::F32Copy_S3r { result: <Local<3>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s4r() -> Self {
        Self::F32Copy_S4r { result: <Local<4>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s5r() -> Self {
        Self::F32Copy_S5r { result: <Local<5>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s6r() -> Self {
        Self::F32Copy_S6r { result: <Local<6>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s7r() -> Self {
        Self::F32Copy_S7r { result: <Local<7>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s8r() -> Self {
        Self::F32Copy_S8r { result: <Local<8>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f32_copy_s9r() -> Self {
        Self::F32Copy_S9r { result: <Local<9>>::default(), value: <Reg<f32>>::default() }
    }
    pub fn f64_copy_s0r() -> Self {
        Self::F64Copy_S0r { result: <Local<0>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s1r() -> Self {
        Self::F64Copy_S1r { result: <Local<1>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s2r() -> Self {
        Self::F64Copy_S2r { result: <Local<2>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s3r() -> Self {
        Self::F64Copy_S3r { result: <Local<3>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s4r() -> Self {
        Self::F64Copy_S4r { result: <Local<4>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s5r() -> Self {
        Self::F64Copy_S5r { result: <Local<5>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s6r() -> Self {
        Self::F64Copy_S6r { result: <Local<6>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s7r() -> Self {
        Self::F64Copy_S7r { result: <Local<7>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s8r() -> Self {
        Self::F64Copy_S8r { result: <Local<8>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn f64_copy_s9r() -> Self {
        Self::F64Copy_S9r { result: <Local<9>>::default(), value: <Reg<f64>>::default() }
    }
    pub fn ref_func(func: Func) -> Self {
        Self::RefFunc { result: <Reg<i64>>::default(), func }
    }
    pub fn call_internal(params: BoundedSlotSpan, func: InternalFunc) -> Self {
        Self::CallInternal { params, func }
    }
    pub fn call_imported(params: BoundedSlotSpan, func: Func) -> Self {
        Self::CallImported { params, func }
    }
    pub fn call_indirect_r(table: Table, func_type: FuncType, params: BoundedSlotSpan) -> Self {
        Self::CallIndirect_R { table, func_type, params, index: <Reg<i64>>::default() }
    }
    pub fn call_indirect_s(table: Table, func_type: FuncType, params: BoundedSlotSpan, index: Slot) -> Self {
        Self::CallIndirect_S { table, func_type, params, index }
    }
    pub fn r#return_call_indirect_r(table: Table, func_type: FuncType, params: BoundedSlotSpan) -> Self {
        Self::ReturnCallIndirect_R { table, func_type, params, index: <Reg<i64>>::default() }
    }
    pub fn r#return_call_indirect_s(table: Table, func_type: FuncType, params: BoundedSlotSpan, index: Slot) -> Self {
        Self::ReturnCallIndirect_S { table, func_type, params, index }
    }
    pub fn return_call_internal(params: BoundedSlotSpan, func: InternalFunc) -> Self {
        Self::ReturnCallInternal { params, func }
    }
    pub fn return_call_imported(params: BoundedSlotSpan, func: Func) -> Self {
        Self::ReturnCallImported { params, func }
    }
    pub fn global_get_u64_r(global: Global) -> Self {
        Self::GlobalGetU64_R { global, result: <Reg<i64>>::default() }
    }
    pub fn global_get_f32_r(global: Global) -> Self {
        Self::GlobalGetF32_R { global, result: <Reg<f32>>::default() }
    }
    pub fn global_get_f64_r(global: Global) -> Self {
        Self::GlobalGetF64_R { global, result: <Reg<f64>>::default() }
    }
    pub fn global_set_u32_i(global: Global, value: u32) -> Self {
        Self::GlobalSetU32_I { global, value }
    }
    pub fn global_set_u64_r(global: Global) -> Self {
        Self::GlobalSetU64_R { global, value: <Reg<i64>>::default() }
    }
    pub fn global_set_u64_s(global: Global, value: Slot) -> Self {
        Self::GlobalSetU64_S { global, value }
    }
    pub fn global_set_u64_i(global: Global, value: u64) -> Self {
        Self::GlobalSetU64_I { global, value }
    }
    pub fn global_set_f32_r(global: Global) -> Self {
        Self::GlobalSetF32_R { global, value: <Reg<f32>>::default() }
    }
    pub fn global_set_f64_r(global: Global) -> Self {
        Self::GlobalSetF64_R { global, value: <Reg<f64>>::default() }
    }
    pub fn data_drop(data: Data) -> Self {
        Self::DataDrop { data }
    }
    pub fn memory_size(memory: Memory) -> Self {
        Self::MemorySize { result: <Reg<i64>>::default(), memory }
    }
    pub fn memory_grow(delta: Slot, memory: Memory) -> Self {
        Self::MemoryGrow { result: <Reg<i64>>::default(), delta, memory }
    }
    pub fn memory_copy(dst_memory: Memory, src_memory: Memory, dst: Slot, src: Slot, len: Slot) -> Self {
        Self::MemoryCopy { dst_memory, src_memory, dst, src, len }
    }
    pub fn memory_fill(memory: Memory, dst: Slot, len: Slot, value: Slot) -> Self {
        Self::MemoryFill { memory, dst, len, value }
    }
    pub fn memory_init(memory: Memory, data: Data, dst: Slot, src: Slot, len: Slot) -> Self {
        Self::MemoryInit { memory, data, dst, src, len }
    }
    pub fn table_get_rr(table: Table) -> Self {
        Self::TableGet_Rr { result: <Reg<i64>>::default(), index: <Reg<i64>>::default(), table }
    }
    pub fn table_set_rs(table: Table, value: Slot) -> Self {
        Self::TableSet_Rs { table, index: <Reg<i64>>::default(), value }
    }
    pub fn table_set_ri(table: Table, value: u32) -> Self {
        Self::TableSet_Ri { table, index: <Reg<i64>>::default(), value }
    }
    pub fn table_get_rs(index: Slot, table: Table) -> Self {
        Self::TableGet_Rs { result: <Reg<i64>>::default(), index, table }
    }
    pub fn table_set_sr(table: Table, index: Slot) -> Self {
        Self::TableSet_Sr { table, index, value: <Reg<i64>>::default() }
    }
    pub fn table_set_ss(table: Table, index: Slot, value: Slot) -> Self {
        Self::TableSet_Ss { table, index, value }
    }
    pub fn table_set_si(table: Table, index: Slot, value: u32) -> Self {
        Self::TableSet_Si { table, index, value }
    }
    pub fn table_get_ri(index: u32, table: Table) -> Self {
        Self::TableGet_Ri { result: <Reg<i64>>::default(), index, table }
    }
    pub fn table_set_ir(table: Table, index: u32) -> Self {
        Self::TableSet_Ir { table, index, value: <Reg<i64>>::default() }
    }
    pub fn table_set_is(table: Table, index: u32, value: Slot) -> Self {
        Self::TableSet_Is { table, index, value }
    }
    pub fn table_set_ii(table: Table, index: u32, value: u32) -> Self {
        Self::TableSet_Ii { table, index, value }
    }
    pub fn table_size(table: Table) -> Self {
        Self::TableSize { result: <Reg<i64>>::default(), table }
    }
    pub fn table_grow(delta: Slot, value: Slot, table: Table) -> Self {
        Self::TableGrow { result: <Reg<i64>>::default(), delta, value, table }
    }
    pub fn table_copy(dst_table: Table, src_table: Table, dst: Slot, src: Slot, len: Slot) -> Self {
        Self::TableCopy { dst_table, src_table, dst, src, len }
    }
    pub fn table_fill(table: Table, dst: Slot, len: Slot, value: Slot) -> Self {
        Self::TableFill { table, dst, len, value }
    }
    pub fn table_init(table: Table, elem: Elem, dst: Slot, src: Slot, len: Slot) -> Self {
        Self::TableInit { table, elem, dst, src, len }
    }
    pub fn elem_drop(elem: Elem) -> Self {
        Self::ElemDrop { elem }
    }
    pub fn i64_add128(results: FixedSlotSpan<2>, lhs_lo: Slot, lhs_hi: Slot, rhs_lo: Slot, rhs_hi: Slot) -> Self {
        Self::I64Add128 { results, lhs_lo, lhs_hi, rhs_lo, rhs_hi }
    }
    pub fn i64_sub128(results: FixedSlotSpan<2>, lhs_lo: Slot, lhs_hi: Slot, rhs_lo: Slot, rhs_hi: Slot) -> Self {
        Self::I64Sub128 { results, lhs_lo, lhs_hi, rhs_lo, rhs_hi }
    }
    pub fn i64_mul_wide(results: FixedSlotSpan<2>, lhs: Slot, rhs: Slot) -> Self {
        Self::I64MulWide { results, lhs, rhs }
    }
    pub fn u64_mul_wide(results: FixedSlotSpan<2>, lhs: Slot, rhs: Slot) -> Self {
        Self::U64MulWide { results, lhs, rhs }
    }
}

