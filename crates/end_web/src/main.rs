use end_web::{Slice, end_web_bootstrap, end_web_free_slice, end_web_solve_from_aic_toml};

fn main() {
    let _ = (
        end_web_bootstrap as unsafe extern "C" fn(*const Slice) -> *mut Slice,
        end_web_solve_from_aic_toml
            as unsafe extern "C" fn(*const Slice, *const Slice) -> *mut Slice,
        end_web_free_slice as unsafe extern "C" fn(*mut Slice),
    );
}
