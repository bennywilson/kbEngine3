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

#[allow(unused_macros)]
#[macro_export]
macro_rules! game_random_f32 {
    ($min:literal, $max:literal) =>{
		{
			let mut buf: [u8; 4] = [0, 0, 0, 0];
			let _ = getrandom::getrandom(&mut buf);
			let mut t = buf[0] as u32;
			t = t + (buf[1] as u32) << 8;
			t = t + (buf[2] as u32) << 16;
			t = t + (buf[3] as u32) << 24;
			let t = (t as f32 / u32::MAX as f32);
			$min + ($max - $min) * t
		}	 
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! game_random_u32 {
    ($min:literal, $max:literal) =>{
		{
			let mut buf: [u8; 4] = [0, 0, 0, 0];
			let _ = getrandom::getrandom(&mut buf);
			let mut t = buf[0] as u32;
			t = t + ((buf[1] as u32) << 8);
			t = t + ((buf[2] as u32) << 16);
			t = t + ((buf[3] as u32) << 24);
			let dif = ($max - $min) + 1;
						//log!("{} {} {} {} {}", (t%dif),buf[0], buf[1], buf[2], buf[3]);
			$min + (t % dif)
		}	 
    }
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
