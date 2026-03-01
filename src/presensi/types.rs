pub struct Nim(pub String);
pub struct Password(pub String);
pub struct Account {
    pub nim: Nim,
    pub password: Password,
}

#[derive(Debug)]
pub enum Subject {
    SubjectId(String),
    OldAttendanceCode(String),
}
pub struct Week(pub String);

#[derive(Debug)]
pub struct Solution(pub u32);

pub struct AttendanceResult {
    pub week: Week,
    pub success: bool,
    pub desc: String,
}
