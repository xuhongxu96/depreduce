use serde::{Deserialize, Serialize};

use crate::syscall_line::FileDescriptor;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum FdVar {
    CWD,
    Fd(FileDescriptor),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Path {
    Unknown(String),
    Path(String),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    P(Path),
    V(FdVar),
    At(FdVar, Path),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    Let(FdVar, Expr),
    Del(Expr),
    Link(Expr, Expr),
    Copy(Expr, Expr),
    Consume(Expr),
    Produce(Expr),
    Newproc(i64),
    BeginTask(String),
    Nop,
}

impl Statement {
    pub fn is_nop(&self) -> bool {
        matches!(self, Statement::Nop)
    }

    pub fn is_newproc(&self) -> bool {
        matches!(self, Statement::Newproc(_))
    }
}
