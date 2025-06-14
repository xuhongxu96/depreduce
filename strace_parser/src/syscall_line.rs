#[derive(PartialEq, Debug, Clone)]
pub struct UnfinishedSyscallDesp {
    pub pid: u64,
    pub syscall: String,
    pub partial_args: String,
    pub line_no: u32,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ResumedSyscallDesp {
    pub pid: u64,
    pub syscall: String,
    pub partial_args: String,
    pub ret: String,
    pub line_no: u32,
}

#[derive(PartialEq, Debug, Clone)]
pub struct SyscallDesp {
    pub pid: u64,
    pub syscall: String,
    pub args: String,
    pub ret: String,
    pub line_no: u32,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ErrorSyscallDesp {
    pub line_no: u32,
    pub line: String,
    pub msg: String,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SyscallLine {
    Full(SyscallDesp),
    Unfinished(UnfinishedSyscallDesp),
    Resumed(ResumedSyscallDesp),
    Error(ErrorSyscallDesp),
}
