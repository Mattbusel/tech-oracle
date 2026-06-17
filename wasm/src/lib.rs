//! signal_wasm: the dust-and-ember engine that runs in the browser.
//!
//! Pure `no_std` Rust compiled to wasm32-unknown-unknown (no wasm-bindgen, no
//! toolchain beyond the target). It simulates particles drifting up through the
//! lamplight of the den; JS reads the buffer straight out of wasm memory and
//! draws them as additive glows over the living painting.
//!
//! Buffer layout per particle (4 f32): x, y, z(depth 0..1), phase(0..1).

#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

const N: usize = 1400;
static mut BUF: [f32; N * 4] = [0.0; N * 4];
static mut RNG: u32 = 0x9E3779B9;

fn rnd() -> f32 {
    unsafe {
        // xorshift32
        RNG ^= RNG << 13;
        RNG ^= RNG >> 17;
        RNG ^= RNG << 5;
        ((RNG & 0x00FF_FFFF) as f32) / 16_777_216.0
    }
}

#[no_mangle]
pub extern "C" fn count() -> usize {
    N
}

#[no_mangle]
pub extern "C" fn particles() -> *const f32 {
    unsafe { BUF.as_ptr() }
}

#[no_mangle]
pub extern "C" fn init(seed: u32) {
    unsafe {
        RNG = seed | 1;
        for i in 0..N {
            BUF[i * 4] = rnd(); // x
            BUF[i * 4 + 1] = rnd(); // y
            BUF[i * 4 + 2] = rnd(); // z (depth)
            BUF[i * 4 + 3] = rnd(); // phase
        }
    }
}

#[no_mangle]
pub extern "C" fn step(dt: f32) {
    unsafe {
        for i in 0..N {
            let z = BUF[i * 4 + 2];
            // drift upward, faster when "closer" (higher z)
            BUF[i * 4 + 1] -= (0.010 + z * 0.030) * dt;
            // advance phase, wrapped to [0,1)
            let mut ph = BUF[i * 4 + 3] + dt * (0.20 + z * 0.50);
            ph -= (ph as i32) as f32;
            BUF[i * 4 + 3] = ph;
            // triangle-wave horizontal sway (no trig in no_std)
            let mut tri = ph * 2.0 - 1.0;
            if tri < 0.0 {
                tri = -tri;
            }
            tri = tri * 2.0 - 1.0;
            let mut x = BUF[i * 4] + tri * 0.0006 * (0.5 + z);
            // recycle at the top
            if BUF[i * 4 + 1] < -0.02 {
                BUF[i * 4 + 1] = 1.03;
                x = rnd();
                BUF[i * 4 + 2] = rnd();
            }
            if x < 0.0 {
                x += 1.0;
            } else if x > 1.0 {
                x -= 1.0;
            }
            BUF[i * 4] = x;
        }
    }
}
