use crate::sys::{
    OS_LOG_TYPE_DEBUG, OS_LOG_TYPE_DEFAULT, OS_LOG_TYPE_ERROR, OS_LOG_TYPE_FAULT, OS_LOG_TYPE_INFO,
};

#[repr(u8)]
pub enum Level {
    Debug = OS_LOG_TYPE_DEBUG,
    Info = OS_LOG_TYPE_INFO,
    Default = OS_LOG_TYPE_DEFAULT,
    Error = OS_LOG_TYPE_ERROR,
    Fault = OS_LOG_TYPE_FAULT,
}
