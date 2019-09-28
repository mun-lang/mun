use crate::{BinaryOp, Ty, TypeCtor};

pub(super) fn binary_op_rhs_expectation(_op: BinaryOp, lhs_ty: Ty) -> Ty {
    lhs_ty
}

pub(super) fn binary_op_return_ty(op: BinaryOp, rhs_ty: Ty) -> Ty {
    match op {
        BinaryOp::ArithOp(_) | BinaryOp::CmpOp(_) => rhs_ty,
        BinaryOp::LogicOp(_) => Ty::simple(TypeCtor::Bool),
        BinaryOp::Assignment => Ty::Empty,
    }
}
