pub type I32Clz_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32Clz_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I32Ctz_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32Ctz_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I32Popcnt_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32Popcnt_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I32Sext8_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32Sext8_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I32Sext16_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32Sext16_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I32WrapI64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32WrapI64_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I64Clz_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64Clz_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I64Ctz_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64Ctz_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I64Popcnt_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64Popcnt_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I64Sext8_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64Sext8_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I64Sext16_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64Sext16_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type I64Sext32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64Sext32_Rr = UnaryOp<Reg<i64>, Reg<i64>>;
pub type F32Abs_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Abs_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Neg_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Neg_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Nabs_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Nabs_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Ceil_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Ceil_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Floor_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Floor_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Trunc_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Trunc_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Nearest_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Nearest_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32Sqrt_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Sqrt_Rr = UnaryOp<Reg<f32>, Reg<f32>>;
pub type F32ConvertI32_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32ConvertI32_Rr = UnaryOp<Reg<f32>, Reg<i64>>;
pub type F32ConvertU32_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32ConvertU32_Rr = UnaryOp<Reg<f32>, Reg<i64>>;
pub type F32ConvertI64_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32ConvertI64_Rr = UnaryOp<Reg<f32>, Reg<i64>>;
pub type F32ConvertU64_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32ConvertU64_Rr = UnaryOp<Reg<f32>, Reg<i64>>;
pub type F32DemoteF64_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32DemoteF64_Rr = UnaryOp<Reg<f32>, Reg<f64>>;
pub type F64Abs_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Abs_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Neg_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Neg_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Nabs_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Nabs_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Ceil_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Ceil_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Floor_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Floor_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Trunc_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Trunc_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Nearest_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Nearest_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64Sqrt_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Sqrt_Rr = UnaryOp<Reg<f64>, Reg<f64>>;
pub type F64ConvertI32_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64ConvertI32_Rr = UnaryOp<Reg<f64>, Reg<i64>>;
pub type F64ConvertU32_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64ConvertU32_Rr = UnaryOp<Reg<f64>, Reg<i64>>;
pub type F64ConvertI64_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64ConvertI64_Rr = UnaryOp<Reg<f64>, Reg<i64>>;
pub type F64ConvertU64_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64ConvertU64_Rr = UnaryOp<Reg<f64>, Reg<i64>>;
pub type F64PromoteF32_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64PromoteF32_Rr = UnaryOp<Reg<f64>, Reg<f32>>;
pub type I32TruncF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32TruncF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type U32TruncF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U32TruncF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type I32TruncF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32TruncF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type U32TruncF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U32TruncF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type I64TruncF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64TruncF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type U64TruncF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U64TruncF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type I64TruncF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64TruncF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type U64TruncF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U64TruncF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type I32TruncSatF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32TruncSatF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type U32TruncSatF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U32TruncSatF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type I32TruncSatF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I32TruncSatF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type U32TruncSatF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U32TruncSatF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type I64TruncSatF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64TruncSatF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type U64TruncSatF32_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U64TruncSatF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type I64TruncSatF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type I64TruncSatF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type U64TruncSatF64_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U64TruncSatF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type I32Eq_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Eq_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32Eq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Eq_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32And_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32And_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32And_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32And_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Or_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Or_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32Or_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Or_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32NotEq_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32NotEq_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32NotEq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32NotEq_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32NotAnd_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32NotAnd_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32NotAnd_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32NotAnd_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32NotOr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32NotOr_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32NotOr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32NotOr_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Lt_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Lt_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32Lt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Lt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Lt_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Lt_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Lt_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type I32Le_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Le_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32Le_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Le_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Le_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Le_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Le_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type U32Lt_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U32Lt_Rri = BinaryOp<Reg<i64>, Reg<i64>, u32>;
pub type U32Lt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U32Lt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U32Lt_Rsi = BinaryOp<Reg<i64>, Slot, u32>;
pub type U32Lt_Rir = BinaryOp<Reg<i64>, u32, Reg<i64>>;
pub type U32Lt_Ris = BinaryOp<Reg<i64>, u32, Slot>;
pub type U32Le_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U32Le_Rri = BinaryOp<Reg<i64>, Reg<i64>, u32>;
pub type U32Le_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U32Le_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U32Le_Rsi = BinaryOp<Reg<i64>, Slot, u32>;
pub type U32Le_Rir = BinaryOp<Reg<i64>, u32, Reg<i64>>;
pub type U32Le_Ris = BinaryOp<Reg<i64>, u32, Slot>;
pub type I64Eq_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Eq_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64Eq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Eq_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64And_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64And_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64And_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64And_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Or_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Or_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64Or_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Or_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64NotEq_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64NotEq_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64NotEq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64NotEq_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64NotAnd_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64NotAnd_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64NotAnd_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64NotAnd_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64NotOr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64NotOr_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64NotOr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64NotOr_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Lt_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Lt_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64Lt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Lt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Lt_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Lt_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Lt_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type I64Le_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Le_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64Le_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Le_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Le_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Le_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Le_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type U64Lt_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U64Lt_Rri = BinaryOp<Reg<i64>, Reg<i64>, u64>;
pub type U64Lt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U64Lt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U64Lt_Rsi = BinaryOp<Reg<i64>, Slot, u64>;
pub type U64Lt_Rir = BinaryOp<Reg<i64>, u64, Reg<i64>>;
pub type U64Lt_Ris = BinaryOp<Reg<i64>, u64, Slot>;
pub type U64Le_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U64Le_Rri = BinaryOp<Reg<i64>, Reg<i64>, u64>;
pub type U64Le_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U64Le_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U64Le_Rsi = BinaryOp<Reg<i64>, Slot, u64>;
pub type U64Le_Rir = BinaryOp<Reg<i64>, u64, Reg<i64>>;
pub type U64Le_Ris = BinaryOp<Reg<i64>, u64, Slot>;
pub type F32Eq_Rrs = BinaryOp<Reg<i64>, Reg<f32>, Slot>;
pub type F32Eq_Rri = BinaryOp<Reg<i64>, Reg<f32>, f32>;
pub type F32Eq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F32Eq_Rsi = BinaryOp<Reg<i64>, Slot, f32>;
pub type F32Lt_Rrs = BinaryOp<Reg<i64>, Reg<f32>, Slot>;
pub type F32Lt_Rri = BinaryOp<Reg<i64>, Reg<f32>, f32>;
pub type F32Lt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f32>>;
pub type F32Lt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F32Lt_Rsi = BinaryOp<Reg<i64>, Slot, f32>;
pub type F32Lt_Rir = BinaryOp<Reg<i64>, f32, Reg<f32>>;
pub type F32Lt_Ris = BinaryOp<Reg<i64>, f32, Slot>;
pub type F32Le_Rrs = BinaryOp<Reg<i64>, Reg<f32>, Slot>;
pub type F32Le_Rri = BinaryOp<Reg<i64>, Reg<f32>, f32>;
pub type F32Le_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f32>>;
pub type F32Le_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F32Le_Rsi = BinaryOp<Reg<i64>, Slot, f32>;
pub type F32Le_Rir = BinaryOp<Reg<i64>, f32, Reg<f32>>;
pub type F32Le_Ris = BinaryOp<Reg<i64>, f32, Slot>;
pub type F32NotEq_Rrs = BinaryOp<Reg<i64>, Reg<f32>, Slot>;
pub type F32NotEq_Rri = BinaryOp<Reg<i64>, Reg<f32>, f32>;
pub type F32NotEq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F32NotEq_Rsi = BinaryOp<Reg<i64>, Slot, f32>;
pub type F32NotLt_Rrs = BinaryOp<Reg<i64>, Reg<f32>, Slot>;
pub type F32NotLt_Rri = BinaryOp<Reg<i64>, Reg<f32>, f32>;
pub type F32NotLt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f32>>;
pub type F32NotLt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F32NotLt_Rsi = BinaryOp<Reg<i64>, Slot, f32>;
pub type F32NotLt_Rir = BinaryOp<Reg<i64>, f32, Reg<f32>>;
pub type F32NotLt_Ris = BinaryOp<Reg<i64>, f32, Slot>;
pub type F32NotLe_Rrs = BinaryOp<Reg<i64>, Reg<f32>, Slot>;
pub type F32NotLe_Rri = BinaryOp<Reg<i64>, Reg<f32>, f32>;
pub type F32NotLe_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f32>>;
pub type F32NotLe_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F32NotLe_Rsi = BinaryOp<Reg<i64>, Slot, f32>;
pub type F32NotLe_Rir = BinaryOp<Reg<i64>, f32, Reg<f32>>;
pub type F32NotLe_Ris = BinaryOp<Reg<i64>, f32, Slot>;
pub type F64Eq_Rrs = BinaryOp<Reg<i64>, Reg<f64>, Slot>;
pub type F64Eq_Rri = BinaryOp<Reg<i64>, Reg<f64>, f64>;
pub type F64Eq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F64Eq_Rsi = BinaryOp<Reg<i64>, Slot, f64>;
pub type F64Lt_Rrs = BinaryOp<Reg<i64>, Reg<f64>, Slot>;
pub type F64Lt_Rri = BinaryOp<Reg<i64>, Reg<f64>, f64>;
pub type F64Lt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f64>>;
pub type F64Lt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F64Lt_Rsi = BinaryOp<Reg<i64>, Slot, f64>;
pub type F64Lt_Rir = BinaryOp<Reg<i64>, f64, Reg<f64>>;
pub type F64Lt_Ris = BinaryOp<Reg<i64>, f64, Slot>;
pub type F64Le_Rrs = BinaryOp<Reg<i64>, Reg<f64>, Slot>;
pub type F64Le_Rri = BinaryOp<Reg<i64>, Reg<f64>, f64>;
pub type F64Le_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f64>>;
pub type F64Le_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F64Le_Rsi = BinaryOp<Reg<i64>, Slot, f64>;
pub type F64Le_Rir = BinaryOp<Reg<i64>, f64, Reg<f64>>;
pub type F64Le_Ris = BinaryOp<Reg<i64>, f64, Slot>;
pub type F64NotEq_Rrs = BinaryOp<Reg<i64>, Reg<f64>, Slot>;
pub type F64NotEq_Rri = BinaryOp<Reg<i64>, Reg<f64>, f64>;
pub type F64NotEq_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F64NotEq_Rsi = BinaryOp<Reg<i64>, Slot, f64>;
pub type F64NotLt_Rrs = BinaryOp<Reg<i64>, Reg<f64>, Slot>;
pub type F64NotLt_Rri = BinaryOp<Reg<i64>, Reg<f64>, f64>;
pub type F64NotLt_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f64>>;
pub type F64NotLt_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F64NotLt_Rsi = BinaryOp<Reg<i64>, Slot, f64>;
pub type F64NotLt_Rir = BinaryOp<Reg<i64>, f64, Reg<f64>>;
pub type F64NotLt_Ris = BinaryOp<Reg<i64>, f64, Slot>;
pub type F64NotLe_Rrs = BinaryOp<Reg<i64>, Reg<f64>, Slot>;
pub type F64NotLe_Rri = BinaryOp<Reg<i64>, Reg<f64>, f64>;
pub type F64NotLe_Rsr = BinaryOp<Reg<i64>, Slot, Reg<f64>>;
pub type F64NotLe_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type F64NotLe_Rsi = BinaryOp<Reg<i64>, Slot, f64>;
pub type F64NotLe_Rir = BinaryOp<Reg<i64>, f64, Reg<f64>>;
pub type F64NotLe_Ris = BinaryOp<Reg<i64>, f64, Slot>;
pub type I32Add_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Add_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32Add_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Add_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Add_Rs_rs = BinaryOp<SlotAndReg<i64>, Reg<i64>, Slot>;
pub type I32Add_Rs_ri = BinaryOp<SlotAndReg<i64>, Reg<i64>, i32>;
pub type I32Add_Rs_ss = BinaryOp<SlotAndReg<i64>, Slot, Slot>;
pub type I32Add_Rs_si = BinaryOp<SlotAndReg<i64>, Slot, i32>;
pub type I32Sub_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Sub_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Sub_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Sub_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Sub_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type I32Mul_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Mul_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32Mul_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Mul_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Div_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Div_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<i32>>;
pub type I32Div_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Div_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Div_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<i32>>;
pub type I32Div_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Div_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type U32Div_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U32Div_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<u32>>;
pub type U32Div_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U32Div_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U32Div_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<u32>>;
pub type U32Div_Rir = BinaryOp<Reg<i64>, u32, Reg<i64>>;
pub type U32Div_Ris = BinaryOp<Reg<i64>, u32, Slot>;
pub type I32Rem_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Rem_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<i32>>;
pub type I32Rem_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Rem_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Rem_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<i32>>;
pub type I32Rem_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Rem_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type U32Rem_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U32Rem_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<u32>>;
pub type U32Rem_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U32Rem_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U32Rem_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<u32>>;
pub type U32Rem_Rir = BinaryOp<Reg<i64>, u32, Reg<i64>>;
pub type U32Rem_Ris = BinaryOp<Reg<i64>, u32, Slot>;
pub type I32BitAnd_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32BitAnd_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32BitAnd_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32BitAnd_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32BitOr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32BitOr_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32BitOr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32BitOr_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32BitXor_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32BitXor_Rri = BinaryOp<Reg<i64>, Reg<i64>, i32>;
pub type I32BitXor_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32BitXor_Rsi = BinaryOp<Reg<i64>, Slot, i32>;
pub type I32Shl_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Shl_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I32Shl_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Shl_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Shl_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I32Shl_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Shl_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type I32Shr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Shr_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I32Shr_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Shr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Shr_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I32Shr_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Shr_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type U32Shr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U32Shr_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type U32Shr_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U32Shr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U32Shr_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type U32Shr_Rir = BinaryOp<Reg<i64>, u32, Reg<i64>>;
pub type U32Shr_Ris = BinaryOp<Reg<i64>, u32, Slot>;
pub type I32Rotl_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Rotl_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I32Rotl_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Rotl_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Rotl_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I32Rotl_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Rotl_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type I32Rotr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I32Rotr_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I32Rotr_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I32Rotr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I32Rotr_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I32Rotr_Rir = BinaryOp<Reg<i64>, i32, Reg<i64>>;
pub type I32Rotr_Ris = BinaryOp<Reg<i64>, i32, Slot>;
pub type I64Add_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Add_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64Add_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Add_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Add_Rs_rs = BinaryOp<SlotAndReg<i64>, Reg<i64>, Slot>;
pub type I64Add_Rs_ri = BinaryOp<SlotAndReg<i64>, Reg<i64>, i64>;
pub type I64Add_Rs_ss = BinaryOp<SlotAndReg<i64>, Slot, Slot>;
pub type I64Add_Rs_si = BinaryOp<SlotAndReg<i64>, Slot, i64>;
pub type I64Sub_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Sub_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Sub_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Sub_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Sub_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type I64Mul_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Mul_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64Mul_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Mul_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Div_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Div_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<i64>>;
pub type I64Div_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Div_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Div_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<i64>>;
pub type I64Div_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Div_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type U64Div_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U64Div_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<u64>>;
pub type U64Div_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U64Div_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U64Div_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<u64>>;
pub type U64Div_Rir = BinaryOp<Reg<i64>, u64, Reg<i64>>;
pub type U64Div_Ris = BinaryOp<Reg<i64>, u64, Slot>;
pub type I64Rem_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Rem_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<i64>>;
pub type I64Rem_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Rem_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Rem_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<i64>>;
pub type I64Rem_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Rem_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type U64Rem_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U64Rem_Rri = BinaryOp<Reg<i64>, Reg<i64>, NonZero<u64>>;
pub type U64Rem_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U64Rem_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U64Rem_Rsi = BinaryOp<Reg<i64>, Slot, NonZero<u64>>;
pub type U64Rem_Rir = BinaryOp<Reg<i64>, u64, Reg<i64>>;
pub type U64Rem_Ris = BinaryOp<Reg<i64>, u64, Slot>;
pub type I64BitAnd_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64BitAnd_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64BitAnd_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64BitAnd_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64BitOr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64BitOr_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64BitOr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64BitOr_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64BitXor_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64BitXor_Rri = BinaryOp<Reg<i64>, Reg<i64>, i64>;
pub type I64BitXor_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64BitXor_Rsi = BinaryOp<Reg<i64>, Slot, i64>;
pub type I64Shl_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Shl_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I64Shl_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Shl_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Shl_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I64Shl_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Shl_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type I64Shr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Shr_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I64Shr_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Shr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Shr_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I64Shr_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Shr_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type U64Shr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type U64Shr_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type U64Shr_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type U64Shr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type U64Shr_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type U64Shr_Rir = BinaryOp<Reg<i64>, u64, Reg<i64>>;
pub type U64Shr_Ris = BinaryOp<Reg<i64>, u64, Slot>;
pub type I64Rotl_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Rotl_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I64Rotl_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Rotl_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Rotl_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I64Rotl_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Rotl_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type I64Rotr_Rrs = BinaryOp<Reg<i64>, Reg<i64>, Slot>;
pub type I64Rotr_Rri = BinaryOp<Reg<i64>, Reg<i64>, ShiftAmount>;
pub type I64Rotr_Rsr = BinaryOp<Reg<i64>, Slot, Reg<i64>>;
pub type I64Rotr_Rss = BinaryOp<Reg<i64>, Slot, Slot>;
pub type I64Rotr_Rsi = BinaryOp<Reg<i64>, Slot, ShiftAmount>;
pub type I64Rotr_Rir = BinaryOp<Reg<i64>, i64, Reg<i64>>;
pub type I64Rotr_Ris = BinaryOp<Reg<i64>, i64, Slot>;
pub type F32Add_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Add_Rri = BinaryOp<Reg<f32>, Reg<f32>, f32>;
pub type F32Add_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Add_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Add_Rsi = BinaryOp<Reg<f32>, Slot, f32>;
pub type F32Add_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Add_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F32Sub_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Sub_Rri = BinaryOp<Reg<f32>, Reg<f32>, f32>;
pub type F32Sub_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Sub_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Sub_Rsi = BinaryOp<Reg<f32>, Slot, f32>;
pub type F32Sub_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Sub_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F32Mul_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Mul_Rri = BinaryOp<Reg<f32>, Reg<f32>, f32>;
pub type F32Mul_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Mul_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Mul_Rsi = BinaryOp<Reg<f32>, Slot, f32>;
pub type F32Mul_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Mul_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F32Div_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Div_Rri = BinaryOp<Reg<f32>, Reg<f32>, f32>;
pub type F32Div_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Div_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Div_Rsi = BinaryOp<Reg<f32>, Slot, f32>;
pub type F32Div_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Div_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F32Min_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Min_Rri = BinaryOp<Reg<f32>, Reg<f32>, f32>;
pub type F32Min_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Min_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Min_Rsi = BinaryOp<Reg<f32>, Slot, f32>;
pub type F32Min_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Min_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F32Max_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Max_Rri = BinaryOp<Reg<f32>, Reg<f32>, f32>;
pub type F32Max_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Max_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Max_Rsi = BinaryOp<Reg<f32>, Slot, f32>;
pub type F32Max_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Max_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F64Add_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Add_Rri = BinaryOp<Reg<f64>, Reg<f64>, f64>;
pub type F64Add_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Add_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Add_Rsi = BinaryOp<Reg<f64>, Slot, f64>;
pub type F64Add_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Add_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type F64Sub_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Sub_Rri = BinaryOp<Reg<f64>, Reg<f64>, f64>;
pub type F64Sub_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Sub_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Sub_Rsi = BinaryOp<Reg<f64>, Slot, f64>;
pub type F64Sub_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Sub_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type F64Mul_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Mul_Rri = BinaryOp<Reg<f64>, Reg<f64>, f64>;
pub type F64Mul_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Mul_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Mul_Rsi = BinaryOp<Reg<f64>, Slot, f64>;
pub type F64Mul_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Mul_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type F64Div_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Div_Rri = BinaryOp<Reg<f64>, Reg<f64>, f64>;
pub type F64Div_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Div_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Div_Rsi = BinaryOp<Reg<f64>, Slot, f64>;
pub type F64Div_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Div_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type F64Min_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Min_Rri = BinaryOp<Reg<f64>, Reg<f64>, f64>;
pub type F64Min_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Min_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Min_Rsi = BinaryOp<Reg<f64>, Slot, f64>;
pub type F64Min_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Min_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type F64Max_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Max_Rri = BinaryOp<Reg<f64>, Reg<f64>, f64>;
pub type F64Max_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Max_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Max_Rsi = BinaryOp<Reg<f64>, Slot, f64>;
pub type F64Max_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Max_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type F32Copysign_Rrs = BinaryOp<Reg<f32>, Reg<f32>, Slot>;
pub type F32Copysign_Rsr = BinaryOp<Reg<f32>, Slot, Reg<f32>>;
pub type F32Copysign_Rss = BinaryOp<Reg<f32>, Slot, Slot>;
pub type F32Copysign_Rir = BinaryOp<Reg<f32>, f32, Reg<f32>>;
pub type F32Copysign_Ris = BinaryOp<Reg<f32>, f32, Slot>;
pub type F64Copysign_Rrs = BinaryOp<Reg<f64>, Reg<f64>, Slot>;
pub type F64Copysign_Rsr = BinaryOp<Reg<f64>, Slot, Reg<f64>>;
pub type F64Copysign_Rss = BinaryOp<Reg<f64>, Slot, Slot>;
pub type F64Copysign_Rir = BinaryOp<Reg<f64>, f64, Reg<f64>>;
pub type F64Copysign_Ris = BinaryOp<Reg<f64>, f64, Slot>;
pub type I32Mul_Rrr = BinaryOp<Reg<i64>, Reg<i64>, Reg<i64>>;
pub type I64Mul_Rrr = BinaryOp<Reg<i64>, Reg<i64>, Reg<i64>>;
pub type F32Mul_Rrr = BinaryOp<Reg<f32>, Reg<f32>, Reg<f32>>;
pub type F64Mul_Rrr = BinaryOp<Reg<f64>, Reg<f64>, Reg<f64>>;
pub type BranchI32Eq_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32Eq_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32Eq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32Eq_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32NotEq_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32NotEq_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32NotEq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32NotEq_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32And_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32And_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32And_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32And_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32NotAnd_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32NotAnd_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32NotAnd_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32NotAnd_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32Or_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32Or_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32Or_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32Or_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32NotOr_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32NotOr_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32NotOr_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32NotOr_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32Lt_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32Lt_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32Lt_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchI32Lt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32Lt_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32Lt_Ir = CmpBranchOp<i32, Reg<i64>>;
pub type BranchI32Lt_Is = CmpBranchOp<i32, Slot>;
pub type BranchI32Le_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI32Le_Ri = CmpBranchOp<Reg<i64>, i32>;
pub type BranchI32Le_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchI32Le_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI32Le_Si = CmpBranchOp<Slot, i32>;
pub type BranchI32Le_Ir = CmpBranchOp<i32, Reg<i64>>;
pub type BranchI32Le_Is = CmpBranchOp<i32, Slot>;
pub type BranchU32Lt_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchU32Lt_Ri = CmpBranchOp<Reg<i64>, u32>;
pub type BranchU32Lt_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchU32Lt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchU32Lt_Si = CmpBranchOp<Slot, u32>;
pub type BranchU32Lt_Ir = CmpBranchOp<u32, Reg<i64>>;
pub type BranchU32Lt_Is = CmpBranchOp<u32, Slot>;
pub type BranchU32Le_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchU32Le_Ri = CmpBranchOp<Reg<i64>, u32>;
pub type BranchU32Le_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchU32Le_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchU32Le_Si = CmpBranchOp<Slot, u32>;
pub type BranchU32Le_Ir = CmpBranchOp<u32, Reg<i64>>;
pub type BranchU32Le_Is = CmpBranchOp<u32, Slot>;
pub type BranchI64Eq_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64Eq_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64Eq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64Eq_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64NotEq_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64NotEq_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64NotEq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64NotEq_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64And_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64And_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64And_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64And_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64NotAnd_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64NotAnd_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64NotAnd_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64NotAnd_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64Or_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64Or_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64Or_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64Or_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64NotOr_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64NotOr_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64NotOr_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64NotOr_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64Lt_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64Lt_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64Lt_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchI64Lt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64Lt_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64Lt_Ir = CmpBranchOp<i64, Reg<i64>>;
pub type BranchI64Lt_Is = CmpBranchOp<i64, Slot>;
pub type BranchI64Le_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchI64Le_Ri = CmpBranchOp<Reg<i64>, i64>;
pub type BranchI64Le_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchI64Le_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchI64Le_Si = CmpBranchOp<Slot, i64>;
pub type BranchI64Le_Ir = CmpBranchOp<i64, Reg<i64>>;
pub type BranchI64Le_Is = CmpBranchOp<i64, Slot>;
pub type BranchU64Lt_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchU64Lt_Ri = CmpBranchOp<Reg<i64>, u64>;
pub type BranchU64Lt_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchU64Lt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchU64Lt_Si = CmpBranchOp<Slot, u64>;
pub type BranchU64Lt_Ir = CmpBranchOp<u64, Reg<i64>>;
pub type BranchU64Lt_Is = CmpBranchOp<u64, Slot>;
pub type BranchU64Le_Rs = CmpBranchOp<Reg<i64>, Slot>;
pub type BranchU64Le_Ri = CmpBranchOp<Reg<i64>, u64>;
pub type BranchU64Le_Sr = CmpBranchOp<Slot, Reg<i64>>;
pub type BranchU64Le_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchU64Le_Si = CmpBranchOp<Slot, u64>;
pub type BranchU64Le_Ir = CmpBranchOp<u64, Reg<i64>>;
pub type BranchU64Le_Is = CmpBranchOp<u64, Slot>;
pub type BranchF32Eq_Rs = CmpBranchOp<Reg<f32>, Slot>;
pub type BranchF32Eq_Ri = CmpBranchOp<Reg<f32>, f32>;
pub type BranchF32Eq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF32Eq_Si = CmpBranchOp<Slot, f32>;
pub type BranchF32NotEq_Rs = CmpBranchOp<Reg<f32>, Slot>;
pub type BranchF32NotEq_Ri = CmpBranchOp<Reg<f32>, f32>;
pub type BranchF32NotEq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF32NotEq_Si = CmpBranchOp<Slot, f32>;
pub type BranchF32Lt_Rs = CmpBranchOp<Reg<f32>, Slot>;
pub type BranchF32Lt_Ri = CmpBranchOp<Reg<f32>, f32>;
pub type BranchF32Lt_Sr = CmpBranchOp<Slot, Reg<f32>>;
pub type BranchF32Lt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF32Lt_Si = CmpBranchOp<Slot, f32>;
pub type BranchF32Lt_Ir = CmpBranchOp<f32, Reg<f32>>;
pub type BranchF32Lt_Is = CmpBranchOp<f32, Slot>;
pub type BranchF32NotLt_Rs = CmpBranchOp<Reg<f32>, Slot>;
pub type BranchF32NotLt_Ri = CmpBranchOp<Reg<f32>, f32>;
pub type BranchF32NotLt_Sr = CmpBranchOp<Slot, Reg<f32>>;
pub type BranchF32NotLt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF32NotLt_Si = CmpBranchOp<Slot, f32>;
pub type BranchF32NotLt_Ir = CmpBranchOp<f32, Reg<f32>>;
pub type BranchF32NotLt_Is = CmpBranchOp<f32, Slot>;
pub type BranchF32Le_Rs = CmpBranchOp<Reg<f32>, Slot>;
pub type BranchF32Le_Ri = CmpBranchOp<Reg<f32>, f32>;
pub type BranchF32Le_Sr = CmpBranchOp<Slot, Reg<f32>>;
pub type BranchF32Le_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF32Le_Si = CmpBranchOp<Slot, f32>;
pub type BranchF32Le_Ir = CmpBranchOp<f32, Reg<f32>>;
pub type BranchF32Le_Is = CmpBranchOp<f32, Slot>;
pub type BranchF32NotLe_Rs = CmpBranchOp<Reg<f32>, Slot>;
pub type BranchF32NotLe_Ri = CmpBranchOp<Reg<f32>, f32>;
pub type BranchF32NotLe_Sr = CmpBranchOp<Slot, Reg<f32>>;
pub type BranchF32NotLe_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF32NotLe_Si = CmpBranchOp<Slot, f32>;
pub type BranchF32NotLe_Ir = CmpBranchOp<f32, Reg<f32>>;
pub type BranchF32NotLe_Is = CmpBranchOp<f32, Slot>;
pub type BranchF64Eq_Rs = CmpBranchOp<Reg<f64>, Slot>;
pub type BranchF64Eq_Ri = CmpBranchOp<Reg<f64>, f64>;
pub type BranchF64Eq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF64Eq_Si = CmpBranchOp<Slot, f64>;
pub type BranchF64NotEq_Rs = CmpBranchOp<Reg<f64>, Slot>;
pub type BranchF64NotEq_Ri = CmpBranchOp<Reg<f64>, f64>;
pub type BranchF64NotEq_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF64NotEq_Si = CmpBranchOp<Slot, f64>;
pub type BranchF64Lt_Rs = CmpBranchOp<Reg<f64>, Slot>;
pub type BranchF64Lt_Ri = CmpBranchOp<Reg<f64>, f64>;
pub type BranchF64Lt_Sr = CmpBranchOp<Slot, Reg<f64>>;
pub type BranchF64Lt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF64Lt_Si = CmpBranchOp<Slot, f64>;
pub type BranchF64Lt_Ir = CmpBranchOp<f64, Reg<f64>>;
pub type BranchF64Lt_Is = CmpBranchOp<f64, Slot>;
pub type BranchF64NotLt_Rs = CmpBranchOp<Reg<f64>, Slot>;
pub type BranchF64NotLt_Ri = CmpBranchOp<Reg<f64>, f64>;
pub type BranchF64NotLt_Sr = CmpBranchOp<Slot, Reg<f64>>;
pub type BranchF64NotLt_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF64NotLt_Si = CmpBranchOp<Slot, f64>;
pub type BranchF64NotLt_Ir = CmpBranchOp<f64, Reg<f64>>;
pub type BranchF64NotLt_Is = CmpBranchOp<f64, Slot>;
pub type BranchF64Le_Rs = CmpBranchOp<Reg<f64>, Slot>;
pub type BranchF64Le_Ri = CmpBranchOp<Reg<f64>, f64>;
pub type BranchF64Le_Sr = CmpBranchOp<Slot, Reg<f64>>;
pub type BranchF64Le_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF64Le_Si = CmpBranchOp<Slot, f64>;
pub type BranchF64Le_Ir = CmpBranchOp<f64, Reg<f64>>;
pub type BranchF64Le_Is = CmpBranchOp<f64, Slot>;
pub type BranchF64NotLe_Rs = CmpBranchOp<Reg<f64>, Slot>;
pub type BranchF64NotLe_Ri = CmpBranchOp<Reg<f64>, f64>;
pub type BranchF64NotLe_Sr = CmpBranchOp<Slot, Reg<f64>>;
pub type BranchF64NotLe_Ss = CmpBranchOp<Slot, Slot>;
pub type BranchF64NotLe_Si = CmpBranchOp<Slot, f64>;
pub type BranchF64NotLe_Ir = CmpBranchOp<f64, Reg<f64>>;
pub type BranchF64NotLe_Is = CmpBranchOp<f64, Slot>;
pub type U32Select_Rrri = SelectOp<Reg<i64>, Reg<i64>, Reg<i64>, u32>;
pub type U32Select_Rrsi = SelectOp<Reg<i64>, Reg<i64>, Slot, u32>;
pub type U32Select_Rrir = SelectOp<Reg<i64>, Reg<i64>, u32, Reg<i64>>;
pub type U32Select_Rris = SelectOp<Reg<i64>, Reg<i64>, u32, Slot>;
pub type U32Select_Rrii = SelectOp<Reg<i64>, Reg<i64>, u32, u32>;
pub type U32Select_Rsri = SelectOp<Reg<i64>, Slot, Reg<i64>, u32>;
pub type U32Select_Rssi = SelectOp<Reg<i64>, Slot, Slot, u32>;
pub type U32Select_Rsir = SelectOp<Reg<i64>, Slot, u32, Reg<i64>>;
pub type U32Select_Rsis = SelectOp<Reg<i64>, Slot, u32, Slot>;
pub type U32Select_Rsii = SelectOp<Reg<i64>, Slot, u32, u32>;
pub type U64Select_Rrrs = SelectOp<Reg<i64>, Reg<i64>, Reg<i64>, Slot>;
pub type U64Select_Rrri = SelectOp<Reg<i64>, Reg<i64>, Reg<i64>, u64>;
pub type U64Select_Rrsr = SelectOp<Reg<i64>, Reg<i64>, Slot, Reg<i64>>;
pub type U64Select_Rrss = SelectOp<Reg<i64>, Reg<i64>, Slot, Slot>;
pub type U64Select_Rrsi = SelectOp<Reg<i64>, Reg<i64>, Slot, u64>;
pub type U64Select_Rrir = SelectOp<Reg<i64>, Reg<i64>, u64, Reg<i64>>;
pub type U64Select_Rris = SelectOp<Reg<i64>, Reg<i64>, u64, Slot>;
pub type U64Select_Rrii = SelectOp<Reg<i64>, Reg<i64>, u64, u64>;
pub type U64Select_Rsrs = SelectOp<Reg<i64>, Slot, Reg<i64>, Slot>;
pub type U64Select_Rsri = SelectOp<Reg<i64>, Slot, Reg<i64>, u64>;
pub type U64Select_Rssr = SelectOp<Reg<i64>, Slot, Slot, Reg<i64>>;
pub type U64Select_Rsss = SelectOp<Reg<i64>, Slot, Slot, Slot>;
pub type U64Select_Rssi = SelectOp<Reg<i64>, Slot, Slot, u64>;
pub type U64Select_Rsir = SelectOp<Reg<i64>, Slot, u64, Reg<i64>>;
pub type U64Select_Rsis = SelectOp<Reg<i64>, Slot, u64, Slot>;
pub type U64Select_Rsii = SelectOp<Reg<i64>, Slot, u64, u64>;
pub type F32Select_Rrrs = SelectOp<Reg<f32>, Reg<i64>, Reg<f32>, Slot>;
pub type F32Select_Rrri = SelectOp<Reg<f32>, Reg<i64>, Reg<f32>, f32>;
pub type F32Select_Rrsr = SelectOp<Reg<f32>, Reg<i64>, Slot, Reg<f32>>;
pub type F32Select_Rrss = SelectOp<Reg<f32>, Reg<i64>, Slot, Slot>;
pub type F32Select_Rrsi = SelectOp<Reg<f32>, Reg<i64>, Slot, f32>;
pub type F32Select_Rrir = SelectOp<Reg<f32>, Reg<i64>, f32, Reg<f32>>;
pub type F32Select_Rris = SelectOp<Reg<f32>, Reg<i64>, f32, Slot>;
pub type F32Select_Rrii = SelectOp<Reg<f32>, Reg<i64>, f32, f32>;
pub type F32Select_Rsrs = SelectOp<Reg<f32>, Slot, Reg<f32>, Slot>;
pub type F32Select_Rsri = SelectOp<Reg<f32>, Slot, Reg<f32>, f32>;
pub type F32Select_Rssr = SelectOp<Reg<f32>, Slot, Slot, Reg<f32>>;
pub type F32Select_Rsss = SelectOp<Reg<f32>, Slot, Slot, Slot>;
pub type F32Select_Rssi = SelectOp<Reg<f32>, Slot, Slot, f32>;
pub type F32Select_Rsir = SelectOp<Reg<f32>, Slot, f32, Reg<f32>>;
pub type F32Select_Rsis = SelectOp<Reg<f32>, Slot, f32, Slot>;
pub type F32Select_Rsii = SelectOp<Reg<f32>, Slot, f32, f32>;
pub type F64Select_Rrrs = SelectOp<Reg<f64>, Reg<i64>, Reg<f64>, Slot>;
pub type F64Select_Rrri = SelectOp<Reg<f64>, Reg<i64>, Reg<f64>, f64>;
pub type F64Select_Rrsr = SelectOp<Reg<f64>, Reg<i64>, Slot, Reg<f64>>;
pub type F64Select_Rrss = SelectOp<Reg<f64>, Reg<i64>, Slot, Slot>;
pub type F64Select_Rrsi = SelectOp<Reg<f64>, Reg<i64>, Slot, f64>;
pub type F64Select_Rrir = SelectOp<Reg<f64>, Reg<i64>, f64, Reg<f64>>;
pub type F64Select_Rris = SelectOp<Reg<f64>, Reg<i64>, f64, Slot>;
pub type F64Select_Rrii = SelectOp<Reg<f64>, Reg<i64>, f64, f64>;
pub type F64Select_Rsrs = SelectOp<Reg<f64>, Slot, Reg<f64>, Slot>;
pub type F64Select_Rsri = SelectOp<Reg<f64>, Slot, Reg<f64>, f64>;
pub type F64Select_Rssr = SelectOp<Reg<f64>, Slot, Slot, Reg<f64>>;
pub type F64Select_Rsss = SelectOp<Reg<f64>, Slot, Slot, Slot>;
pub type F64Select_Rssi = SelectOp<Reg<f64>, Slot, Slot, f64>;
pub type F64Select_Rsir = SelectOp<Reg<f64>, Slot, f64, Reg<f64>>;
pub type F64Select_Rsis = SelectOp<Reg<f64>, Slot, f64, Slot>;
pub type F64Select_Rsii = SelectOp<Reg<f64>, Slot, f64, f64>;
pub type U32Load_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U32LoadMem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U32LoadMem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U32Load_Rs = LoadOp<Reg<i64>,Slot>;
pub type U32LoadMem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U32LoadMem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U32Load_Ri = LoadAtOp<Reg<i64>>;
pub type U64Load_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U64LoadMem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U64LoadMem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U64Load_Rs = LoadOp<Reg<i64>,Slot>;
pub type U64LoadMem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U64LoadMem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U64Load_Ri = LoadAtOp<Reg<i64>>;
pub type F32Load_Rr = LoadOp<Reg<f32>,Reg<i64>>;
pub type F32LoadMem0Offset16_Rr = LoadOpMem0Offset16<Reg<f32>,Reg<i64>>;
pub type F32LoadMem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<f32>,Reg<i64>>;
pub type F32Load_Rs = LoadOp<Reg<f32>,Slot>;
pub type F32LoadMem0Offset16_Rs = LoadOpMem0Offset16<Reg<f32>,Slot>;
pub type F32LoadMem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<f32>,Slot>;
pub type F32Load_Ri = LoadAtOp<Reg<f32>>;
pub type F64Load_Rr = LoadOp<Reg<f64>,Reg<i64>>;
pub type F64LoadMem0Offset16_Rr = LoadOpMem0Offset16<Reg<f64>,Reg<i64>>;
pub type F64LoadMem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<f64>,Reg<i64>>;
pub type F64Load_Rs = LoadOp<Reg<f64>,Slot>;
pub type F64LoadMem0Offset16_Rs = LoadOpMem0Offset16<Reg<f64>,Slot>;
pub type F64LoadMem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<f64>,Slot>;
pub type F64Load_Ri = LoadAtOp<Reg<f64>>;
pub type I32LoadExtend8_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type I32LoadExtend8Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type I32LoadExtend8Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type I32LoadExtend8_Rs = LoadOp<Reg<i64>,Slot>;
pub type I32LoadExtend8Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type I32LoadExtend8Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type I32LoadExtend8_Ri = LoadAtOp<Reg<i64>>;
pub type I32LoadExtend16_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type I32LoadExtend16Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type I32LoadExtend16Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type I32LoadExtend16_Rs = LoadOp<Reg<i64>,Slot>;
pub type I32LoadExtend16Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type I32LoadExtend16Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type I32LoadExtend16_Ri = LoadAtOp<Reg<i64>>;
pub type U32LoadExtend8_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U32LoadExtend8Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U32LoadExtend8Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U32LoadExtend8_Rs = LoadOp<Reg<i64>,Slot>;
pub type U32LoadExtend8Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U32LoadExtend8Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U32LoadExtend8_Ri = LoadAtOp<Reg<i64>>;
pub type U32LoadExtend16_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U32LoadExtend16Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U32LoadExtend16Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U32LoadExtend16_Rs = LoadOp<Reg<i64>,Slot>;
pub type U32LoadExtend16Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U32LoadExtend16Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U32LoadExtend16_Ri = LoadAtOp<Reg<i64>>;
pub type I64LoadExtend8_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type I64LoadExtend8Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type I64LoadExtend8Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type I64LoadExtend8_Rs = LoadOp<Reg<i64>,Slot>;
pub type I64LoadExtend8Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type I64LoadExtend8Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type I64LoadExtend8_Ri = LoadAtOp<Reg<i64>>;
pub type I64LoadExtend16_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type I64LoadExtend16Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type I64LoadExtend16Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type I64LoadExtend16_Rs = LoadOp<Reg<i64>,Slot>;
pub type I64LoadExtend16Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type I64LoadExtend16Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type I64LoadExtend16_Ri = LoadAtOp<Reg<i64>>;
pub type I64LoadExtend32_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type I64LoadExtend32Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type I64LoadExtend32Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type I64LoadExtend32_Rs = LoadOp<Reg<i64>,Slot>;
pub type I64LoadExtend32Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type I64LoadExtend32Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type I64LoadExtend32_Ri = LoadAtOp<Reg<i64>>;
pub type U64LoadExtend8_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U64LoadExtend8Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U64LoadExtend8Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U64LoadExtend8_Rs = LoadOp<Reg<i64>,Slot>;
pub type U64LoadExtend8Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U64LoadExtend8Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U64LoadExtend8_Ri = LoadAtOp<Reg<i64>>;
pub type U64LoadExtend16_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U64LoadExtend16Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U64LoadExtend16Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U64LoadExtend16_Rs = LoadOp<Reg<i64>,Slot>;
pub type U64LoadExtend16Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U64LoadExtend16Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U64LoadExtend16_Ri = LoadAtOp<Reg<i64>>;
pub type U64LoadExtend32_Rr = LoadOp<Reg<i64>,Reg<i64>>;
pub type U64LoadExtend32Mem0Offset16_Rr = LoadOpMem0Offset16<Reg<i64>,Reg<i64>>;
pub type U64LoadExtend32Mem0Offset16_Rs_r = LoadOpMem0Offset16<SlotAndReg<i64>,Reg<i64>>;
pub type U64LoadExtend32_Rs = LoadOp<Reg<i64>,Slot>;
pub type U64LoadExtend32Mem0Offset16_Rs = LoadOpMem0Offset16<Reg<i64>,Slot>;
pub type U64LoadExtend32Mem0Offset16_Rs_s = LoadOpMem0Offset16<SlotAndReg<i64>,Slot>;
pub type U64LoadExtend32_Ri = LoadAtOp<Reg<i64>>;
pub type U32Store_Rs = StoreOp<Reg<i64>,Slot>;
pub type U32StoreMem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type U32Store_Ri = StoreOp<Reg<i64>,u32>;
pub type U32StoreMem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,u32>;
pub type U32Store_Sr = StoreOp<Slot,Reg<i64>>;
pub type U32StoreMem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type U32Store_Ss = StoreOp<Slot,Slot>;
pub type U32StoreMem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type U32Store_Si = StoreOp<Slot,u32>;
pub type U32StoreMem0Offset16_Si = StoreOpMem0Offset16<Slot,u32>;
pub type U32Store_Ir = StoreAtOp<Reg<i64>>;
pub type U32Store_Is = StoreAtOp<Slot>;
pub type U32Store_Ii = StoreAtOp<u32>;
pub type U64Store_Rs = StoreOp<Reg<i64>,Slot>;
pub type U64StoreMem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type U64Store_Ri = StoreOp<Reg<i64>,u64>;
pub type U64StoreMem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,u64>;
pub type U64Store_Sr = StoreOp<Slot,Reg<i64>>;
pub type U64StoreMem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type U64Store_Ss = StoreOp<Slot,Slot>;
pub type U64StoreMem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type U64Store_Si = StoreOp<Slot,u64>;
pub type U64StoreMem0Offset16_Si = StoreOpMem0Offset16<Slot,u64>;
pub type U64Store_Ir = StoreAtOp<Reg<i64>>;
pub type U64Store_Is = StoreAtOp<Slot>;
pub type U64Store_Ii = StoreAtOp<u64>;
pub type F32Store_Rr = StoreOp<Reg<i64>,Reg<f32>>;
pub type F32StoreMem0Offset16_Rr = StoreOpMem0Offset16<Reg<i64>,Reg<f32>>;
pub type F32Store_Sr = StoreOp<Slot,Reg<f32>>;
pub type F32StoreMem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<f32>>;
pub type F32Store_Ir = StoreAtOp<Reg<f32>>;
pub type F64Store_Rr = StoreOp<Reg<i64>,Reg<f64>>;
pub type F64StoreMem0Offset16_Rr = StoreOpMem0Offset16<Reg<i64>,Reg<f64>>;
pub type F64Store_Sr = StoreOp<Slot,Reg<f64>>;
pub type F64StoreMem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<f64>>;
pub type F64Store_Ir = StoreAtOp<Reg<f64>>;
pub type I32StoreWrap8_Rs = StoreOp<Reg<i64>,Slot>;
pub type I32StoreWrap8Mem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type I32StoreWrap8_Ri = StoreOp<Reg<i64>,i8>;
pub type I32StoreWrap8Mem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,i8>;
pub type I32StoreWrap8_Sr = StoreOp<Slot,Reg<i64>>;
pub type I32StoreWrap8Mem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type I32StoreWrap8_Ss = StoreOp<Slot,Slot>;
pub type I32StoreWrap8Mem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type I32StoreWrap8_Si = StoreOp<Slot,i8>;
pub type I32StoreWrap8Mem0Offset16_Si = StoreOpMem0Offset16<Slot,i8>;
pub type I32StoreWrap8_Ir = StoreAtOp<Reg<i64>>;
pub type I32StoreWrap8_Is = StoreAtOp<Slot>;
pub type I32StoreWrap8_Ii = StoreAtOp<i8>;
pub type I32StoreWrap16_Rs = StoreOp<Reg<i64>,Slot>;
pub type I32StoreWrap16Mem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type I32StoreWrap16_Ri = StoreOp<Reg<i64>,i16>;
pub type I32StoreWrap16Mem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,i16>;
pub type I32StoreWrap16_Sr = StoreOp<Slot,Reg<i64>>;
pub type I32StoreWrap16Mem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type I32StoreWrap16_Ss = StoreOp<Slot,Slot>;
pub type I32StoreWrap16Mem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type I32StoreWrap16_Si = StoreOp<Slot,i16>;
pub type I32StoreWrap16Mem0Offset16_Si = StoreOpMem0Offset16<Slot,i16>;
pub type I32StoreWrap16_Ir = StoreAtOp<Reg<i64>>;
pub type I32StoreWrap16_Is = StoreAtOp<Slot>;
pub type I32StoreWrap16_Ii = StoreAtOp<i16>;
pub type I64StoreWrap8_Rs = StoreOp<Reg<i64>,Slot>;
pub type I64StoreWrap8Mem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type I64StoreWrap8_Ri = StoreOp<Reg<i64>,i8>;
pub type I64StoreWrap8Mem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,i8>;
pub type I64StoreWrap8_Sr = StoreOp<Slot,Reg<i64>>;
pub type I64StoreWrap8Mem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type I64StoreWrap8_Ss = StoreOp<Slot,Slot>;
pub type I64StoreWrap8Mem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type I64StoreWrap8_Si = StoreOp<Slot,i8>;
pub type I64StoreWrap8Mem0Offset16_Si = StoreOpMem0Offset16<Slot,i8>;
pub type I64StoreWrap8_Ir = StoreAtOp<Reg<i64>>;
pub type I64StoreWrap8_Is = StoreAtOp<Slot>;
pub type I64StoreWrap8_Ii = StoreAtOp<i8>;
pub type I64StoreWrap16_Rs = StoreOp<Reg<i64>,Slot>;
pub type I64StoreWrap16Mem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type I64StoreWrap16_Ri = StoreOp<Reg<i64>,i16>;
pub type I64StoreWrap16Mem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,i16>;
pub type I64StoreWrap16_Sr = StoreOp<Slot,Reg<i64>>;
pub type I64StoreWrap16Mem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type I64StoreWrap16_Ss = StoreOp<Slot,Slot>;
pub type I64StoreWrap16Mem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type I64StoreWrap16_Si = StoreOp<Slot,i16>;
pub type I64StoreWrap16Mem0Offset16_Si = StoreOpMem0Offset16<Slot,i16>;
pub type I64StoreWrap16_Ir = StoreAtOp<Reg<i64>>;
pub type I64StoreWrap16_Is = StoreAtOp<Slot>;
pub type I64StoreWrap16_Ii = StoreAtOp<i16>;
pub type I64StoreWrap32_Rs = StoreOp<Reg<i64>,Slot>;
pub type I64StoreWrap32Mem0Offset16_Rs = StoreOpMem0Offset16<Reg<i64>,Slot>;
pub type I64StoreWrap32_Ri = StoreOp<Reg<i64>,i32>;
pub type I64StoreWrap32Mem0Offset16_Ri = StoreOpMem0Offset16<Reg<i64>,i32>;
pub type I64StoreWrap32_Sr = StoreOp<Slot,Reg<i64>>;
pub type I64StoreWrap32Mem0Offset16_Sr = StoreOpMem0Offset16<Slot,Reg<i64>>;
pub type I64StoreWrap32_Ss = StoreOp<Slot,Slot>;
pub type I64StoreWrap32Mem0Offset16_Ss = StoreOpMem0Offset16<Slot,Slot>;
pub type I64StoreWrap32_Si = StoreOp<Slot,i32>;
pub type I64StoreWrap32Mem0Offset16_Si = StoreOpMem0Offset16<Slot,i32>;
pub type I64StoreWrap32_Ir = StoreAtOp<Reg<i64>>;
pub type I64StoreWrap32_Is = StoreAtOp<Slot>;
pub type I64StoreWrap32_Ii = StoreAtOp<i32>;
pub struct Trap {
    pub trap_code: TrapCode
}
impl Decode for Trap {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            trap_code: Decode::decode(decoder)?
        })
    }
}
pub struct ConsumeFuel {
    pub fuel: BlockFuel
}
impl Decode for ConsumeFuel {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            fuel: Decode::decode(decoder)?
        })
    }
}
pub struct Branch {
    pub offset: BranchOffset
}
impl Decode for Branch {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            offset: Decode::decode(decoder)?
        })
    }
}
pub type BranchTable_R = BranchTableOp<Reg<i64>, ()>;
pub type BranchTable_S = BranchTableOp<Slot, ()>;
pub type BranchTableSpan_R = BranchTableOp<Reg<i64>, BoundedSlotSpan>;
pub type BranchTableSpan_S = BranchTableOp<Slot, BoundedSlotSpan>;
pub type U32Copy_Ri = UnaryOp<Reg<i64>, u32>;
pub type U32Copy_Si = UnaryOp<Slot, u32>;
pub type U64Copy_Rs = UnaryOp<Reg<i64>, Slot>;
pub type U64Copy_Ri = UnaryOp<Reg<i64>, u64>;
pub type U64Copy_Sr = UnaryOp<Slot, Reg<i64>>;
pub type U64Copy_Ss = UnaryOp<Slot, Slot>;
pub type U64Copy_Si = UnaryOp<Slot, u64>;
pub type F32Copy_Ri = UnaryOp<Reg<f32>, f32>;
pub type F32Copy_Rs = UnaryOp<Reg<f32>, Slot>;
pub type F32Copy_Sr = UnaryOp<Slot, Reg<f32>>;
pub type F64Copy_Rs = UnaryOp<Reg<f64>, Slot>;
pub type F64Copy_Ri = UnaryOp<Reg<f64>, f64>;
pub type F64Copy_Sr = UnaryOp<Slot, Reg<f64>>;
pub type U64Copy_S0s1 = UnaryOp<Local<0>, Local<1>>;
pub type U64Copy_S0s2 = UnaryOp<Local<0>, Local<2>>;
pub type U64Copy_S0s3 = UnaryOp<Local<0>, Local<3>>;
pub type U64Copy_S0s4 = UnaryOp<Local<0>, Local<4>>;
pub type U64Copy_S0s5 = UnaryOp<Local<0>, Local<5>>;
pub type U64Copy_S1s0 = UnaryOp<Local<1>, Local<0>>;
pub type U64Copy_S1s2 = UnaryOp<Local<1>, Local<2>>;
pub type U64Copy_S1s3 = UnaryOp<Local<1>, Local<3>>;
pub type U64Copy_S1s4 = UnaryOp<Local<1>, Local<4>>;
pub type U64Copy_S1s5 = UnaryOp<Local<1>, Local<5>>;
pub type U64Copy_S2s0 = UnaryOp<Local<2>, Local<0>>;
pub type U64Copy_S2s1 = UnaryOp<Local<2>, Local<1>>;
pub type U64Copy_S2s3 = UnaryOp<Local<2>, Local<3>>;
pub type U64Copy_S2s4 = UnaryOp<Local<2>, Local<4>>;
pub type U64Copy_S2s5 = UnaryOp<Local<2>, Local<5>>;
pub type U64Copy_S3s0 = UnaryOp<Local<3>, Local<0>>;
pub type U64Copy_S3s1 = UnaryOp<Local<3>, Local<1>>;
pub type U64Copy_S3s2 = UnaryOp<Local<3>, Local<2>>;
pub type U64Copy_S3s4 = UnaryOp<Local<3>, Local<4>>;
pub type U64Copy_S3s5 = UnaryOp<Local<3>, Local<5>>;
pub type U64Copy_S4s0 = UnaryOp<Local<4>, Local<0>>;
pub type U64Copy_S4s1 = UnaryOp<Local<4>, Local<1>>;
pub type U64Copy_S4s2 = UnaryOp<Local<4>, Local<2>>;
pub type U64Copy_S4s3 = UnaryOp<Local<4>, Local<3>>;
pub type U64Copy_S4s5 = UnaryOp<Local<4>, Local<5>>;
pub type U64Copy_S5s0 = UnaryOp<Local<5>, Local<0>>;
pub type U64Copy_S5s1 = UnaryOp<Local<5>, Local<1>>;
pub type U64Copy_S5s2 = UnaryOp<Local<5>, Local<2>>;
pub type U64Copy_S5s3 = UnaryOp<Local<5>, Local<3>>;
pub type U64Copy_S5s4 = UnaryOp<Local<5>, Local<4>>;
pub type F32ReinterpretI32_Rr = UnaryOp<Reg<f32>, Reg<i64>>;
pub type I32ReinterpretF32_Rr = UnaryOp<Reg<i64>, Reg<f32>>;
pub type F64ReinterpretI64_Rr = UnaryOp<Reg<f64>, Reg<i64>>;
pub type I64ReinterpretF64_Rr = UnaryOp<Reg<i64>, Reg<f64>>;
pub type U64Copy_S0r = UnaryOp<Local<0>, Reg<i64>>;
pub type U64Copy_S1r = UnaryOp<Local<1>, Reg<i64>>;
pub type U64Copy_S2r = UnaryOp<Local<2>, Reg<i64>>;
pub type U64Copy_S3r = UnaryOp<Local<3>, Reg<i64>>;
pub type U64Copy_S4r = UnaryOp<Local<4>, Reg<i64>>;
pub type U64Copy_S5r = UnaryOp<Local<5>, Reg<i64>>;
pub type U64Copy_S6r = UnaryOp<Local<6>, Reg<i64>>;
pub type U64Copy_S7r = UnaryOp<Local<7>, Reg<i64>>;
pub type U64Copy_S8r = UnaryOp<Local<8>, Reg<i64>>;
pub type U64Copy_S9r = UnaryOp<Local<9>, Reg<i64>>;
pub type F32Copy_S0r = UnaryOp<Local<0>, Reg<f32>>;
pub type F32Copy_S1r = UnaryOp<Local<1>, Reg<f32>>;
pub type F32Copy_S2r = UnaryOp<Local<2>, Reg<f32>>;
pub type F32Copy_S3r = UnaryOp<Local<3>, Reg<f32>>;
pub type F32Copy_S4r = UnaryOp<Local<4>, Reg<f32>>;
pub type F32Copy_S5r = UnaryOp<Local<5>, Reg<f32>>;
pub type F32Copy_S6r = UnaryOp<Local<6>, Reg<f32>>;
pub type F32Copy_S7r = UnaryOp<Local<7>, Reg<f32>>;
pub type F32Copy_S8r = UnaryOp<Local<8>, Reg<f32>>;
pub type F32Copy_S9r = UnaryOp<Local<9>, Reg<f32>>;
pub type F64Copy_S0r = UnaryOp<Local<0>, Reg<f64>>;
pub type F64Copy_S1r = UnaryOp<Local<1>, Reg<f64>>;
pub type F64Copy_S2r = UnaryOp<Local<2>, Reg<f64>>;
pub type F64Copy_S3r = UnaryOp<Local<3>, Reg<f64>>;
pub type F64Copy_S4r = UnaryOp<Local<4>, Reg<f64>>;
pub type F64Copy_S5r = UnaryOp<Local<5>, Reg<f64>>;
pub type F64Copy_S6r = UnaryOp<Local<6>, Reg<f64>>;
pub type F64Copy_S7r = UnaryOp<Local<7>, Reg<f64>>;
pub type F64Copy_S8r = UnaryOp<Local<8>, Reg<f64>>;
pub type F64Copy_S9r = UnaryOp<Local<9>, Reg<f64>>;
pub struct RefFunc {
    pub result: Reg<i64>,
    pub func: Func
}
impl Decode for RefFunc {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            result: Decode::decode(decoder)?,
            func: Decode::decode(decoder)?
        })
    }
}
pub struct CallInternal {
    pub params: BoundedSlotSpan,
    pub func: InternalFunc
}
impl Decode for CallInternal {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            params: Decode::decode(decoder)?,
            func: Decode::decode(decoder)?
        })
    }
}
pub struct CallImported {
    pub params: BoundedSlotSpan,
    pub func: Func
}
impl Decode for CallImported {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            params: Decode::decode(decoder)?,
            func: Decode::decode(decoder)?
        })
    }
}
pub type CallIndirect_R = CallIndirect<Reg<i64>>;
pub type CallIndirect_S = CallIndirect<Slot>;
pub type ReturnCallIndirect_R = CallIndirect<Reg<i64>>;
pub type ReturnCallIndirect_S = CallIndirect<Slot>;
pub struct ReturnCallInternal {
    pub params: BoundedSlotSpan,
    pub func: InternalFunc
}
impl Decode for ReturnCallInternal {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            params: Decode::decode(decoder)?,
            func: Decode::decode(decoder)?
        })
    }
}
pub struct ReturnCallImported {
    pub params: BoundedSlotSpan,
    pub func: Func
}
impl Decode for ReturnCallImported {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            params: Decode::decode(decoder)?,
            func: Decode::decode(decoder)?
        })
    }
}
pub type GlobalGetU64_R = GlobalGet<Reg<i64>>;
pub type GlobalGetF32_R = GlobalGet<Reg<f32>>;
pub type GlobalGetF64_R = GlobalGet<Reg<f64>>;
pub type GlobalSetU32_I = GlobalSet<u32>;
pub type GlobalSetU64_R = GlobalSet<Reg<i64>>;
pub type GlobalSetU64_S = GlobalSet<Slot>;
pub type GlobalSetU64_I = GlobalSet<u64>;
pub type GlobalSetF32_R = GlobalSet<Reg<f32>>;
pub type GlobalSetF64_R = GlobalSet<Reg<f64>>;
pub struct DataDrop {
    pub data: Data
}
impl Decode for DataDrop {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            data: Decode::decode(decoder)?
        })
    }
}
pub struct MemorySize {
    pub result: Reg<i64>,
    pub memory: Memory
}
impl Decode for MemorySize {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            result: Decode::decode(decoder)?,
            memory: Decode::decode(decoder)?
        })
    }
}
pub struct MemoryGrow {
    pub result: Reg<i64>,
    pub delta: Slot,
    pub memory: Memory
}
impl Decode for MemoryGrow {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            result: Decode::decode(decoder)?,
            delta: Decode::decode(decoder)?,
            memory: Decode::decode(decoder)?
        })
    }
}
pub struct MemoryCopy {
    pub dst_memory: Memory,
    pub src_memory: Memory,
    pub dst: Slot,
    pub src: Slot,
    pub len: Slot
}
impl Decode for MemoryCopy {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            dst_memory: Decode::decode(decoder)?,
            src_memory: Decode::decode(decoder)?,
            dst: Decode::decode(decoder)?,
            src: Decode::decode(decoder)?,
            len: Decode::decode(decoder)?
        })
    }
}
pub struct MemoryFill {
    pub memory: Memory,
    pub dst: Slot,
    pub len: Slot,
    pub value: Slot
}
impl Decode for MemoryFill {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            memory: Decode::decode(decoder)?,
            dst: Decode::decode(decoder)?,
            len: Decode::decode(decoder)?,
            value: Decode::decode(decoder)?
        })
    }
}
pub struct MemoryInit {
    pub memory: Memory,
    pub data: Data,
    pub dst: Slot,
    pub src: Slot,
    pub len: Slot
}
impl Decode for MemoryInit {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            memory: Decode::decode(decoder)?,
            data: Decode::decode(decoder)?,
            dst: Decode::decode(decoder)?,
            src: Decode::decode(decoder)?,
            len: Decode::decode(decoder)?
        })
    }
}
pub type TableGet_Rr = TableGet<Reg<i64>>;
pub type TableSet_Rs = TableSet<Reg<i64>, Slot>;
pub type TableSet_Ri = TableSet<Reg<i64>, u32>;
pub type TableGet_Rs = TableGet<Slot>;
pub type TableSet_Sr = TableSet<Slot, Reg<i64>>;
pub type TableSet_Ss = TableSet<Slot, Slot>;
pub type TableSet_Si = TableSet<Slot, u32>;
pub type TableGet_Ri = TableGet<u32>;
pub type TableSet_Ir = TableSet<u32, Reg<i64>>;
pub type TableSet_Is = TableSet<u32, Slot>;
pub type TableSet_Ii = TableSet<u32, u32>;
pub struct TableSize {
    pub result: Reg<i64>,
    pub table: Table
}
impl Decode for TableSize {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            result: Decode::decode(decoder)?,
            table: Decode::decode(decoder)?
        })
    }
}
pub struct TableGrow {
    pub result: Reg<i64>,
    pub delta: Slot,
    pub value: Slot,
    pub table: Table
}
impl Decode for TableGrow {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            result: Decode::decode(decoder)?,
            delta: Decode::decode(decoder)?,
            value: Decode::decode(decoder)?,
            table: Decode::decode(decoder)?
        })
    }
}
pub struct TableCopy {
    pub dst_table: Table,
    pub src_table: Table,
    pub dst: Slot,
    pub src: Slot,
    pub len: Slot
}
impl Decode for TableCopy {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            dst_table: Decode::decode(decoder)?,
            src_table: Decode::decode(decoder)?,
            dst: Decode::decode(decoder)?,
            src: Decode::decode(decoder)?,
            len: Decode::decode(decoder)?
        })
    }
}
pub struct TableFill {
    pub table: Table,
    pub dst: Slot,
    pub len: Slot,
    pub value: Slot
}
impl Decode for TableFill {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            table: Decode::decode(decoder)?,
            dst: Decode::decode(decoder)?,
            len: Decode::decode(decoder)?,
            value: Decode::decode(decoder)?
        })
    }
}
pub struct TableInit {
    pub table: Table,
    pub elem: Elem,
    pub dst: Slot,
    pub src: Slot,
    pub len: Slot
}
impl Decode for TableInit {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            table: Decode::decode(decoder)?,
            elem: Decode::decode(decoder)?,
            dst: Decode::decode(decoder)?,
            src: Decode::decode(decoder)?,
            len: Decode::decode(decoder)?
        })
    }
}
pub struct ElemDrop {
    pub elem: Elem
}
impl Decode for ElemDrop {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            elem: Decode::decode(decoder)?
        })
    }
}
pub struct I64Add128 {
    pub results: FixedSlotSpan<2>,
    pub lhs_lo: Slot,
    pub lhs_hi: Slot,
    pub rhs_lo: Slot,
    pub rhs_hi: Slot
}
impl Decode for I64Add128 {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            results: Decode::decode(decoder)?,
            lhs_lo: Decode::decode(decoder)?,
            lhs_hi: Decode::decode(decoder)?,
            rhs_lo: Decode::decode(decoder)?,
            rhs_hi: Decode::decode(decoder)?
        })
    }
}
pub struct I64Sub128 {
    pub results: FixedSlotSpan<2>,
    pub lhs_lo: Slot,
    pub lhs_hi: Slot,
    pub rhs_lo: Slot,
    pub rhs_hi: Slot
}
impl Decode for I64Sub128 {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            results: Decode::decode(decoder)?,
            lhs_lo: Decode::decode(decoder)?,
            lhs_hi: Decode::decode(decoder)?,
            rhs_lo: Decode::decode(decoder)?,
            rhs_hi: Decode::decode(decoder)?
        })
    }
}
pub struct I64MulWide {
    pub results: FixedSlotSpan<2>,
    pub lhs: Slot,
    pub rhs: Slot
}
impl Decode for I64MulWide {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            results: Decode::decode(decoder)?,
            lhs: Decode::decode(decoder)?,
            rhs: Decode::decode(decoder)?
        })
    }
}
pub struct U64MulWide {
    pub results: FixedSlotSpan<2>,
    pub lhs: Slot,
    pub rhs: Slot
}
impl Decode for U64MulWide {
    #[inline]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            results: Decode::decode(decoder)?,
            lhs: Decode::decode(decoder)?,
            rhs: Decode::decode(decoder)?
        })
    }
}
