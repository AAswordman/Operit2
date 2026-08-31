impl Encode for Op {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<E::Pos, E::Error> {
        match self {
            Self::I32Clz_Rs { result, value } => {
                (OpCode::I32Clz_Rs, result, value).encode(encoder)
            }
            Self::I32Clz_Rr { result, value } => {
                (OpCode::I32Clz_Rr, result, value).encode(encoder)
            }
            Self::I32Ctz_Rs { result, value } => {
                (OpCode::I32Ctz_Rs, result, value).encode(encoder)
            }
            Self::I32Ctz_Rr { result, value } => {
                (OpCode::I32Ctz_Rr, result, value).encode(encoder)
            }
            Self::I32Popcnt_Rs { result, value } => {
                (OpCode::I32Popcnt_Rs, result, value).encode(encoder)
            }
            Self::I32Popcnt_Rr { result, value } => {
                (OpCode::I32Popcnt_Rr, result, value).encode(encoder)
            }
            Self::I32Sext8_Rs { result, value } => {
                (OpCode::I32Sext8_Rs, result, value).encode(encoder)
            }
            Self::I32Sext8_Rr { result, value } => {
                (OpCode::I32Sext8_Rr, result, value).encode(encoder)
            }
            Self::I32Sext16_Rs { result, value } => {
                (OpCode::I32Sext16_Rs, result, value).encode(encoder)
            }
            Self::I32Sext16_Rr { result, value } => {
                (OpCode::I32Sext16_Rr, result, value).encode(encoder)
            }
            Self::I32WrapI64_Rs { result, value } => {
                (OpCode::I32WrapI64_Rs, result, value).encode(encoder)
            }
            Self::I32WrapI64_Rr { result, value } => {
                (OpCode::I32WrapI64_Rr, result, value).encode(encoder)
            }
            Self::I64Clz_Rs { result, value } => {
                (OpCode::I64Clz_Rs, result, value).encode(encoder)
            }
            Self::I64Clz_Rr { result, value } => {
                (OpCode::I64Clz_Rr, result, value).encode(encoder)
            }
            Self::I64Ctz_Rs { result, value } => {
                (OpCode::I64Ctz_Rs, result, value).encode(encoder)
            }
            Self::I64Ctz_Rr { result, value } => {
                (OpCode::I64Ctz_Rr, result, value).encode(encoder)
            }
            Self::I64Popcnt_Rs { result, value } => {
                (OpCode::I64Popcnt_Rs, result, value).encode(encoder)
            }
            Self::I64Popcnt_Rr { result, value } => {
                (OpCode::I64Popcnt_Rr, result, value).encode(encoder)
            }
            Self::I64Sext8_Rs { result, value } => {
                (OpCode::I64Sext8_Rs, result, value).encode(encoder)
            }
            Self::I64Sext8_Rr { result, value } => {
                (OpCode::I64Sext8_Rr, result, value).encode(encoder)
            }
            Self::I64Sext16_Rs { result, value } => {
                (OpCode::I64Sext16_Rs, result, value).encode(encoder)
            }
            Self::I64Sext16_Rr { result, value } => {
                (OpCode::I64Sext16_Rr, result, value).encode(encoder)
            }
            Self::I64Sext32_Rs { result, value } => {
                (OpCode::I64Sext32_Rs, result, value).encode(encoder)
            }
            Self::I64Sext32_Rr { result, value } => {
                (OpCode::I64Sext32_Rr, result, value).encode(encoder)
            }
            Self::F32Abs_Rs { result, value } => {
                (OpCode::F32Abs_Rs, result, value).encode(encoder)
            }
            Self::F32Abs_Rr { result, value } => {
                (OpCode::F32Abs_Rr, result, value).encode(encoder)
            }
            Self::F32Neg_Rs { result, value } => {
                (OpCode::F32Neg_Rs, result, value).encode(encoder)
            }
            Self::F32Neg_Rr { result, value } => {
                (OpCode::F32Neg_Rr, result, value).encode(encoder)
            }
            Self::F32Nabs_Rs { result, value } => {
                (OpCode::F32Nabs_Rs, result, value).encode(encoder)
            }
            Self::F32Nabs_Rr { result, value } => {
                (OpCode::F32Nabs_Rr, result, value).encode(encoder)
            }
            Self::F32Ceil_Rs { result, value } => {
                (OpCode::F32Ceil_Rs, result, value).encode(encoder)
            }
            Self::F32Ceil_Rr { result, value } => {
                (OpCode::F32Ceil_Rr, result, value).encode(encoder)
            }
            Self::F32Floor_Rs { result, value } => {
                (OpCode::F32Floor_Rs, result, value).encode(encoder)
            }
            Self::F32Floor_Rr { result, value } => {
                (OpCode::F32Floor_Rr, result, value).encode(encoder)
            }
            Self::F32Trunc_Rs { result, value } => {
                (OpCode::F32Trunc_Rs, result, value).encode(encoder)
            }
            Self::F32Trunc_Rr { result, value } => {
                (OpCode::F32Trunc_Rr, result, value).encode(encoder)
            }
            Self::F32Nearest_Rs { result, value } => {
                (OpCode::F32Nearest_Rs, result, value).encode(encoder)
            }
            Self::F32Nearest_Rr { result, value } => {
                (OpCode::F32Nearest_Rr, result, value).encode(encoder)
            }
            Self::F32Sqrt_Rs { result, value } => {
                (OpCode::F32Sqrt_Rs, result, value).encode(encoder)
            }
            Self::F32Sqrt_Rr { result, value } => {
                (OpCode::F32Sqrt_Rr, result, value).encode(encoder)
            }
            Self::F32ConvertI32_Rs { result, value } => {
                (OpCode::F32ConvertI32_Rs, result, value).encode(encoder)
            }
            Self::F32ConvertI32_Rr { result, value } => {
                (OpCode::F32ConvertI32_Rr, result, value).encode(encoder)
            }
            Self::F32ConvertU32_Rs { result, value } => {
                (OpCode::F32ConvertU32_Rs, result, value).encode(encoder)
            }
            Self::F32ConvertU32_Rr { result, value } => {
                (OpCode::F32ConvertU32_Rr, result, value).encode(encoder)
            }
            Self::F32ConvertI64_Rs { result, value } => {
                (OpCode::F32ConvertI64_Rs, result, value).encode(encoder)
            }
            Self::F32ConvertI64_Rr { result, value } => {
                (OpCode::F32ConvertI64_Rr, result, value).encode(encoder)
            }
            Self::F32ConvertU64_Rs { result, value } => {
                (OpCode::F32ConvertU64_Rs, result, value).encode(encoder)
            }
            Self::F32ConvertU64_Rr { result, value } => {
                (OpCode::F32ConvertU64_Rr, result, value).encode(encoder)
            }
            Self::F32DemoteF64_Rs { result, value } => {
                (OpCode::F32DemoteF64_Rs, result, value).encode(encoder)
            }
            Self::F32DemoteF64_Rr { result, value } => {
                (OpCode::F32DemoteF64_Rr, result, value).encode(encoder)
            }
            Self::F64Abs_Rs { result, value } => {
                (OpCode::F64Abs_Rs, result, value).encode(encoder)
            }
            Self::F64Abs_Rr { result, value } => {
                (OpCode::F64Abs_Rr, result, value).encode(encoder)
            }
            Self::F64Neg_Rs { result, value } => {
                (OpCode::F64Neg_Rs, result, value).encode(encoder)
            }
            Self::F64Neg_Rr { result, value } => {
                (OpCode::F64Neg_Rr, result, value).encode(encoder)
            }
            Self::F64Nabs_Rs { result, value } => {
                (OpCode::F64Nabs_Rs, result, value).encode(encoder)
            }
            Self::F64Nabs_Rr { result, value } => {
                (OpCode::F64Nabs_Rr, result, value).encode(encoder)
            }
            Self::F64Ceil_Rs { result, value } => {
                (OpCode::F64Ceil_Rs, result, value).encode(encoder)
            }
            Self::F64Ceil_Rr { result, value } => {
                (OpCode::F64Ceil_Rr, result, value).encode(encoder)
            }
            Self::F64Floor_Rs { result, value } => {
                (OpCode::F64Floor_Rs, result, value).encode(encoder)
            }
            Self::F64Floor_Rr { result, value } => {
                (OpCode::F64Floor_Rr, result, value).encode(encoder)
            }
            Self::F64Trunc_Rs { result, value } => {
                (OpCode::F64Trunc_Rs, result, value).encode(encoder)
            }
            Self::F64Trunc_Rr { result, value } => {
                (OpCode::F64Trunc_Rr, result, value).encode(encoder)
            }
            Self::F64Nearest_Rs { result, value } => {
                (OpCode::F64Nearest_Rs, result, value).encode(encoder)
            }
            Self::F64Nearest_Rr { result, value } => {
                (OpCode::F64Nearest_Rr, result, value).encode(encoder)
            }
            Self::F64Sqrt_Rs { result, value } => {
                (OpCode::F64Sqrt_Rs, result, value).encode(encoder)
            }
            Self::F64Sqrt_Rr { result, value } => {
                (OpCode::F64Sqrt_Rr, result, value).encode(encoder)
            }
            Self::F64ConvertI32_Rs { result, value } => {
                (OpCode::F64ConvertI32_Rs, result, value).encode(encoder)
            }
            Self::F64ConvertI32_Rr { result, value } => {
                (OpCode::F64ConvertI32_Rr, result, value).encode(encoder)
            }
            Self::F64ConvertU32_Rs { result, value } => {
                (OpCode::F64ConvertU32_Rs, result, value).encode(encoder)
            }
            Self::F64ConvertU32_Rr { result, value } => {
                (OpCode::F64ConvertU32_Rr, result, value).encode(encoder)
            }
            Self::F64ConvertI64_Rs { result, value } => {
                (OpCode::F64ConvertI64_Rs, result, value).encode(encoder)
            }
            Self::F64ConvertI64_Rr { result, value } => {
                (OpCode::F64ConvertI64_Rr, result, value).encode(encoder)
            }
            Self::F64ConvertU64_Rs { result, value } => {
                (OpCode::F64ConvertU64_Rs, result, value).encode(encoder)
            }
            Self::F64ConvertU64_Rr { result, value } => {
                (OpCode::F64ConvertU64_Rr, result, value).encode(encoder)
            }
            Self::F64PromoteF32_Rs { result, value } => {
                (OpCode::F64PromoteF32_Rs, result, value).encode(encoder)
            }
            Self::F64PromoteF32_Rr { result, value } => {
                (OpCode::F64PromoteF32_Rr, result, value).encode(encoder)
            }
            Self::I32TruncF32_Rs { result, value } => {
                (OpCode::I32TruncF32_Rs, result, value).encode(encoder)
            }
            Self::I32TruncF32_Rr { result, value } => {
                (OpCode::I32TruncF32_Rr, result, value).encode(encoder)
            }
            Self::U32TruncF32_Rs { result, value } => {
                (OpCode::U32TruncF32_Rs, result, value).encode(encoder)
            }
            Self::U32TruncF32_Rr { result, value } => {
                (OpCode::U32TruncF32_Rr, result, value).encode(encoder)
            }
            Self::I32TruncF64_Rs { result, value } => {
                (OpCode::I32TruncF64_Rs, result, value).encode(encoder)
            }
            Self::I32TruncF64_Rr { result, value } => {
                (OpCode::I32TruncF64_Rr, result, value).encode(encoder)
            }
            Self::U32TruncF64_Rs { result, value } => {
                (OpCode::U32TruncF64_Rs, result, value).encode(encoder)
            }
            Self::U32TruncF64_Rr { result, value } => {
                (OpCode::U32TruncF64_Rr, result, value).encode(encoder)
            }
            Self::I64TruncF32_Rs { result, value } => {
                (OpCode::I64TruncF32_Rs, result, value).encode(encoder)
            }
            Self::I64TruncF32_Rr { result, value } => {
                (OpCode::I64TruncF32_Rr, result, value).encode(encoder)
            }
            Self::U64TruncF32_Rs { result, value } => {
                (OpCode::U64TruncF32_Rs, result, value).encode(encoder)
            }
            Self::U64TruncF32_Rr { result, value } => {
                (OpCode::U64TruncF32_Rr, result, value).encode(encoder)
            }
            Self::I64TruncF64_Rs { result, value } => {
                (OpCode::I64TruncF64_Rs, result, value).encode(encoder)
            }
            Self::I64TruncF64_Rr { result, value } => {
                (OpCode::I64TruncF64_Rr, result, value).encode(encoder)
            }
            Self::U64TruncF64_Rs { result, value } => {
                (OpCode::U64TruncF64_Rs, result, value).encode(encoder)
            }
            Self::U64TruncF64_Rr { result, value } => {
                (OpCode::U64TruncF64_Rr, result, value).encode(encoder)
            }
            Self::I32TruncSatF32_Rs { result, value } => {
                (OpCode::I32TruncSatF32_Rs, result, value).encode(encoder)
            }
            Self::I32TruncSatF32_Rr { result, value } => {
                (OpCode::I32TruncSatF32_Rr, result, value).encode(encoder)
            }
            Self::U32TruncSatF32_Rs { result, value } => {
                (OpCode::U32TruncSatF32_Rs, result, value).encode(encoder)
            }
            Self::U32TruncSatF32_Rr { result, value } => {
                (OpCode::U32TruncSatF32_Rr, result, value).encode(encoder)
            }
            Self::I32TruncSatF64_Rs { result, value } => {
                (OpCode::I32TruncSatF64_Rs, result, value).encode(encoder)
            }
            Self::I32TruncSatF64_Rr { result, value } => {
                (OpCode::I32TruncSatF64_Rr, result, value).encode(encoder)
            }
            Self::U32TruncSatF64_Rs { result, value } => {
                (OpCode::U32TruncSatF64_Rs, result, value).encode(encoder)
            }
            Self::U32TruncSatF64_Rr { result, value } => {
                (OpCode::U32TruncSatF64_Rr, result, value).encode(encoder)
            }
            Self::I64TruncSatF32_Rs { result, value } => {
                (OpCode::I64TruncSatF32_Rs, result, value).encode(encoder)
            }
            Self::I64TruncSatF32_Rr { result, value } => {
                (OpCode::I64TruncSatF32_Rr, result, value).encode(encoder)
            }
            Self::U64TruncSatF32_Rs { result, value } => {
                (OpCode::U64TruncSatF32_Rs, result, value).encode(encoder)
            }
            Self::U64TruncSatF32_Rr { result, value } => {
                (OpCode::U64TruncSatF32_Rr, result, value).encode(encoder)
            }
            Self::I64TruncSatF64_Rs { result, value } => {
                (OpCode::I64TruncSatF64_Rs, result, value).encode(encoder)
            }
            Self::I64TruncSatF64_Rr { result, value } => {
                (OpCode::I64TruncSatF64_Rr, result, value).encode(encoder)
            }
            Self::U64TruncSatF64_Rs { result, value } => {
                (OpCode::U64TruncSatF64_Rs, result, value).encode(encoder)
            }
            Self::U64TruncSatF64_Rr { result, value } => {
                (OpCode::U64TruncSatF64_Rr, result, value).encode(encoder)
            }
            Self::I32Eq_Rrs { result, lhs, rhs } => {
                (OpCode::I32Eq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Eq_Rri { result, lhs, rhs } => {
                (OpCode::I32Eq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Eq_Rss { result, lhs, rhs } => {
                (OpCode::I32Eq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Eq_Rsi { result, lhs, rhs } => {
                (OpCode::I32Eq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32And_Rrs { result, lhs, rhs } => {
                (OpCode::I32And_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32And_Rri { result, lhs, rhs } => {
                (OpCode::I32And_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32And_Rss { result, lhs, rhs } => {
                (OpCode::I32And_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32And_Rsi { result, lhs, rhs } => {
                (OpCode::I32And_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Or_Rrs { result, lhs, rhs } => {
                (OpCode::I32Or_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Or_Rri { result, lhs, rhs } => {
                (OpCode::I32Or_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Or_Rss { result, lhs, rhs } => {
                (OpCode::I32Or_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Or_Rsi { result, lhs, rhs } => {
                (OpCode::I32Or_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotEq_Rrs { result, lhs, rhs } => {
                (OpCode::I32NotEq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotEq_Rri { result, lhs, rhs } => {
                (OpCode::I32NotEq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotEq_Rss { result, lhs, rhs } => {
                (OpCode::I32NotEq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotEq_Rsi { result, lhs, rhs } => {
                (OpCode::I32NotEq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotAnd_Rrs { result, lhs, rhs } => {
                (OpCode::I32NotAnd_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotAnd_Rri { result, lhs, rhs } => {
                (OpCode::I32NotAnd_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotAnd_Rss { result, lhs, rhs } => {
                (OpCode::I32NotAnd_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotAnd_Rsi { result, lhs, rhs } => {
                (OpCode::I32NotAnd_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotOr_Rrs { result, lhs, rhs } => {
                (OpCode::I32NotOr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotOr_Rri { result, lhs, rhs } => {
                (OpCode::I32NotOr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotOr_Rss { result, lhs, rhs } => {
                (OpCode::I32NotOr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32NotOr_Rsi { result, lhs, rhs } => {
                (OpCode::I32NotOr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Rrs { result, lhs, rhs } => {
                (OpCode::I32Lt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Rri { result, lhs, rhs } => {
                (OpCode::I32Lt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Rsr { result, lhs, rhs } => {
                (OpCode::I32Lt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Rss { result, lhs, rhs } => {
                (OpCode::I32Lt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Rsi { result, lhs, rhs } => {
                (OpCode::I32Lt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Rir { result, lhs, rhs } => {
                (OpCode::I32Lt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Lt_Ris { result, lhs, rhs } => {
                (OpCode::I32Lt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Rrs { result, lhs, rhs } => {
                (OpCode::I32Le_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Rri { result, lhs, rhs } => {
                (OpCode::I32Le_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Rsr { result, lhs, rhs } => {
                (OpCode::I32Le_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Rss { result, lhs, rhs } => {
                (OpCode::I32Le_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Rsi { result, lhs, rhs } => {
                (OpCode::I32Le_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Rir { result, lhs, rhs } => {
                (OpCode::I32Le_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Le_Ris { result, lhs, rhs } => {
                (OpCode::I32Le_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Rrs { result, lhs, rhs } => {
                (OpCode::U32Lt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Rri { result, lhs, rhs } => {
                (OpCode::U32Lt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Rsr { result, lhs, rhs } => {
                (OpCode::U32Lt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Rss { result, lhs, rhs } => {
                (OpCode::U32Lt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Rsi { result, lhs, rhs } => {
                (OpCode::U32Lt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Rir { result, lhs, rhs } => {
                (OpCode::U32Lt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U32Lt_Ris { result, lhs, rhs } => {
                (OpCode::U32Lt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Rrs { result, lhs, rhs } => {
                (OpCode::U32Le_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Rri { result, lhs, rhs } => {
                (OpCode::U32Le_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Rsr { result, lhs, rhs } => {
                (OpCode::U32Le_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Rss { result, lhs, rhs } => {
                (OpCode::U32Le_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Rsi { result, lhs, rhs } => {
                (OpCode::U32Le_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Rir { result, lhs, rhs } => {
                (OpCode::U32Le_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U32Le_Ris { result, lhs, rhs } => {
                (OpCode::U32Le_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Eq_Rrs { result, lhs, rhs } => {
                (OpCode::I64Eq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Eq_Rri { result, lhs, rhs } => {
                (OpCode::I64Eq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Eq_Rss { result, lhs, rhs } => {
                (OpCode::I64Eq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Eq_Rsi { result, lhs, rhs } => {
                (OpCode::I64Eq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64And_Rrs { result, lhs, rhs } => {
                (OpCode::I64And_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64And_Rri { result, lhs, rhs } => {
                (OpCode::I64And_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64And_Rss { result, lhs, rhs } => {
                (OpCode::I64And_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64And_Rsi { result, lhs, rhs } => {
                (OpCode::I64And_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Or_Rrs { result, lhs, rhs } => {
                (OpCode::I64Or_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Or_Rri { result, lhs, rhs } => {
                (OpCode::I64Or_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Or_Rss { result, lhs, rhs } => {
                (OpCode::I64Or_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Or_Rsi { result, lhs, rhs } => {
                (OpCode::I64Or_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotEq_Rrs { result, lhs, rhs } => {
                (OpCode::I64NotEq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotEq_Rri { result, lhs, rhs } => {
                (OpCode::I64NotEq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotEq_Rss { result, lhs, rhs } => {
                (OpCode::I64NotEq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotEq_Rsi { result, lhs, rhs } => {
                (OpCode::I64NotEq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotAnd_Rrs { result, lhs, rhs } => {
                (OpCode::I64NotAnd_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotAnd_Rri { result, lhs, rhs } => {
                (OpCode::I64NotAnd_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotAnd_Rss { result, lhs, rhs } => {
                (OpCode::I64NotAnd_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotAnd_Rsi { result, lhs, rhs } => {
                (OpCode::I64NotAnd_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotOr_Rrs { result, lhs, rhs } => {
                (OpCode::I64NotOr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotOr_Rri { result, lhs, rhs } => {
                (OpCode::I64NotOr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotOr_Rss { result, lhs, rhs } => {
                (OpCode::I64NotOr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64NotOr_Rsi { result, lhs, rhs } => {
                (OpCode::I64NotOr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Rrs { result, lhs, rhs } => {
                (OpCode::I64Lt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Rri { result, lhs, rhs } => {
                (OpCode::I64Lt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Rsr { result, lhs, rhs } => {
                (OpCode::I64Lt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Rss { result, lhs, rhs } => {
                (OpCode::I64Lt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Rsi { result, lhs, rhs } => {
                (OpCode::I64Lt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Rir { result, lhs, rhs } => {
                (OpCode::I64Lt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Lt_Ris { result, lhs, rhs } => {
                (OpCode::I64Lt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Rrs { result, lhs, rhs } => {
                (OpCode::I64Le_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Rri { result, lhs, rhs } => {
                (OpCode::I64Le_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Rsr { result, lhs, rhs } => {
                (OpCode::I64Le_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Rss { result, lhs, rhs } => {
                (OpCode::I64Le_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Rsi { result, lhs, rhs } => {
                (OpCode::I64Le_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Rir { result, lhs, rhs } => {
                (OpCode::I64Le_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Le_Ris { result, lhs, rhs } => {
                (OpCode::I64Le_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Rrs { result, lhs, rhs } => {
                (OpCode::U64Lt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Rri { result, lhs, rhs } => {
                (OpCode::U64Lt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Rsr { result, lhs, rhs } => {
                (OpCode::U64Lt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Rss { result, lhs, rhs } => {
                (OpCode::U64Lt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Rsi { result, lhs, rhs } => {
                (OpCode::U64Lt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Rir { result, lhs, rhs } => {
                (OpCode::U64Lt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U64Lt_Ris { result, lhs, rhs } => {
                (OpCode::U64Lt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Rrs { result, lhs, rhs } => {
                (OpCode::U64Le_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Rri { result, lhs, rhs } => {
                (OpCode::U64Le_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Rsr { result, lhs, rhs } => {
                (OpCode::U64Le_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Rss { result, lhs, rhs } => {
                (OpCode::U64Le_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Rsi { result, lhs, rhs } => {
                (OpCode::U64Le_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Rir { result, lhs, rhs } => {
                (OpCode::U64Le_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U64Le_Ris { result, lhs, rhs } => {
                (OpCode::U64Le_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Eq_Rrs { result, lhs, rhs } => {
                (OpCode::F32Eq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Eq_Rri { result, lhs, rhs } => {
                (OpCode::F32Eq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Eq_Rss { result, lhs, rhs } => {
                (OpCode::F32Eq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Eq_Rsi { result, lhs, rhs } => {
                (OpCode::F32Eq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Rrs { result, lhs, rhs } => {
                (OpCode::F32Lt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Rri { result, lhs, rhs } => {
                (OpCode::F32Lt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Rsr { result, lhs, rhs } => {
                (OpCode::F32Lt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Rss { result, lhs, rhs } => {
                (OpCode::F32Lt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Rsi { result, lhs, rhs } => {
                (OpCode::F32Lt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Rir { result, lhs, rhs } => {
                (OpCode::F32Lt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Lt_Ris { result, lhs, rhs } => {
                (OpCode::F32Lt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Rrs { result, lhs, rhs } => {
                (OpCode::F32Le_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Rri { result, lhs, rhs } => {
                (OpCode::F32Le_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Rsr { result, lhs, rhs } => {
                (OpCode::F32Le_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Rss { result, lhs, rhs } => {
                (OpCode::F32Le_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Rsi { result, lhs, rhs } => {
                (OpCode::F32Le_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Rir { result, lhs, rhs } => {
                (OpCode::F32Le_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Le_Ris { result, lhs, rhs } => {
                (OpCode::F32Le_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotEq_Rrs { result, lhs, rhs } => {
                (OpCode::F32NotEq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotEq_Rri { result, lhs, rhs } => {
                (OpCode::F32NotEq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotEq_Rss { result, lhs, rhs } => {
                (OpCode::F32NotEq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotEq_Rsi { result, lhs, rhs } => {
                (OpCode::F32NotEq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Rrs { result, lhs, rhs } => {
                (OpCode::F32NotLt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Rri { result, lhs, rhs } => {
                (OpCode::F32NotLt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Rsr { result, lhs, rhs } => {
                (OpCode::F32NotLt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Rss { result, lhs, rhs } => {
                (OpCode::F32NotLt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Rsi { result, lhs, rhs } => {
                (OpCode::F32NotLt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Rir { result, lhs, rhs } => {
                (OpCode::F32NotLt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLt_Ris { result, lhs, rhs } => {
                (OpCode::F32NotLt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Rrs { result, lhs, rhs } => {
                (OpCode::F32NotLe_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Rri { result, lhs, rhs } => {
                (OpCode::F32NotLe_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Rsr { result, lhs, rhs } => {
                (OpCode::F32NotLe_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Rss { result, lhs, rhs } => {
                (OpCode::F32NotLe_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Rsi { result, lhs, rhs } => {
                (OpCode::F32NotLe_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Rir { result, lhs, rhs } => {
                (OpCode::F32NotLe_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32NotLe_Ris { result, lhs, rhs } => {
                (OpCode::F32NotLe_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Eq_Rrs { result, lhs, rhs } => {
                (OpCode::F64Eq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Eq_Rri { result, lhs, rhs } => {
                (OpCode::F64Eq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Eq_Rss { result, lhs, rhs } => {
                (OpCode::F64Eq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Eq_Rsi { result, lhs, rhs } => {
                (OpCode::F64Eq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Rrs { result, lhs, rhs } => {
                (OpCode::F64Lt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Rri { result, lhs, rhs } => {
                (OpCode::F64Lt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Rsr { result, lhs, rhs } => {
                (OpCode::F64Lt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Rss { result, lhs, rhs } => {
                (OpCode::F64Lt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Rsi { result, lhs, rhs } => {
                (OpCode::F64Lt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Rir { result, lhs, rhs } => {
                (OpCode::F64Lt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Lt_Ris { result, lhs, rhs } => {
                (OpCode::F64Lt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Rrs { result, lhs, rhs } => {
                (OpCode::F64Le_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Rri { result, lhs, rhs } => {
                (OpCode::F64Le_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Rsr { result, lhs, rhs } => {
                (OpCode::F64Le_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Rss { result, lhs, rhs } => {
                (OpCode::F64Le_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Rsi { result, lhs, rhs } => {
                (OpCode::F64Le_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Rir { result, lhs, rhs } => {
                (OpCode::F64Le_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Le_Ris { result, lhs, rhs } => {
                (OpCode::F64Le_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotEq_Rrs { result, lhs, rhs } => {
                (OpCode::F64NotEq_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotEq_Rri { result, lhs, rhs } => {
                (OpCode::F64NotEq_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotEq_Rss { result, lhs, rhs } => {
                (OpCode::F64NotEq_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotEq_Rsi { result, lhs, rhs } => {
                (OpCode::F64NotEq_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Rrs { result, lhs, rhs } => {
                (OpCode::F64NotLt_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Rri { result, lhs, rhs } => {
                (OpCode::F64NotLt_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Rsr { result, lhs, rhs } => {
                (OpCode::F64NotLt_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Rss { result, lhs, rhs } => {
                (OpCode::F64NotLt_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Rsi { result, lhs, rhs } => {
                (OpCode::F64NotLt_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Rir { result, lhs, rhs } => {
                (OpCode::F64NotLt_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLt_Ris { result, lhs, rhs } => {
                (OpCode::F64NotLt_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Rrs { result, lhs, rhs } => {
                (OpCode::F64NotLe_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Rri { result, lhs, rhs } => {
                (OpCode::F64NotLe_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Rsr { result, lhs, rhs } => {
                (OpCode::F64NotLe_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Rss { result, lhs, rhs } => {
                (OpCode::F64NotLe_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Rsi { result, lhs, rhs } => {
                (OpCode::F64NotLe_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Rir { result, lhs, rhs } => {
                (OpCode::F64NotLe_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64NotLe_Ris { result, lhs, rhs } => {
                (OpCode::F64NotLe_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rrs { result, lhs, rhs } => {
                (OpCode::I32Add_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rri { result, lhs, rhs } => {
                (OpCode::I32Add_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rss { result, lhs, rhs } => {
                (OpCode::I32Add_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rsi { result, lhs, rhs } => {
                (OpCode::I32Add_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rs_rs { result, lhs, rhs } => {
                (OpCode::I32Add_Rs_rs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rs_ri { result, lhs, rhs } => {
                (OpCode::I32Add_Rs_ri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rs_ss { result, lhs, rhs } => {
                (OpCode::I32Add_Rs_ss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Add_Rs_si { result, lhs, rhs } => {
                (OpCode::I32Add_Rs_si, result, lhs, rhs).encode(encoder)
            }
            Self::I32Sub_Rrs { result, lhs, rhs } => {
                (OpCode::I32Sub_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Sub_Rsr { result, lhs, rhs } => {
                (OpCode::I32Sub_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Sub_Rss { result, lhs, rhs } => {
                (OpCode::I32Sub_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Sub_Rir { result, lhs, rhs } => {
                (OpCode::I32Sub_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Sub_Ris { result, lhs, rhs } => {
                (OpCode::I32Sub_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Mul_Rrs { result, lhs, rhs } => {
                (OpCode::I32Mul_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Mul_Rri { result, lhs, rhs } => {
                (OpCode::I32Mul_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Mul_Rss { result, lhs, rhs } => {
                (OpCode::I32Mul_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Mul_Rsi { result, lhs, rhs } => {
                (OpCode::I32Mul_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Rrs { result, lhs, rhs } => {
                (OpCode::I32Div_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Rri { result, lhs, rhs } => {
                (OpCode::I32Div_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Rsr { result, lhs, rhs } => {
                (OpCode::I32Div_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Rss { result, lhs, rhs } => {
                (OpCode::I32Div_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Rsi { result, lhs, rhs } => {
                (OpCode::I32Div_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Rir { result, lhs, rhs } => {
                (OpCode::I32Div_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Div_Ris { result, lhs, rhs } => {
                (OpCode::I32Div_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Rrs { result, lhs, rhs } => {
                (OpCode::U32Div_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Rri { result, lhs, rhs } => {
                (OpCode::U32Div_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Rsr { result, lhs, rhs } => {
                (OpCode::U32Div_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Rss { result, lhs, rhs } => {
                (OpCode::U32Div_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Rsi { result, lhs, rhs } => {
                (OpCode::U32Div_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Rir { result, lhs, rhs } => {
                (OpCode::U32Div_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U32Div_Ris { result, lhs, rhs } => {
                (OpCode::U32Div_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Rrs { result, lhs, rhs } => {
                (OpCode::I32Rem_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Rri { result, lhs, rhs } => {
                (OpCode::I32Rem_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Rsr { result, lhs, rhs } => {
                (OpCode::I32Rem_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Rss { result, lhs, rhs } => {
                (OpCode::I32Rem_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Rsi { result, lhs, rhs } => {
                (OpCode::I32Rem_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Rir { result, lhs, rhs } => {
                (OpCode::I32Rem_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rem_Ris { result, lhs, rhs } => {
                (OpCode::I32Rem_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Rrs { result, lhs, rhs } => {
                (OpCode::U32Rem_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Rri { result, lhs, rhs } => {
                (OpCode::U32Rem_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Rsr { result, lhs, rhs } => {
                (OpCode::U32Rem_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Rss { result, lhs, rhs } => {
                (OpCode::U32Rem_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Rsi { result, lhs, rhs } => {
                (OpCode::U32Rem_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Rir { result, lhs, rhs } => {
                (OpCode::U32Rem_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U32Rem_Ris { result, lhs, rhs } => {
                (OpCode::U32Rem_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitAnd_Rrs { result, lhs, rhs } => {
                (OpCode::I32BitAnd_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitAnd_Rri { result, lhs, rhs } => {
                (OpCode::I32BitAnd_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitAnd_Rss { result, lhs, rhs } => {
                (OpCode::I32BitAnd_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitAnd_Rsi { result, lhs, rhs } => {
                (OpCode::I32BitAnd_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitOr_Rrs { result, lhs, rhs } => {
                (OpCode::I32BitOr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitOr_Rri { result, lhs, rhs } => {
                (OpCode::I32BitOr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitOr_Rss { result, lhs, rhs } => {
                (OpCode::I32BitOr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitOr_Rsi { result, lhs, rhs } => {
                (OpCode::I32BitOr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitXor_Rrs { result, lhs, rhs } => {
                (OpCode::I32BitXor_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitXor_Rri { result, lhs, rhs } => {
                (OpCode::I32BitXor_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitXor_Rss { result, lhs, rhs } => {
                (OpCode::I32BitXor_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32BitXor_Rsi { result, lhs, rhs } => {
                (OpCode::I32BitXor_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Rrs { result, lhs, rhs } => {
                (OpCode::I32Shl_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Rri { result, lhs, rhs } => {
                (OpCode::I32Shl_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Rsr { result, lhs, rhs } => {
                (OpCode::I32Shl_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Rss { result, lhs, rhs } => {
                (OpCode::I32Shl_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Rsi { result, lhs, rhs } => {
                (OpCode::I32Shl_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Rir { result, lhs, rhs } => {
                (OpCode::I32Shl_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shl_Ris { result, lhs, rhs } => {
                (OpCode::I32Shl_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Rrs { result, lhs, rhs } => {
                (OpCode::I32Shr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Rri { result, lhs, rhs } => {
                (OpCode::I32Shr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Rsr { result, lhs, rhs } => {
                (OpCode::I32Shr_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Rss { result, lhs, rhs } => {
                (OpCode::I32Shr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Rsi { result, lhs, rhs } => {
                (OpCode::I32Shr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Rir { result, lhs, rhs } => {
                (OpCode::I32Shr_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Shr_Ris { result, lhs, rhs } => {
                (OpCode::I32Shr_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Rrs { result, lhs, rhs } => {
                (OpCode::U32Shr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Rri { result, lhs, rhs } => {
                (OpCode::U32Shr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Rsr { result, lhs, rhs } => {
                (OpCode::U32Shr_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Rss { result, lhs, rhs } => {
                (OpCode::U32Shr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Rsi { result, lhs, rhs } => {
                (OpCode::U32Shr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Rir { result, lhs, rhs } => {
                (OpCode::U32Shr_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U32Shr_Ris { result, lhs, rhs } => {
                (OpCode::U32Shr_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Rrs { result, lhs, rhs } => {
                (OpCode::I32Rotl_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Rri { result, lhs, rhs } => {
                (OpCode::I32Rotl_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Rsr { result, lhs, rhs } => {
                (OpCode::I32Rotl_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Rss { result, lhs, rhs } => {
                (OpCode::I32Rotl_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Rsi { result, lhs, rhs } => {
                (OpCode::I32Rotl_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Rir { result, lhs, rhs } => {
                (OpCode::I32Rotl_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotl_Ris { result, lhs, rhs } => {
                (OpCode::I32Rotl_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Rrs { result, lhs, rhs } => {
                (OpCode::I32Rotr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Rri { result, lhs, rhs } => {
                (OpCode::I32Rotr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Rsr { result, lhs, rhs } => {
                (OpCode::I32Rotr_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Rss { result, lhs, rhs } => {
                (OpCode::I32Rotr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Rsi { result, lhs, rhs } => {
                (OpCode::I32Rotr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Rir { result, lhs, rhs } => {
                (OpCode::I32Rotr_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I32Rotr_Ris { result, lhs, rhs } => {
                (OpCode::I32Rotr_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rrs { result, lhs, rhs } => {
                (OpCode::I64Add_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rri { result, lhs, rhs } => {
                (OpCode::I64Add_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rss { result, lhs, rhs } => {
                (OpCode::I64Add_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rsi { result, lhs, rhs } => {
                (OpCode::I64Add_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rs_rs { result, lhs, rhs } => {
                (OpCode::I64Add_Rs_rs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rs_ri { result, lhs, rhs } => {
                (OpCode::I64Add_Rs_ri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rs_ss { result, lhs, rhs } => {
                (OpCode::I64Add_Rs_ss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Add_Rs_si { result, lhs, rhs } => {
                (OpCode::I64Add_Rs_si, result, lhs, rhs).encode(encoder)
            }
            Self::I64Sub_Rrs { result, lhs, rhs } => {
                (OpCode::I64Sub_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Sub_Rsr { result, lhs, rhs } => {
                (OpCode::I64Sub_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Sub_Rss { result, lhs, rhs } => {
                (OpCode::I64Sub_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Sub_Rir { result, lhs, rhs } => {
                (OpCode::I64Sub_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Sub_Ris { result, lhs, rhs } => {
                (OpCode::I64Sub_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Mul_Rrs { result, lhs, rhs } => {
                (OpCode::I64Mul_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Mul_Rri { result, lhs, rhs } => {
                (OpCode::I64Mul_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Mul_Rss { result, lhs, rhs } => {
                (OpCode::I64Mul_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Mul_Rsi { result, lhs, rhs } => {
                (OpCode::I64Mul_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Rrs { result, lhs, rhs } => {
                (OpCode::I64Div_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Rri { result, lhs, rhs } => {
                (OpCode::I64Div_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Rsr { result, lhs, rhs } => {
                (OpCode::I64Div_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Rss { result, lhs, rhs } => {
                (OpCode::I64Div_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Rsi { result, lhs, rhs } => {
                (OpCode::I64Div_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Rir { result, lhs, rhs } => {
                (OpCode::I64Div_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Div_Ris { result, lhs, rhs } => {
                (OpCode::I64Div_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Rrs { result, lhs, rhs } => {
                (OpCode::U64Div_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Rri { result, lhs, rhs } => {
                (OpCode::U64Div_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Rsr { result, lhs, rhs } => {
                (OpCode::U64Div_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Rss { result, lhs, rhs } => {
                (OpCode::U64Div_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Rsi { result, lhs, rhs } => {
                (OpCode::U64Div_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Rir { result, lhs, rhs } => {
                (OpCode::U64Div_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U64Div_Ris { result, lhs, rhs } => {
                (OpCode::U64Div_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Rrs { result, lhs, rhs } => {
                (OpCode::I64Rem_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Rri { result, lhs, rhs } => {
                (OpCode::I64Rem_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Rsr { result, lhs, rhs } => {
                (OpCode::I64Rem_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Rss { result, lhs, rhs } => {
                (OpCode::I64Rem_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Rsi { result, lhs, rhs } => {
                (OpCode::I64Rem_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Rir { result, lhs, rhs } => {
                (OpCode::I64Rem_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rem_Ris { result, lhs, rhs } => {
                (OpCode::I64Rem_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Rrs { result, lhs, rhs } => {
                (OpCode::U64Rem_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Rri { result, lhs, rhs } => {
                (OpCode::U64Rem_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Rsr { result, lhs, rhs } => {
                (OpCode::U64Rem_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Rss { result, lhs, rhs } => {
                (OpCode::U64Rem_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Rsi { result, lhs, rhs } => {
                (OpCode::U64Rem_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Rir { result, lhs, rhs } => {
                (OpCode::U64Rem_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U64Rem_Ris { result, lhs, rhs } => {
                (OpCode::U64Rem_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitAnd_Rrs { result, lhs, rhs } => {
                (OpCode::I64BitAnd_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitAnd_Rri { result, lhs, rhs } => {
                (OpCode::I64BitAnd_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitAnd_Rss { result, lhs, rhs } => {
                (OpCode::I64BitAnd_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitAnd_Rsi { result, lhs, rhs } => {
                (OpCode::I64BitAnd_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitOr_Rrs { result, lhs, rhs } => {
                (OpCode::I64BitOr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitOr_Rri { result, lhs, rhs } => {
                (OpCode::I64BitOr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitOr_Rss { result, lhs, rhs } => {
                (OpCode::I64BitOr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitOr_Rsi { result, lhs, rhs } => {
                (OpCode::I64BitOr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitXor_Rrs { result, lhs, rhs } => {
                (OpCode::I64BitXor_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitXor_Rri { result, lhs, rhs } => {
                (OpCode::I64BitXor_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitXor_Rss { result, lhs, rhs } => {
                (OpCode::I64BitXor_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64BitXor_Rsi { result, lhs, rhs } => {
                (OpCode::I64BitXor_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Rrs { result, lhs, rhs } => {
                (OpCode::I64Shl_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Rri { result, lhs, rhs } => {
                (OpCode::I64Shl_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Rsr { result, lhs, rhs } => {
                (OpCode::I64Shl_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Rss { result, lhs, rhs } => {
                (OpCode::I64Shl_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Rsi { result, lhs, rhs } => {
                (OpCode::I64Shl_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Rir { result, lhs, rhs } => {
                (OpCode::I64Shl_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shl_Ris { result, lhs, rhs } => {
                (OpCode::I64Shl_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Rrs { result, lhs, rhs } => {
                (OpCode::I64Shr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Rri { result, lhs, rhs } => {
                (OpCode::I64Shr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Rsr { result, lhs, rhs } => {
                (OpCode::I64Shr_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Rss { result, lhs, rhs } => {
                (OpCode::I64Shr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Rsi { result, lhs, rhs } => {
                (OpCode::I64Shr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Rir { result, lhs, rhs } => {
                (OpCode::I64Shr_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Shr_Ris { result, lhs, rhs } => {
                (OpCode::I64Shr_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Rrs { result, lhs, rhs } => {
                (OpCode::U64Shr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Rri { result, lhs, rhs } => {
                (OpCode::U64Shr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Rsr { result, lhs, rhs } => {
                (OpCode::U64Shr_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Rss { result, lhs, rhs } => {
                (OpCode::U64Shr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Rsi { result, lhs, rhs } => {
                (OpCode::U64Shr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Rir { result, lhs, rhs } => {
                (OpCode::U64Shr_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::U64Shr_Ris { result, lhs, rhs } => {
                (OpCode::U64Shr_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Rrs { result, lhs, rhs } => {
                (OpCode::I64Rotl_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Rri { result, lhs, rhs } => {
                (OpCode::I64Rotl_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Rsr { result, lhs, rhs } => {
                (OpCode::I64Rotl_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Rss { result, lhs, rhs } => {
                (OpCode::I64Rotl_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Rsi { result, lhs, rhs } => {
                (OpCode::I64Rotl_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Rir { result, lhs, rhs } => {
                (OpCode::I64Rotl_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotl_Ris { result, lhs, rhs } => {
                (OpCode::I64Rotl_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Rrs { result, lhs, rhs } => {
                (OpCode::I64Rotr_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Rri { result, lhs, rhs } => {
                (OpCode::I64Rotr_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Rsr { result, lhs, rhs } => {
                (OpCode::I64Rotr_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Rss { result, lhs, rhs } => {
                (OpCode::I64Rotr_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Rsi { result, lhs, rhs } => {
                (OpCode::I64Rotr_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Rir { result, lhs, rhs } => {
                (OpCode::I64Rotr_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::I64Rotr_Ris { result, lhs, rhs } => {
                (OpCode::I64Rotr_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Rrs { result, lhs, rhs } => {
                (OpCode::F32Add_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Rri { result, lhs, rhs } => {
                (OpCode::F32Add_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Rsr { result, lhs, rhs } => {
                (OpCode::F32Add_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Rss { result, lhs, rhs } => {
                (OpCode::F32Add_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Rsi { result, lhs, rhs } => {
                (OpCode::F32Add_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Rir { result, lhs, rhs } => {
                (OpCode::F32Add_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Add_Ris { result, lhs, rhs } => {
                (OpCode::F32Add_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Rrs { result, lhs, rhs } => {
                (OpCode::F32Sub_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Rri { result, lhs, rhs } => {
                (OpCode::F32Sub_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Rsr { result, lhs, rhs } => {
                (OpCode::F32Sub_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Rss { result, lhs, rhs } => {
                (OpCode::F32Sub_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Rsi { result, lhs, rhs } => {
                (OpCode::F32Sub_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Rir { result, lhs, rhs } => {
                (OpCode::F32Sub_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Sub_Ris { result, lhs, rhs } => {
                (OpCode::F32Sub_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rrs { result, lhs, rhs } => {
                (OpCode::F32Mul_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rri { result, lhs, rhs } => {
                (OpCode::F32Mul_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rsr { result, lhs, rhs } => {
                (OpCode::F32Mul_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rss { result, lhs, rhs } => {
                (OpCode::F32Mul_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rsi { result, lhs, rhs } => {
                (OpCode::F32Mul_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rir { result, lhs, rhs } => {
                (OpCode::F32Mul_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Ris { result, lhs, rhs } => {
                (OpCode::F32Mul_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Rrs { result, lhs, rhs } => {
                (OpCode::F32Div_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Rri { result, lhs, rhs } => {
                (OpCode::F32Div_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Rsr { result, lhs, rhs } => {
                (OpCode::F32Div_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Rss { result, lhs, rhs } => {
                (OpCode::F32Div_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Rsi { result, lhs, rhs } => {
                (OpCode::F32Div_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Rir { result, lhs, rhs } => {
                (OpCode::F32Div_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Div_Ris { result, lhs, rhs } => {
                (OpCode::F32Div_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Rrs { result, lhs, rhs } => {
                (OpCode::F32Min_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Rri { result, lhs, rhs } => {
                (OpCode::F32Min_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Rsr { result, lhs, rhs } => {
                (OpCode::F32Min_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Rss { result, lhs, rhs } => {
                (OpCode::F32Min_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Rsi { result, lhs, rhs } => {
                (OpCode::F32Min_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Rir { result, lhs, rhs } => {
                (OpCode::F32Min_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Min_Ris { result, lhs, rhs } => {
                (OpCode::F32Min_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Rrs { result, lhs, rhs } => {
                (OpCode::F32Max_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Rri { result, lhs, rhs } => {
                (OpCode::F32Max_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Rsr { result, lhs, rhs } => {
                (OpCode::F32Max_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Rss { result, lhs, rhs } => {
                (OpCode::F32Max_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Rsi { result, lhs, rhs } => {
                (OpCode::F32Max_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Rir { result, lhs, rhs } => {
                (OpCode::F32Max_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Max_Ris { result, lhs, rhs } => {
                (OpCode::F32Max_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Rrs { result, lhs, rhs } => {
                (OpCode::F64Add_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Rri { result, lhs, rhs } => {
                (OpCode::F64Add_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Rsr { result, lhs, rhs } => {
                (OpCode::F64Add_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Rss { result, lhs, rhs } => {
                (OpCode::F64Add_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Rsi { result, lhs, rhs } => {
                (OpCode::F64Add_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Rir { result, lhs, rhs } => {
                (OpCode::F64Add_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Add_Ris { result, lhs, rhs } => {
                (OpCode::F64Add_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Rrs { result, lhs, rhs } => {
                (OpCode::F64Sub_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Rri { result, lhs, rhs } => {
                (OpCode::F64Sub_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Rsr { result, lhs, rhs } => {
                (OpCode::F64Sub_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Rss { result, lhs, rhs } => {
                (OpCode::F64Sub_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Rsi { result, lhs, rhs } => {
                (OpCode::F64Sub_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Rir { result, lhs, rhs } => {
                (OpCode::F64Sub_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Sub_Ris { result, lhs, rhs } => {
                (OpCode::F64Sub_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rrs { result, lhs, rhs } => {
                (OpCode::F64Mul_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rri { result, lhs, rhs } => {
                (OpCode::F64Mul_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rsr { result, lhs, rhs } => {
                (OpCode::F64Mul_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rss { result, lhs, rhs } => {
                (OpCode::F64Mul_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rsi { result, lhs, rhs } => {
                (OpCode::F64Mul_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rir { result, lhs, rhs } => {
                (OpCode::F64Mul_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Ris { result, lhs, rhs } => {
                (OpCode::F64Mul_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Rrs { result, lhs, rhs } => {
                (OpCode::F64Div_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Rri { result, lhs, rhs } => {
                (OpCode::F64Div_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Rsr { result, lhs, rhs } => {
                (OpCode::F64Div_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Rss { result, lhs, rhs } => {
                (OpCode::F64Div_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Rsi { result, lhs, rhs } => {
                (OpCode::F64Div_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Rir { result, lhs, rhs } => {
                (OpCode::F64Div_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Div_Ris { result, lhs, rhs } => {
                (OpCode::F64Div_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Rrs { result, lhs, rhs } => {
                (OpCode::F64Min_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Rri { result, lhs, rhs } => {
                (OpCode::F64Min_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Rsr { result, lhs, rhs } => {
                (OpCode::F64Min_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Rss { result, lhs, rhs } => {
                (OpCode::F64Min_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Rsi { result, lhs, rhs } => {
                (OpCode::F64Min_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Rir { result, lhs, rhs } => {
                (OpCode::F64Min_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Min_Ris { result, lhs, rhs } => {
                (OpCode::F64Min_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Rrs { result, lhs, rhs } => {
                (OpCode::F64Max_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Rri { result, lhs, rhs } => {
                (OpCode::F64Max_Rri, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Rsr { result, lhs, rhs } => {
                (OpCode::F64Max_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Rss { result, lhs, rhs } => {
                (OpCode::F64Max_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Rsi { result, lhs, rhs } => {
                (OpCode::F64Max_Rsi, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Rir { result, lhs, rhs } => {
                (OpCode::F64Max_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Max_Ris { result, lhs, rhs } => {
                (OpCode::F64Max_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F32Copysign_Rrs { result, lhs, rhs } => {
                (OpCode::F32Copysign_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F32Copysign_Rsr { result, lhs, rhs } => {
                (OpCode::F32Copysign_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Copysign_Rss { result, lhs, rhs } => {
                (OpCode::F32Copysign_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F32Copysign_Rir { result, lhs, rhs } => {
                (OpCode::F32Copysign_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F32Copysign_Ris { result, lhs, rhs } => {
                (OpCode::F32Copysign_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::F64Copysign_Rrs { result, lhs, rhs } => {
                (OpCode::F64Copysign_Rrs, result, lhs, rhs).encode(encoder)
            }
            Self::F64Copysign_Rsr { result, lhs, rhs } => {
                (OpCode::F64Copysign_Rsr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Copysign_Rss { result, lhs, rhs } => {
                (OpCode::F64Copysign_Rss, result, lhs, rhs).encode(encoder)
            }
            Self::F64Copysign_Rir { result, lhs, rhs } => {
                (OpCode::F64Copysign_Rir, result, lhs, rhs).encode(encoder)
            }
            Self::F64Copysign_Ris { result, lhs, rhs } => {
                (OpCode::F64Copysign_Ris, result, lhs, rhs).encode(encoder)
            }
            Self::I32Mul_Rrr { result, lhs, rhs } => {
                (OpCode::I32Mul_Rrr, result, lhs, rhs).encode(encoder)
            }
            Self::I64Mul_Rrr { result, lhs, rhs } => {
                (OpCode::I64Mul_Rrr, result, lhs, rhs).encode(encoder)
            }
            Self::F32Mul_Rrr { result, lhs, rhs } => {
                (OpCode::F32Mul_Rrr, result, lhs, rhs).encode(encoder)
            }
            Self::F64Mul_Rrr { result, lhs, rhs } => {
                (OpCode::F64Mul_Rrr, result, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Eq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32Eq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Eq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32Eq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Eq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32Eq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Eq_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32Eq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotEq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32NotEq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotEq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32NotEq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotEq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32NotEq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotEq_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32NotEq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32And_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32And_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32And_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32And_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32And_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32And_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32And_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32And_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotAnd_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32NotAnd_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotAnd_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32NotAnd_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotAnd_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32NotAnd_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotAnd_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32NotAnd_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Or_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32Or_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Or_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32Or_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Or_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32Or_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Or_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32Or_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotOr_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32NotOr_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotOr_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32NotOr_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotOr_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32NotOr_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32NotOr_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32NotOr_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Lt_Is { offset, lhs, rhs } => {
                (OpCode::BranchI32Lt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Sr { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Si { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Ir { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI32Le_Is { offset, lhs, rhs } => {
                (OpCode::BranchI32Le_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Si { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Lt_Is { offset, lhs, rhs } => {
                (OpCode::BranchU32Lt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Rs { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Ri { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Sr { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Ss { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Si { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Ir { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU32Le_Is { offset, lhs, rhs } => {
                (OpCode::BranchU32Le_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Eq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64Eq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Eq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64Eq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Eq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64Eq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Eq_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64Eq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotEq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64NotEq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotEq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64NotEq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotEq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64NotEq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotEq_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64NotEq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64And_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64And_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64And_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64And_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64And_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64And_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64And_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64And_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotAnd_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64NotAnd_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotAnd_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64NotAnd_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotAnd_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64NotAnd_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotAnd_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64NotAnd_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Or_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64Or_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Or_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64Or_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Or_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64Or_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Or_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64Or_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotOr_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64NotOr_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotOr_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64NotOr_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotOr_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64NotOr_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64NotOr_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64NotOr_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Lt_Is { offset, lhs, rhs } => {
                (OpCode::BranchI64Lt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Rs { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Ri { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Sr { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Ss { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Si { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Ir { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchI64Le_Is { offset, lhs, rhs } => {
                (OpCode::BranchI64Le_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Si { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Lt_Is { offset, lhs, rhs } => {
                (OpCode::BranchU64Lt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Rs { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Ri { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Sr { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Ss { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Si { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Ir { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchU64Le_Is { offset, lhs, rhs } => {
                (OpCode::BranchU64Le_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Eq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF32Eq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Eq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF32Eq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Eq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF32Eq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Eq_Si { offset, lhs, rhs } => {
                (OpCode::BranchF32Eq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotEq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF32NotEq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotEq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF32NotEq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotEq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF32NotEq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotEq_Si { offset, lhs, rhs } => {
                (OpCode::BranchF32NotEq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Si { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Lt_Is { offset, lhs, rhs } => {
                (OpCode::BranchF32Lt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Si { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLt_Is { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Si { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32Le_Is { offset, lhs, rhs } => {
                (OpCode::BranchF32Le_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Si { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF32NotLe_Is { offset, lhs, rhs } => {
                (OpCode::BranchF32NotLe_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Eq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF64Eq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Eq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF64Eq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Eq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF64Eq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Eq_Si { offset, lhs, rhs } => {
                (OpCode::BranchF64Eq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotEq_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF64NotEq_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotEq_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF64NotEq_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotEq_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF64NotEq_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotEq_Si { offset, lhs, rhs } => {
                (OpCode::BranchF64NotEq_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Si { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Lt_Is { offset, lhs, rhs } => {
                (OpCode::BranchF64Lt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Si { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLt_Is { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLt_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Si { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64Le_Is { offset, lhs, rhs } => {
                (OpCode::BranchF64Le_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Rs { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Rs, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Ri { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Ri, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Sr { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Sr, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Ss { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Ss, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Si { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Si, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Ir { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Ir, offset, lhs, rhs).encode(encoder)
            }
            Self::BranchF64NotLe_Is { offset, lhs, rhs } => {
                (OpCode::BranchF64NotLe_Is, offset, lhs, rhs).encode(encoder)
            }
            Self::U32Select_Rrri { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rrri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rrsi { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rrsi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rrir { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rrir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rris { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rris, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rrii { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rrii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rsri { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rsri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rssi { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rssi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rsir { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rsir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rsis { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rsis, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Select_Rsii { result, condition, true_val, false_val } => {
                (OpCode::U32Select_Rsii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrrs { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrrs, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrri { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrsr { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrsr, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrss { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrss, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrsi { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrsi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrir { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rris { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rris, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rrii { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rrii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rsrs { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rsrs, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rsri { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rsri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rssr { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rssr, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rsss { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rsss, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rssi { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rssi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rsir { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rsir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rsis { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rsis, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U64Select_Rsii { result, condition, true_val, false_val } => {
                (OpCode::U64Select_Rsii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrrs { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrrs, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrri { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrsr { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrsr, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrss { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrss, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrsi { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrsi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrir { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rris { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rris, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rrii { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rrii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rsrs { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rsrs, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rsri { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rsri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rssr { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rssr, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rsss { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rsss, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rssi { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rssi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rsir { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rsir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rsis { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rsis, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F32Select_Rsii { result, condition, true_val, false_val } => {
                (OpCode::F32Select_Rsii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrrs { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrrs, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrri { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrsr { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrsr, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrss { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrss, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrsi { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrsi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrir { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rris { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rris, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rrii { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rrii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rsrs { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rsrs, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rsri { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rsri, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rssr { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rssr, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rsss { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rsss, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rssi { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rssi, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rsir { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rsir, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rsis { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rsis, result, condition, true_val, false_val).encode(encoder)
            }
            Self::F64Select_Rsii { result, condition, true_val, false_val } => {
                (OpCode::F64Select_Rsii, result, condition, true_val, false_val).encode(encoder)
            }
            Self::U32Load_Rr { result, ptr, offset, memory } => {
                (OpCode::U32Load_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U32LoadMem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U32LoadMem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadMem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U32LoadMem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U32Load_Rs { result, ptr, offset, memory } => {
                (OpCode::U32Load_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U32LoadMem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U32LoadMem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadMem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U32LoadMem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U32Load_Ri { result, ptr, memory } => {
                (OpCode::U32Load_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U64Load_Rr { result, ptr, offset, memory } => {
                (OpCode::U64Load_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadMem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U64LoadMem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadMem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U64LoadMem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U64Load_Rs { result, ptr, offset, memory } => {
                (OpCode::U64Load_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadMem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U64LoadMem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadMem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U64LoadMem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U64Load_Ri { result, ptr, memory } => {
                (OpCode::U64Load_Ri, result, ptr, memory).encode(encoder)
            }
            Self::F32Load_Rr { result, ptr, offset, memory } => {
                (OpCode::F32Load_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::F32LoadMem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::F32LoadMem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::F32LoadMem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::F32LoadMem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::F32Load_Rs { result, ptr, offset, memory } => {
                (OpCode::F32Load_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::F32LoadMem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::F32LoadMem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::F32LoadMem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::F32LoadMem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::F32Load_Ri { result, ptr, memory } => {
                (OpCode::F32Load_Ri, result, ptr, memory).encode(encoder)
            }
            Self::F64Load_Rr { result, ptr, offset, memory } => {
                (OpCode::F64Load_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::F64LoadMem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::F64LoadMem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::F64LoadMem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::F64LoadMem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::F64Load_Rs { result, ptr, offset, memory } => {
                (OpCode::F64Load_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::F64LoadMem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::F64LoadMem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::F64LoadMem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::F64LoadMem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::F64Load_Ri { result, ptr, memory } => {
                (OpCode::F64Load_Ri, result, ptr, memory).encode(encoder)
            }
            Self::I32LoadExtend8_Rr { result, ptr, offset, memory } => {
                (OpCode::I32LoadExtend8_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::I32LoadExtend8Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::I32LoadExtend8Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend8Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::I32LoadExtend8Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend8_Rs { result, ptr, offset, memory } => {
                (OpCode::I32LoadExtend8_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::I32LoadExtend8Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::I32LoadExtend8Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::I32LoadExtend8Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend8_Ri { result, ptr, memory } => {
                (OpCode::I32LoadExtend8_Ri, result, ptr, memory).encode(encoder)
            }
            Self::I32LoadExtend16_Rr { result, ptr, offset, memory } => {
                (OpCode::I32LoadExtend16_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::I32LoadExtend16Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::I32LoadExtend16Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend16Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::I32LoadExtend16Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend16_Rs { result, ptr, offset, memory } => {
                (OpCode::I32LoadExtend16_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::I32LoadExtend16Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::I32LoadExtend16Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::I32LoadExtend16Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::I32LoadExtend16_Ri { result, ptr, memory } => {
                (OpCode::I32LoadExtend16_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U32LoadExtend8_Rr { result, ptr, offset, memory } => {
                (OpCode::U32LoadExtend8_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U32LoadExtend8Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U32LoadExtend8Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend8Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U32LoadExtend8Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend8_Rs { result, ptr, offset, memory } => {
                (OpCode::U32LoadExtend8_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U32LoadExtend8Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U32LoadExtend8Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U32LoadExtend8Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend8_Ri { result, ptr, memory } => {
                (OpCode::U32LoadExtend8_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U32LoadExtend16_Rr { result, ptr, offset, memory } => {
                (OpCode::U32LoadExtend16_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U32LoadExtend16Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U32LoadExtend16Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend16Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U32LoadExtend16Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend16_Rs { result, ptr, offset, memory } => {
                (OpCode::U32LoadExtend16_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U32LoadExtend16Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U32LoadExtend16Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U32LoadExtend16Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U32LoadExtend16_Ri { result, ptr, memory } => {
                (OpCode::U32LoadExtend16_Ri, result, ptr, memory).encode(encoder)
            }
            Self::I64LoadExtend8_Rr { result, ptr, offset, memory } => {
                (OpCode::I64LoadExtend8_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::I64LoadExtend8Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::I64LoadExtend8Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend8Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::I64LoadExtend8Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend8_Rs { result, ptr, offset, memory } => {
                (OpCode::I64LoadExtend8_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::I64LoadExtend8Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::I64LoadExtend8Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::I64LoadExtend8Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend8_Ri { result, ptr, memory } => {
                (OpCode::I64LoadExtend8_Ri, result, ptr, memory).encode(encoder)
            }
            Self::I64LoadExtend16_Rr { result, ptr, offset, memory } => {
                (OpCode::I64LoadExtend16_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::I64LoadExtend16Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::I64LoadExtend16Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend16Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::I64LoadExtend16Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend16_Rs { result, ptr, offset, memory } => {
                (OpCode::I64LoadExtend16_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::I64LoadExtend16Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::I64LoadExtend16Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::I64LoadExtend16Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend16_Ri { result, ptr, memory } => {
                (OpCode::I64LoadExtend16_Ri, result, ptr, memory).encode(encoder)
            }
            Self::I64LoadExtend32_Rr { result, ptr, offset, memory } => {
                (OpCode::I64LoadExtend32_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::I64LoadExtend32Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::I64LoadExtend32Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend32Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::I64LoadExtend32Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend32_Rs { result, ptr, offset, memory } => {
                (OpCode::I64LoadExtend32_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::I64LoadExtend32Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::I64LoadExtend32Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend32Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::I64LoadExtend32Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::I64LoadExtend32_Ri { result, ptr, memory } => {
                (OpCode::I64LoadExtend32_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U64LoadExtend8_Rr { result, ptr, offset, memory } => {
                (OpCode::U64LoadExtend8_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadExtend8Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U64LoadExtend8Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend8Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U64LoadExtend8Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend8_Rs { result, ptr, offset, memory } => {
                (OpCode::U64LoadExtend8_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadExtend8Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U64LoadExtend8Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend8Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U64LoadExtend8Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend8_Ri { result, ptr, memory } => {
                (OpCode::U64LoadExtend8_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U64LoadExtend16_Rr { result, ptr, offset, memory } => {
                (OpCode::U64LoadExtend16_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadExtend16Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U64LoadExtend16Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend16Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U64LoadExtend16Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend16_Rs { result, ptr, offset, memory } => {
                (OpCode::U64LoadExtend16_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadExtend16Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U64LoadExtend16Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend16Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U64LoadExtend16Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend16_Ri { result, ptr, memory } => {
                (OpCode::U64LoadExtend16_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U64LoadExtend32_Rr { result, ptr, offset, memory } => {
                (OpCode::U64LoadExtend32_Rr, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadExtend32Mem0Offset16_Rr { result, ptr, offset } => {
                (OpCode::U64LoadExtend32Mem0Offset16_Rr, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend32Mem0Offset16_Rs_r { result, ptr, offset } => {
                (OpCode::U64LoadExtend32Mem0Offset16_Rs_r, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend32_Rs { result, ptr, offset, memory } => {
                (OpCode::U64LoadExtend32_Rs, result, ptr, offset, memory).encode(encoder)
            }
            Self::U64LoadExtend32Mem0Offset16_Rs { result, ptr, offset } => {
                (OpCode::U64LoadExtend32Mem0Offset16_Rs, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend32Mem0Offset16_Rs_s { result, ptr, offset } => {
                (OpCode::U64LoadExtend32Mem0Offset16_Rs_s, result, ptr, offset).encode(encoder)
            }
            Self::U64LoadExtend32_Ri { result, ptr, memory } => {
                (OpCode::U64LoadExtend32_Ri, result, ptr, memory).encode(encoder)
            }
            Self::U32Store_Rs { ptr, offset, value, memory } => {
                (OpCode::U32Store_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::U32StoreMem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::U32StoreMem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::U32Store_Ri { ptr, offset, value, memory } => {
                (OpCode::U32Store_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::U32StoreMem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::U32StoreMem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::U32Store_Sr { ptr, offset, value, memory } => {
                (OpCode::U32Store_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::U32StoreMem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::U32StoreMem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::U32Store_Ss { ptr, offset, value, memory } => {
                (OpCode::U32Store_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::U32StoreMem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::U32StoreMem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::U32Store_Si { ptr, offset, value, memory } => {
                (OpCode::U32Store_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::U32StoreMem0Offset16_Si { ptr, offset, value } => {
                (OpCode::U32StoreMem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::U32Store_Ir { ptr, value, memory } => {
                (OpCode::U32Store_Ir, ptr, value, memory).encode(encoder)
            }
            Self::U32Store_Is { ptr, value, memory } => {
                (OpCode::U32Store_Is, ptr, value, memory).encode(encoder)
            }
            Self::U32Store_Ii { ptr, value, memory } => {
                (OpCode::U32Store_Ii, ptr, value, memory).encode(encoder)
            }
            Self::U64Store_Rs { ptr, offset, value, memory } => {
                (OpCode::U64Store_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::U64StoreMem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::U64StoreMem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::U64Store_Ri { ptr, offset, value, memory } => {
                (OpCode::U64Store_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::U64StoreMem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::U64StoreMem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::U64Store_Sr { ptr, offset, value, memory } => {
                (OpCode::U64Store_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::U64StoreMem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::U64StoreMem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::U64Store_Ss { ptr, offset, value, memory } => {
                (OpCode::U64Store_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::U64StoreMem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::U64StoreMem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::U64Store_Si { ptr, offset, value, memory } => {
                (OpCode::U64Store_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::U64StoreMem0Offset16_Si { ptr, offset, value } => {
                (OpCode::U64StoreMem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::U64Store_Ir { ptr, value, memory } => {
                (OpCode::U64Store_Ir, ptr, value, memory).encode(encoder)
            }
            Self::U64Store_Is { ptr, value, memory } => {
                (OpCode::U64Store_Is, ptr, value, memory).encode(encoder)
            }
            Self::U64Store_Ii { ptr, value, memory } => {
                (OpCode::U64Store_Ii, ptr, value, memory).encode(encoder)
            }
            Self::F32Store_Rr { ptr, offset, value, memory } => {
                (OpCode::F32Store_Rr, ptr, offset, value, memory).encode(encoder)
            }
            Self::F32StoreMem0Offset16_Rr { ptr, offset, value } => {
                (OpCode::F32StoreMem0Offset16_Rr, ptr, offset, value).encode(encoder)
            }
            Self::F32Store_Sr { ptr, offset, value, memory } => {
                (OpCode::F32Store_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::F32StoreMem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::F32StoreMem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::F32Store_Ir { ptr, value, memory } => {
                (OpCode::F32Store_Ir, ptr, value, memory).encode(encoder)
            }
            Self::F64Store_Rr { ptr, offset, value, memory } => {
                (OpCode::F64Store_Rr, ptr, offset, value, memory).encode(encoder)
            }
            Self::F64StoreMem0Offset16_Rr { ptr, offset, value } => {
                (OpCode::F64StoreMem0Offset16_Rr, ptr, offset, value).encode(encoder)
            }
            Self::F64Store_Sr { ptr, offset, value, memory } => {
                (OpCode::F64Store_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::F64StoreMem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::F64StoreMem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::F64Store_Ir { ptr, value, memory } => {
                (OpCode::F64Store_Ir, ptr, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8_Rs { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap8_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8Mem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::I32StoreWrap8Mem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap8_Ri { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap8_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8Mem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::I32StoreWrap8Mem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap8_Sr { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap8_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8Mem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::I32StoreWrap8Mem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap8_Ss { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap8_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8Mem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::I32StoreWrap8Mem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap8_Si { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap8_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8Mem0Offset16_Si { ptr, offset, value } => {
                (OpCode::I32StoreWrap8Mem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap8_Ir { ptr, value, memory } => {
                (OpCode::I32StoreWrap8_Ir, ptr, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8_Is { ptr, value, memory } => {
                (OpCode::I32StoreWrap8_Is, ptr, value, memory).encode(encoder)
            }
            Self::I32StoreWrap8_Ii { ptr, value, memory } => {
                (OpCode::I32StoreWrap8_Ii, ptr, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16_Rs { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap16_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16Mem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::I32StoreWrap16Mem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap16_Ri { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap16_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16Mem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::I32StoreWrap16Mem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap16_Sr { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap16_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16Mem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::I32StoreWrap16Mem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap16_Ss { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap16_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16Mem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::I32StoreWrap16Mem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap16_Si { ptr, offset, value, memory } => {
                (OpCode::I32StoreWrap16_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16Mem0Offset16_Si { ptr, offset, value } => {
                (OpCode::I32StoreWrap16Mem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::I32StoreWrap16_Ir { ptr, value, memory } => {
                (OpCode::I32StoreWrap16_Ir, ptr, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16_Is { ptr, value, memory } => {
                (OpCode::I32StoreWrap16_Is, ptr, value, memory).encode(encoder)
            }
            Self::I32StoreWrap16_Ii { ptr, value, memory } => {
                (OpCode::I32StoreWrap16_Ii, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8_Rs { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap8_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8Mem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::I64StoreWrap8Mem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap8_Ri { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap8_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8Mem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::I64StoreWrap8Mem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap8_Sr { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap8_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8Mem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::I64StoreWrap8Mem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap8_Ss { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap8_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8Mem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::I64StoreWrap8Mem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap8_Si { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap8_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8Mem0Offset16_Si { ptr, offset, value } => {
                (OpCode::I64StoreWrap8Mem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap8_Ir { ptr, value, memory } => {
                (OpCode::I64StoreWrap8_Ir, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8_Is { ptr, value, memory } => {
                (OpCode::I64StoreWrap8_Is, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap8_Ii { ptr, value, memory } => {
                (OpCode::I64StoreWrap8_Ii, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16_Rs { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap16_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16Mem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::I64StoreWrap16Mem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap16_Ri { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap16_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16Mem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::I64StoreWrap16Mem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap16_Sr { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap16_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16Mem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::I64StoreWrap16Mem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap16_Ss { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap16_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16Mem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::I64StoreWrap16Mem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap16_Si { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap16_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16Mem0Offset16_Si { ptr, offset, value } => {
                (OpCode::I64StoreWrap16Mem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap16_Ir { ptr, value, memory } => {
                (OpCode::I64StoreWrap16_Ir, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16_Is { ptr, value, memory } => {
                (OpCode::I64StoreWrap16_Is, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap16_Ii { ptr, value, memory } => {
                (OpCode::I64StoreWrap16_Ii, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32_Rs { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap32_Rs, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32Mem0Offset16_Rs { ptr, offset, value } => {
                (OpCode::I64StoreWrap32Mem0Offset16_Rs, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap32_Ri { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap32_Ri, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32Mem0Offset16_Ri { ptr, offset, value } => {
                (OpCode::I64StoreWrap32Mem0Offset16_Ri, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap32_Sr { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap32_Sr, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32Mem0Offset16_Sr { ptr, offset, value } => {
                (OpCode::I64StoreWrap32Mem0Offset16_Sr, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap32_Ss { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap32_Ss, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32Mem0Offset16_Ss { ptr, offset, value } => {
                (OpCode::I64StoreWrap32Mem0Offset16_Ss, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap32_Si { ptr, offset, value, memory } => {
                (OpCode::I64StoreWrap32_Si, ptr, offset, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32Mem0Offset16_Si { ptr, offset, value } => {
                (OpCode::I64StoreWrap32Mem0Offset16_Si, ptr, offset, value).encode(encoder)
            }
            Self::I64StoreWrap32_Ir { ptr, value, memory } => {
                (OpCode::I64StoreWrap32_Ir, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32_Is { ptr, value, memory } => {
                (OpCode::I64StoreWrap32_Is, ptr, value, memory).encode(encoder)
            }
            Self::I64StoreWrap32_Ii { ptr, value, memory } => {
                (OpCode::I64StoreWrap32_Ii, ptr, value, memory).encode(encoder)
            }
            Self::Return {  } => {
                (OpCode::Return, ).encode(encoder)
            }
            Self::Trap { trap_code } => {
                (OpCode::Trap, trap_code).encode(encoder)
            }
            Self::ConsumeFuel { fuel } => {
                (OpCode::ConsumeFuel, fuel).encode(encoder)
            }
            Self::Branch { offset } => {
                (OpCode::Branch, offset).encode(encoder)
            }
            Self::BranchTable_R { len_targets, index } => {
                (OpCode::BranchTable_R, len_targets, index).encode(encoder)
            }
            Self::BranchTable_S { len_targets, index } => {
                (OpCode::BranchTable_S, len_targets, index).encode(encoder)
            }
            Self::BranchTableSpan_R { len_targets, index, values } => {
                (OpCode::BranchTableSpan_R, len_targets, index, values).encode(encoder)
            }
            Self::BranchTableSpan_S { len_targets, index, values } => {
                (OpCode::BranchTableSpan_S, len_targets, index, values).encode(encoder)
            }
            Self::U32Copy_Ri { result, value } => {
                (OpCode::U32Copy_Ri, result, value).encode(encoder)
            }
            Self::U32Copy_Si { result, value } => {
                (OpCode::U32Copy_Si, result, value).encode(encoder)
            }
            Self::U64Copy_Rs { result, value } => {
                (OpCode::U64Copy_Rs, result, value).encode(encoder)
            }
            Self::U64Copy_Ri { result, value } => {
                (OpCode::U64Copy_Ri, result, value).encode(encoder)
            }
            Self::U64Copy_Sr { result, value } => {
                (OpCode::U64Copy_Sr, result, value).encode(encoder)
            }
            Self::U64Copy_Ss { result, value } => {
                (OpCode::U64Copy_Ss, result, value).encode(encoder)
            }
            Self::U64Copy_Si { result, value } => {
                (OpCode::U64Copy_Si, result, value).encode(encoder)
            }
            Self::F32Copy_Ri { result, value } => {
                (OpCode::F32Copy_Ri, result, value).encode(encoder)
            }
            Self::F32Copy_Rs { result, value } => {
                (OpCode::F32Copy_Rs, result, value).encode(encoder)
            }
            Self::F32Copy_Sr { result, value } => {
                (OpCode::F32Copy_Sr, result, value).encode(encoder)
            }
            Self::F64Copy_Rs { result, value } => {
                (OpCode::F64Copy_Rs, result, value).encode(encoder)
            }
            Self::F64Copy_Ri { result, value } => {
                (OpCode::F64Copy_Ri, result, value).encode(encoder)
            }
            Self::F64Copy_Sr { result, value } => {
                (OpCode::F64Copy_Sr, result, value).encode(encoder)
            }
            Self::U64Copy_S0s1 { result, value } => {
                (OpCode::U64Copy_S0s1, result, value).encode(encoder)
            }
            Self::U64Copy_S0s2 { result, value } => {
                (OpCode::U64Copy_S0s2, result, value).encode(encoder)
            }
            Self::U64Copy_S0s3 { result, value } => {
                (OpCode::U64Copy_S0s3, result, value).encode(encoder)
            }
            Self::U64Copy_S0s4 { result, value } => {
                (OpCode::U64Copy_S0s4, result, value).encode(encoder)
            }
            Self::U64Copy_S0s5 { result, value } => {
                (OpCode::U64Copy_S0s5, result, value).encode(encoder)
            }
            Self::U64Copy_S1s0 { result, value } => {
                (OpCode::U64Copy_S1s0, result, value).encode(encoder)
            }
            Self::U64Copy_S1s2 { result, value } => {
                (OpCode::U64Copy_S1s2, result, value).encode(encoder)
            }
            Self::U64Copy_S1s3 { result, value } => {
                (OpCode::U64Copy_S1s3, result, value).encode(encoder)
            }
            Self::U64Copy_S1s4 { result, value } => {
                (OpCode::U64Copy_S1s4, result, value).encode(encoder)
            }
            Self::U64Copy_S1s5 { result, value } => {
                (OpCode::U64Copy_S1s5, result, value).encode(encoder)
            }
            Self::U64Copy_S2s0 { result, value } => {
                (OpCode::U64Copy_S2s0, result, value).encode(encoder)
            }
            Self::U64Copy_S2s1 { result, value } => {
                (OpCode::U64Copy_S2s1, result, value).encode(encoder)
            }
            Self::U64Copy_S2s3 { result, value } => {
                (OpCode::U64Copy_S2s3, result, value).encode(encoder)
            }
            Self::U64Copy_S2s4 { result, value } => {
                (OpCode::U64Copy_S2s4, result, value).encode(encoder)
            }
            Self::U64Copy_S2s5 { result, value } => {
                (OpCode::U64Copy_S2s5, result, value).encode(encoder)
            }
            Self::U64Copy_S3s0 { result, value } => {
                (OpCode::U64Copy_S3s0, result, value).encode(encoder)
            }
            Self::U64Copy_S3s1 { result, value } => {
                (OpCode::U64Copy_S3s1, result, value).encode(encoder)
            }
            Self::U64Copy_S3s2 { result, value } => {
                (OpCode::U64Copy_S3s2, result, value).encode(encoder)
            }
            Self::U64Copy_S3s4 { result, value } => {
                (OpCode::U64Copy_S3s4, result, value).encode(encoder)
            }
            Self::U64Copy_S3s5 { result, value } => {
                (OpCode::U64Copy_S3s5, result, value).encode(encoder)
            }
            Self::U64Copy_S4s0 { result, value } => {
                (OpCode::U64Copy_S4s0, result, value).encode(encoder)
            }
            Self::U64Copy_S4s1 { result, value } => {
                (OpCode::U64Copy_S4s1, result, value).encode(encoder)
            }
            Self::U64Copy_S4s2 { result, value } => {
                (OpCode::U64Copy_S4s2, result, value).encode(encoder)
            }
            Self::U64Copy_S4s3 { result, value } => {
                (OpCode::U64Copy_S4s3, result, value).encode(encoder)
            }
            Self::U64Copy_S4s5 { result, value } => {
                (OpCode::U64Copy_S4s5, result, value).encode(encoder)
            }
            Self::U64Copy_S5s0 { result, value } => {
                (OpCode::U64Copy_S5s0, result, value).encode(encoder)
            }
            Self::U64Copy_S5s1 { result, value } => {
                (OpCode::U64Copy_S5s1, result, value).encode(encoder)
            }
            Self::U64Copy_S5s2 { result, value } => {
                (OpCode::U64Copy_S5s2, result, value).encode(encoder)
            }
            Self::U64Copy_S5s3 { result, value } => {
                (OpCode::U64Copy_S5s3, result, value).encode(encoder)
            }
            Self::U64Copy_S5s4 { result, value } => {
                (OpCode::U64Copy_S5s4, result, value).encode(encoder)
            }
            Self::F32ReinterpretI32_Rr { result, value } => {
                (OpCode::F32ReinterpretI32_Rr, result, value).encode(encoder)
            }
            Self::I32ReinterpretF32_Rr { result, value } => {
                (OpCode::I32ReinterpretF32_Rr, result, value).encode(encoder)
            }
            Self::F64ReinterpretI64_Rr { result, value } => {
                (OpCode::F64ReinterpretI64_Rr, result, value).encode(encoder)
            }
            Self::I64ReinterpretF64_Rr { result, value } => {
                (OpCode::I64ReinterpretF64_Rr, result, value).encode(encoder)
            }
            Self::U64Copy_S0r { result, value } => {
                (OpCode::U64Copy_S0r, result, value).encode(encoder)
            }
            Self::U64Copy_S1r { result, value } => {
                (OpCode::U64Copy_S1r, result, value).encode(encoder)
            }
            Self::U64Copy_S2r { result, value } => {
                (OpCode::U64Copy_S2r, result, value).encode(encoder)
            }
            Self::U64Copy_S3r { result, value } => {
                (OpCode::U64Copy_S3r, result, value).encode(encoder)
            }
            Self::U64Copy_S4r { result, value } => {
                (OpCode::U64Copy_S4r, result, value).encode(encoder)
            }
            Self::U64Copy_S5r { result, value } => {
                (OpCode::U64Copy_S5r, result, value).encode(encoder)
            }
            Self::U64Copy_S6r { result, value } => {
                (OpCode::U64Copy_S6r, result, value).encode(encoder)
            }
            Self::U64Copy_S7r { result, value } => {
                (OpCode::U64Copy_S7r, result, value).encode(encoder)
            }
            Self::U64Copy_S8r { result, value } => {
                (OpCode::U64Copy_S8r, result, value).encode(encoder)
            }
            Self::U64Copy_S9r { result, value } => {
                (OpCode::U64Copy_S9r, result, value).encode(encoder)
            }
            Self::F32Copy_S0r { result, value } => {
                (OpCode::F32Copy_S0r, result, value).encode(encoder)
            }
            Self::F32Copy_S1r { result, value } => {
                (OpCode::F32Copy_S1r, result, value).encode(encoder)
            }
            Self::F32Copy_S2r { result, value } => {
                (OpCode::F32Copy_S2r, result, value).encode(encoder)
            }
            Self::F32Copy_S3r { result, value } => {
                (OpCode::F32Copy_S3r, result, value).encode(encoder)
            }
            Self::F32Copy_S4r { result, value } => {
                (OpCode::F32Copy_S4r, result, value).encode(encoder)
            }
            Self::F32Copy_S5r { result, value } => {
                (OpCode::F32Copy_S5r, result, value).encode(encoder)
            }
            Self::F32Copy_S6r { result, value } => {
                (OpCode::F32Copy_S6r, result, value).encode(encoder)
            }
            Self::F32Copy_S7r { result, value } => {
                (OpCode::F32Copy_S7r, result, value).encode(encoder)
            }
            Self::F32Copy_S8r { result, value } => {
                (OpCode::F32Copy_S8r, result, value).encode(encoder)
            }
            Self::F32Copy_S9r { result, value } => {
                (OpCode::F32Copy_S9r, result, value).encode(encoder)
            }
            Self::F64Copy_S0r { result, value } => {
                (OpCode::F64Copy_S0r, result, value).encode(encoder)
            }
            Self::F64Copy_S1r { result, value } => {
                (OpCode::F64Copy_S1r, result, value).encode(encoder)
            }
            Self::F64Copy_S2r { result, value } => {
                (OpCode::F64Copy_S2r, result, value).encode(encoder)
            }
            Self::F64Copy_S3r { result, value } => {
                (OpCode::F64Copy_S3r, result, value).encode(encoder)
            }
            Self::F64Copy_S4r { result, value } => {
                (OpCode::F64Copy_S4r, result, value).encode(encoder)
            }
            Self::F64Copy_S5r { result, value } => {
                (OpCode::F64Copy_S5r, result, value).encode(encoder)
            }
            Self::F64Copy_S6r { result, value } => {
                (OpCode::F64Copy_S6r, result, value).encode(encoder)
            }
            Self::F64Copy_S7r { result, value } => {
                (OpCode::F64Copy_S7r, result, value).encode(encoder)
            }
            Self::F64Copy_S8r { result, value } => {
                (OpCode::F64Copy_S8r, result, value).encode(encoder)
            }
            Self::F64Copy_S9r { result, value } => {
                (OpCode::F64Copy_S9r, result, value).encode(encoder)
            }
            Self::RefFunc { result, func } => {
                (OpCode::RefFunc, result, func).encode(encoder)
            }
            Self::CallInternal { params, func } => {
                (OpCode::CallInternal, params, func).encode(encoder)
            }
            Self::CallImported { params, func } => {
                (OpCode::CallImported, params, func).encode(encoder)
            }
            Self::CallIndirect_R { table, func_type, params, index } => {
                (OpCode::CallIndirect_R, table, func_type, params, index).encode(encoder)
            }
            Self::CallIndirect_S { table, func_type, params, index } => {
                (OpCode::CallIndirect_S, table, func_type, params, index).encode(encoder)
            }
            Self::ReturnCallIndirect_R { table, func_type, params, index } => {
                (OpCode::ReturnCallIndirect_R, table, func_type, params, index).encode(encoder)
            }
            Self::ReturnCallIndirect_S { table, func_type, params, index } => {
                (OpCode::ReturnCallIndirect_S, table, func_type, params, index).encode(encoder)
            }
            Self::ReturnCallInternal { params, func } => {
                (OpCode::ReturnCallInternal, params, func).encode(encoder)
            }
            Self::ReturnCallImported { params, func } => {
                (OpCode::ReturnCallImported, params, func).encode(encoder)
            }
            Self::GlobalGetU64_R { global, result } => {
                (OpCode::GlobalGetU64_R, global, result).encode(encoder)
            }
            Self::GlobalGetF32_R { global, result } => {
                (OpCode::GlobalGetF32_R, global, result).encode(encoder)
            }
            Self::GlobalGetF64_R { global, result } => {
                (OpCode::GlobalGetF64_R, global, result).encode(encoder)
            }
            Self::GlobalSetU32_I { global, value } => {
                (OpCode::GlobalSetU32_I, global, value).encode(encoder)
            }
            Self::GlobalSetU64_R { global, value } => {
                (OpCode::GlobalSetU64_R, global, value).encode(encoder)
            }
            Self::GlobalSetU64_S { global, value } => {
                (OpCode::GlobalSetU64_S, global, value).encode(encoder)
            }
            Self::GlobalSetU64_I { global, value } => {
                (OpCode::GlobalSetU64_I, global, value).encode(encoder)
            }
            Self::GlobalSetF32_R { global, value } => {
                (OpCode::GlobalSetF32_R, global, value).encode(encoder)
            }
            Self::GlobalSetF64_R { global, value } => {
                (OpCode::GlobalSetF64_R, global, value).encode(encoder)
            }
            Self::DataDrop { data } => {
                (OpCode::DataDrop, data).encode(encoder)
            }
            Self::MemorySize { result, memory } => {
                (OpCode::MemorySize, result, memory).encode(encoder)
            }
            Self::MemoryGrow { result, delta, memory } => {
                (OpCode::MemoryGrow, result, delta, memory).encode(encoder)
            }
            Self::MemoryCopy { dst_memory, src_memory, dst, src, len } => {
                (OpCode::MemoryCopy, dst_memory, src_memory, dst, src, len).encode(encoder)
            }
            Self::MemoryFill { memory, dst, len, value } => {
                (OpCode::MemoryFill, memory, dst, len, value).encode(encoder)
            }
            Self::MemoryInit { memory, data, dst, src, len } => {
                (OpCode::MemoryInit, memory, data, dst, src, len).encode(encoder)
            }
            Self::TableGet_Rr { result, index, table } => {
                (OpCode::TableGet_Rr, result, index, table).encode(encoder)
            }
            Self::TableSet_Rs { table, index, value } => {
                (OpCode::TableSet_Rs, table, index, value).encode(encoder)
            }
            Self::TableSet_Ri { table, index, value } => {
                (OpCode::TableSet_Ri, table, index, value).encode(encoder)
            }
            Self::TableGet_Rs { result, index, table } => {
                (OpCode::TableGet_Rs, result, index, table).encode(encoder)
            }
            Self::TableSet_Sr { table, index, value } => {
                (OpCode::TableSet_Sr, table, index, value).encode(encoder)
            }
            Self::TableSet_Ss { table, index, value } => {
                (OpCode::TableSet_Ss, table, index, value).encode(encoder)
            }
            Self::TableSet_Si { table, index, value } => {
                (OpCode::TableSet_Si, table, index, value).encode(encoder)
            }
            Self::TableGet_Ri { result, index, table } => {
                (OpCode::TableGet_Ri, result, index, table).encode(encoder)
            }
            Self::TableSet_Ir { table, index, value } => {
                (OpCode::TableSet_Ir, table, index, value).encode(encoder)
            }
            Self::TableSet_Is { table, index, value } => {
                (OpCode::TableSet_Is, table, index, value).encode(encoder)
            }
            Self::TableSet_Ii { table, index, value } => {
                (OpCode::TableSet_Ii, table, index, value).encode(encoder)
            }
            Self::TableSize { result, table } => {
                (OpCode::TableSize, result, table).encode(encoder)
            }
            Self::TableGrow { result, delta, value, table } => {
                (OpCode::TableGrow, result, delta, value, table).encode(encoder)
            }
            Self::TableCopy { dst_table, src_table, dst, src, len } => {
                (OpCode::TableCopy, dst_table, src_table, dst, src, len).encode(encoder)
            }
            Self::TableFill { table, dst, len, value } => {
                (OpCode::TableFill, table, dst, len, value).encode(encoder)
            }
            Self::TableInit { table, elem, dst, src, len } => {
                (OpCode::TableInit, table, elem, dst, src, len).encode(encoder)
            }
            Self::ElemDrop { elem } => {
                (OpCode::ElemDrop, elem).encode(encoder)
            }
            Self::I64Add128 { results, lhs_lo, lhs_hi, rhs_lo, rhs_hi } => {
                (OpCode::I64Add128, results, lhs_lo, lhs_hi, rhs_lo, rhs_hi).encode(encoder)
            }
            Self::I64Sub128 { results, lhs_lo, lhs_hi, rhs_lo, rhs_hi } => {
                (OpCode::I64Sub128, results, lhs_lo, lhs_hi, rhs_lo, rhs_hi).encode(encoder)
            }
            Self::I64MulWide { results, lhs, rhs } => {
                (OpCode::I64MulWide, results, lhs, rhs).encode(encoder)
            }
            Self::U64MulWide { results, lhs, rhs } => {
                (OpCode::U64MulWide, results, lhs, rhs).encode(encoder)
            }

        }
    }
}
