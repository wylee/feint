use crate::types::ObjectExt;
use crate::util::BinaryOperator;
use crate::vm::*;

#[test]
fn execute_simple_program() {
    let mut vm = VM::default();
    let i = vm.ctx.add_const(vm.ctx.builtins.new_int(1));
    let j = vm.ctx.add_const(vm.ctx.builtins.new_int(2));
    let chunk: Chunk = vec![
        Inst::LoadConst(i),
        Inst::LoadConst(j),
        Inst::BinaryOp(BinaryOperator::Add),
        Inst::Halt(0),
    ];
    if let Ok(result) = vm.execute(&chunk, false) {
        assert_eq!(result, VMState::Halted(0));
    }
}

#[test]
fn test_add_retrieve() {
    let mut ctx = RuntimeContext::default();
    let int = ctx.builtins.new_int(0);
    let int_copy = int.clone();
    let index = ctx.add_const(int.clone());
    let retrieved = ctx.get_const(index).unwrap();
    assert!(retrieved.class().is(&int.class()));
    assert!(retrieved.is(&*int_copy));
    assert!(retrieved.is_equal(&*int_copy, &ctx));
    assert_eq!(retrieved.id(), int_copy.id());
}
