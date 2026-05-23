use crate::jobs::PrimitiveType;
use crate::tensor::DType;
use crate::verify::VerificationResult;

pub(crate) fn dtype_tag(dtype: DType) -> u8 {
    dtype.tag()
}

pub(crate) fn dtype_from_tag(tag: u8) -> Option<DType> {
    match tag {
        1 => Some(DType::Int32),
        2 => Some(DType::Int64),
        3 => Some(DType::Fixed32),
        4 => Some(DType::FieldElement),
        _ => None,
    }
}

pub(crate) fn primitive_type_tag(primitive_type: PrimitiveType) -> u8 {
    match primitive_type {
        PrimitiveType::TensorOp => 1,
        PrimitiveType::LinearTrainingStep => 2,
    }
}

pub(crate) fn primitive_type_from_tag(tag: u8) -> Option<PrimitiveType> {
    match tag {
        1 => Some(PrimitiveType::TensorOp),
        2 => Some(PrimitiveType::LinearTrainingStep),
        _ => None,
    }
}

pub(crate) fn verification_result_tag(result: VerificationResult) -> u8 {
    match result {
        VerificationResult::Valid => 1,
        VerificationResult::Invalid => 2,
        VerificationResult::Unavailable => 3,
    }
}

pub(crate) fn verification_result_from_tag(tag: u8) -> Option<VerificationResult> {
    match tag {
        1 => Some(VerificationResult::Valid),
        2 => Some(VerificationResult::Invalid),
        3 => Some(VerificationResult::Unavailable),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_enum_tags_roundtrip_and_reject_unknown_tags() {
        for dtype in [
            DType::Int32,
            DType::Int64,
            DType::Fixed32,
            DType::FieldElement,
        ] {
            assert_eq!(dtype_from_tag(dtype_tag(dtype)), Some(dtype));
        }

        for primitive_type in [PrimitiveType::TensorOp, PrimitiveType::LinearTrainingStep] {
            assert_eq!(
                primitive_type_from_tag(primitive_type_tag(primitive_type)),
                Some(primitive_type)
            );
        }

        for result in [
            VerificationResult::Valid,
            VerificationResult::Invalid,
            VerificationResult::Unavailable,
        ] {
            assert_eq!(
                verification_result_from_tag(verification_result_tag(result)),
                Some(result)
            );
        }

        assert_eq!(dtype_from_tag(0), None);
        assert_eq!(primitive_type_from_tag(0), None);
        assert_eq!(verification_result_from_tag(0), None);
    }
}
