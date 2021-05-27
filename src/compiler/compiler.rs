use std::rc::Rc;

use num_traits::cast::ToPrimitive;

use crate::ast;
use crate::types::ObjectRef;
use crate::util::BinaryOperator;
use crate::vm::{
    format_instructions, Instruction, Instructions, ObjectStore, RuntimeContext, VM,
};

use super::result::{CompilationError, CompilationErrorKind, CompilationResult};
use std::borrow::BorrowMut;

// Compiler ------------------------------------------------------------

/// Compile AST to VM instructions.
pub fn compile(vm: &mut VM, program: ast::Program, debug: bool) -> CompilationResult {
    if debug {
        eprintln!("COMPILING:\n{:?}", program);
    }

    let mut visitor = Visitor::new(&mut vm.ctx);
    visitor.visit_program(program)?;

    if debug {
        eprintln!("INSTRUCTIONS:\n{}", format_instructions(&visitor.instructions));
    }

    Ok(visitor.instructions)
}

// Visitor -------------------------------------------------------------

type VisitResult = Result<(), CompilationError>;

struct Visitor<'a> {
    ctx: &'a mut RuntimeContext,
    instructions: Instructions,
}

impl<'a> Visitor<'a> {
    fn new(ctx: &'a mut RuntimeContext) -> Self {
        Self { ctx, instructions: Instructions::new() }
    }

    // Utilities -------------------------------------------------------

    fn err(&self, message: String) -> VisitResult {
        Err(CompilationError::new(CompilationErrorKind::VisitationError(message)))
    }

    fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    fn push_const(&mut self, index: usize) {
        self.push(Instruction::LoadConst(index));
    }

    fn add_const(&mut self, val: ObjectRef) {
        let index = self.ctx.arena.add(val);
        self.push_const(index);
    }

    // Visitors --------------------------------------------------------

    fn visit_program(&mut self, node: ast::Program) -> VisitResult {
        for statement in node.statements {
            self.visit_statement(statement)?;
        }
        self.push(Instruction::Halt(0));
        Ok(())
    }

    fn visit_statement(&mut self, node: ast::Statement) -> VisitResult {
        match node.kind {
            ast::StatementKind::Print(maybe_expr) => {
                match maybe_expr {
                    Some(expr) => self.visit_expr(*expr)?,
                    None => self.push_const(0),
                }
                self.push(Instruction::Print);
                // XXX: This is sort of like the return value of print
                self.push_const(0);
            }
            ast::StatementKind::Expr(expr) => self.visit_expr(*expr)?,
            _ => self.err(format!("Unhandled statement: {:?}", node))?,
        }
        Ok(())
    }

    fn visit_expr(&mut self, node: ast::Expr) -> VisitResult {
        match node.kind {
            ast::ExprKind::BinaryOperation(a, op, b) => {
                self.visit_binary_operation(*a, op, *b)?
            }
            ast::ExprKind::Identifier(ident) => self.visit_identifier(*ident)?,
            ast::ExprKind::Literal(literal) => self.visit_literal(*literal)?,
            _ => self.err(format!("Unhandled expression: {:?}", node))?,
        }
        Ok(())
    }

    fn visit_binary_operation(
        &mut self,
        expr_a: ast::Expr,
        op: BinaryOperator,
        expr_b: ast::Expr,
    ) -> VisitResult {
        if let BinaryOperator::Assign = op {
            self.visit_assignment(expr_a, expr_b)
        } else {
            self.visit_expr(expr_a)?;
            self.visit_expr(expr_b)?;
            self.push(Instruction::BinaryOp(op));
            Ok(())
        }
    }

    fn visit_assignment(
        &mut self,
        name_expr: ast::Expr,
        value_expr: ast::Expr,
    ) -> VisitResult {
        match name_expr.kind {
            ast::ExprKind::Identifier(ident) => match ident.kind {
                ast::IdentifierKind::Identifier(name) => {
                    // Point at the next expression
                    // FIXME: This isn't correct
                    let index = self.ctx.arena.size();
                    self.ctx.arena.add_name(name, index);
                }
                _ => return self.err("Expected identifier".to_owned()),
            },
            _ => return self.err("Expected identifier".to_owned()),
        }

        self.visit_expr(value_expr)?;
        self.push(Instruction::BinaryOp(BinaryOperator::Assign));
        Ok(())
    }

    // Visit identifier as expression (i.e., not as part of an
    // assignment).
    fn visit_identifier(&mut self, node: ast::Identifier) -> VisitResult {
        match node.kind {
            ast::IdentifierKind::Identifier(name) => {
                self.push(Instruction::LoadConstByName(name))
            }
            ast::IdentifierKind::TypeIdentifier(name) => {
                self.push(Instruction::LoadConstByName(name))
            }
        }
        Ok(())
    }

    fn visit_literal(&mut self, node: ast::Literal) -> VisitResult {
        match node.kind {
            ast::LiteralKind::Nil => self.push_const(0),
            ast::LiteralKind::Bool(true) => self.push_const(1),
            ast::LiteralKind::Bool(false) => self.push_const(2),
            ast::LiteralKind::Float(value) => {
                self.add_const(self.ctx.builtins.new_float(value))
            }
            ast::LiteralKind::Int(value) => {
                self.add_const(self.ctx.builtins.new_int(value))
            }
            _ => return self.err(format!("Unhandled literal: {:?}", node)),
        }
        Ok(())
    }
}
