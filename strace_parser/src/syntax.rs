#[derive(PartialEq, Debug, Clone)]
pub enum FdVar {
    CWD,
    Fd(String),
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
    Newproc(String),
    BeginTask(String),
    Nop,
}
