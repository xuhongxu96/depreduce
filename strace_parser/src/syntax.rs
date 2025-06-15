#[derive(PartialEq, Debug, Clone)]
pub enum Eff {
    Cons,
    Expunge,
    Prod,
}

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
    Consume(Expr),
    Produce(Expr),
    Input(String, String),
    Output(String, String),
    DependsOn(String, String),
    Newproc(String),
    BeginTask(String),
    EndTask(String),
    Nop,
}
