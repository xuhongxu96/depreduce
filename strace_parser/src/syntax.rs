#[derive(PartialEq, Debug, Clone)]
pub enum FdVar {
    CWD,
    Fd(i64),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Path {
    Unknown(String),
    Path(String),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Expr {
    P(Path),
    V(FdVar),
    At(FdVar, Path),
}

#[derive(PartialEq, Debug, Clone)]
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
