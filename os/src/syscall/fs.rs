use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            // 获取用户空间中的数据
            let buf = unsafe { core::slice::from_raw_parts(buf, len) };
            for buf in translated_byte_buffer(current_user_token(), buf) {
                print!("{}", core::str::from_utf8(buf).unwrap());
            }
            len as isize
        }
        _ => -1,
    }
}
