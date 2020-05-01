use crate::{ApplicationTy, ArithOp, BinaryOp, CmpOp, Ty, TypeCtor};

/// Given a binary operation and the type on the left of that operation, returns the expected type
/// for the right hand side of the operation or `Ty::Unknown` if such an operation is invalid.
pub(super) fn binary_op_rhs_expectation(op: BinaryOp, lhs_ty: Ty) -> Ty {
    match op {
        BinaryOp::LogicOp(..) => Ty::simple(TypeCtor::Bool),

        BinaryOp::CmpOp(CmpOp::Eq { .. }) => match lhs_ty {
            Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
                TypeCtor::Int(_) | TypeCtor::Float(_) | TypeCtor::Bool => lhs_ty,
                _ => Ty::Unknown,
            },
            _ => Ty::Unknown,
        },

        BinaryOp::Assignment { op: None } => match lhs_ty {
            Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
                TypeCtor::Int(_) | TypeCtor::Float(_) | TypeCtor::Bool | TypeCtor::Struct(_) => {
                    lhs_ty
                }
                _ => Ty::Unknown,
            },
            _ => Ty::Unknown,
        },
        BinaryOp::Assignment {
            op: Some(ArithOp::LeftShift),
        }
        | BinaryOp::Assignment {
            op: Some(ArithOp::RightShift),
        }
        | BinaryOp::Assignment {
            op: Some(ArithOp::BitAnd),
        }
        | BinaryOp::Assignment {
            op: Some(ArithOp::BitOr),
        }
        | BinaryOp::Assignment {
            op: Some(ArithOp::BitXor),
        }
        | BinaryOp::ArithOp(ArithOp::LeftShift)
        | BinaryOp::ArithOp(ArithOp::RightShift)
        | BinaryOp::ArithOp(ArithOp::BitAnd)
        | BinaryOp::ArithOp(ArithOp::BitOr)
        | BinaryOp::ArithOp(ArithOp::BitXor) => match lhs_ty {
            Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
                TypeCtor::Bool | TypeCtor::Int(_) => lhs_ty,
                _ => Ty::Unknown,
            },
            _ => Ty::Unknown,
        },
        BinaryOp::CmpOp(CmpOp::Ord { .. })
        | BinaryOp::Assignment { op: Some(_) }
        | BinaryOp::ArithOp(_) => match lhs_ty {
            Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
                TypeCtor::Int(_) | TypeCtor::Float(_) => lhs_ty,
                _ => Ty::Unknown,
            },
            _ => Ty::Unknown,
        },
    }
}

/// For a binary operation with the specified type on the right hand side of the operation, return
/// the return type of that operation.
pub(super) fn binary_op_return_ty(op: BinaryOp, rhs_ty: Ty) -> Ty {
    match op {
        BinaryOp::ArithOp(_) => match rhs_ty {
            Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
                TypeCtor::Int(_) | TypeCtor::Float(_) => rhs_ty,
                _ => Ty::Unknown,
            },
            _ => Ty::Unknown,
        },
        BinaryOp::CmpOp(_) | BinaryOp::LogicOp(_) => Ty::simple(TypeCtor::Bool),
        BinaryOp::Assignment { .. } => Ty::Empty,
    }
}
