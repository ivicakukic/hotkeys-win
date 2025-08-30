#[derive(Clone, Debug)]
pub enum Message {
    HookEvt(ProcessInfo),
    WinCreated(isize),
    Quit,
}


#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub title: String,
    pub hwnd: isize,
}