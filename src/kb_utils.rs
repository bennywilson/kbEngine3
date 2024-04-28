#[allow(unused_macros)]
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}
#[allow(unused_macros)]
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! log {
    ( $ ( $t:tt )* ) => {
        println!( $( $t )* );
    };
}

pub fn kb_random_f32(min_val: f32, max_val: f32) -> f32 {
	let mut buf: [u8; 4] = [0, 0, 0, 0];
	let _ = getrandom::getrandom(&mut buf);
	let mut t = buf[0] as u32;
	t = t + (buf[1] as u32) << 8;
	t = t + (buf[2] as u32) << 16;
	t = t + (buf[3] as u32) << 24;
	let t = t as f32 / u32::MAX as f32;
	min_val + (max_val - min_val) * t
}

pub fn kb_random_u32(min_val: u32, max_val: u32) -> u32 {
	let mut buf: [u8; 4] = [0, 0, 0, 0];
	let _ = getrandom::getrandom(&mut buf);
	let mut t = buf[0] as u32;
	t = t + ((buf[1] as u32) << 8);
	t = t + ((buf[2] as u32) << 16);
	t = t + ((buf[3] as u32) << 24);
	let dif = (max_val - min_val) + 1;
	min_val + (t % dif)
}

pub fn kb_random_vec3(min_vec: CgVec3, max_vec: CgVec3) -> CgVec3 {
	let x = kb_random_f32(min_vec.x, max_vec.x);
	let y = kb_random_f32(min_vec.y, max_vec.y);
	let z = kb_random_f32(min_vec.z, max_vec.z);
	CgVec3::new(x, y, z)
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! PERF_SCOPE {
	($label:literal) => { }
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! PERF_SCOPE {
	($label:literal) =>{
		tracy_full::zone!($label);
	}
}

pub type CgVec3 = cgmath::Vector3<f32>;
pub const CG_VEC3_ZERO: CgVec3 = CgVec3::new(0.0, 0.0, 0.0);
pub const CG_VEC3_ONE: CgVec3 = CgVec3::new(1.0, 1.0, 1.0);
pub const CG_VEC3_UP: CgVec3 = CgVec3::new(0.0, 1.0, 0.0);

pub type CgVec4 = cgmath::Vector4<f32>;
pub const CG_VEC4_ZERO: CgVec4 = CgVec4::new(0.0, 0.0, 0.0, 0.0);


pub type CgPoint = cgmath::Point3<f32>;
pub const CG_POINT_ZERO: CgPoint = CgPoint::new(0.0, 0.0, 0.0);

pub type CgQuat = cgmath::Quaternion<f32>;
pub const CG_QUAT_IDENT: CgQuat = CgQuat::new(0.0, 0.0, 0.0, 1.0);

pub type CgMat = cgmath::Matrix4<f32>;