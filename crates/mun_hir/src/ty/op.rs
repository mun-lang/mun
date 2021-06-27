use crate::{ty::infer::InferTy, ty::TyKind, ArithOp, BinaryOp, Ty};

/// Given a binary operation and the type on the left of that operation, returns the expected type
/// for the right hand side of the operation or `Ty::Unknown` if such an operation is invalid.
pub(super) fn binary_op_rhs_expectation(op: BinaryOp, lhs_ty: Ty) -> Ty {
    match op {
        BinaryOp::LogicOp(..) => TyKind::Bool.intern(),

        // Compare operations are allowed for all scalar types
        BinaryOp::CmpOp(..) => match lhs_ty.interned() {
            TyKind::Int(_)
            | TyKind::Float(_)
            | TyKind::Bool
            | TyKind::InferenceVar(InferTy::Float(_))
            | TyKind::InferenceVar(InferTy::Int(_)) => lhs_ty,
            _ => TyKind::Unknown.intern(),
        },

        BinaryOp::Assignment { op: None } => match lhs_ty.interned() {
            TyKind::Int(_)
            | TyKind::Float(_)
            | TyKind::Bool
            | TyKind::Struct(_)
            | TyKind::Array(_)
            | TyKind::InferenceVar(InferTy::Float(_))
            | TyKind::InferenceVar(InferTy::Int(_)) => lhs_ty,
            _ => TyKind::Unknown.intern(),
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
        | BinaryOp::ArithOp(ArithOp::BitXor) => match lhs_ty.interned() {
            TyKind::Int(_) | TyKind::Bool | TyKind::InferenceVar(InferTy::Int(_)) => lhs_ty,
            _ => TyKind::Unknown.intern(),
        },

        // Arithmetic operations are supported only on number types
        BinaryOp::Assignment { op: Some(_) } | BinaryOp::ArithOp(_) => match lhs_ty.interned() {
            TyKind::Int(_)
            | TyKind::Float(_)
            | TyKind::InferenceVar(InferTy::Float(_))
            | TyKind::InferenceVar(InferTy::Int(_)) => lhs_ty,
            _ => TyKind::Unknown.intern(),
        },
    }
}

/// For a binary operation with the specified type on the right hand side of the operation, return
/// the return type of that operation.
pub(super) fn binary_op_return_ty(op: BinaryOp, rhs_ty: Ty) -> Ty {
    match op {
        BinaryOp::ArithOp(_) => match rhs_ty.interned() {
            TyKind::Int(_)
            | TyKind::Float(_)
            | TyKind::InferenceVar(InferTy::Float(_))
            | TyKind::InferenceVar(InferTy::Int(_)) => rhs_ty,
            _ => TyKind::Unknown.intern(),
        },
        BinaryOp::CmpOp(_) | BinaryOp::LogicOp(_) => TyKind::Bool.intern(),
        BinaryOp::Assignment { .. } => Ty::unit(),
    }
}
