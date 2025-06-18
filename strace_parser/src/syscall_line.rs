pub type ProcessId = i64;
pub type FileDescriptor = i64;

#[derive(PartialEq, Debug, Clone)]
pub struct UnfinishedSyscallDesp {
    pub pid: ProcessId,
    pub cmd: String,
    pub syscall: String,
    pub partial_args: String,
    pub line_no: u32,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ResumedSyscallDesp {
    pub pid: ProcessId,
    pub cmd: String,
    pub syscall: String,
    pub partial_args: String,
    pub ret: String,
    pub line_no: u32,
}

#[derive(PartialEq, Debug, Clone, Default)]
pub struct SyscallDesp {
    pub pid: ProcessId,
    pub cmd: String,
    pub resumed_cmd: Option<String>,
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
