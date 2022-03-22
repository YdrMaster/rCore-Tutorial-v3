extern "C" {
    fn _num_app();
}

const PTR: *const usize = _num_app as _;

pub fn get_num_app() -> usize {
    unsafe { *PTR }
}

pub fn get_app_data(app_id: usize) -> &'static [u8] {
    unsafe {
        debug_assert!(app_id < *PTR);
        let ptr = PTR.add(1 + app_id);
        core::slice::from_raw_parts(*ptr as _, *ptr.add(1) - *ptr)
    }
}
